"""Compare the 22050 Hz build with the rate-44100 build, band by band.

usage: python tools/compare_rates.py [--out DIR] [--sounds NAME,...]

Builds tts_demo at both rates, renders a fixed set of sounds with each, and prints every sound's level in 1 kHz bands
from 0 to 11 kHz (dB against the 22050 Hz render's 0-1 kHz band), the difference, the 44100 Hz render's 11-16 kHz
level, and both peaks. The renders stay in DIR to listen to.
"""
import argparse
import os
import subprocess
import sys
import wave

import numpy as np

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
sys.path.insert(0, os.path.join(ROOT, 'tools'))
from build_samples import BINGO, score  # noqa: E402

RATES = (22050, 44100)
EDGES = list(range(0, 12000, 1000))		# 1 kHz bands up to 11 kHz
TOP = (11025, 16000)					# what the 44100 Hz build adds


def sung(voice, key, lyric, ms):
	return ['--voice', str(voice), '--sing', f'r:200 {key}:{ms}:{lyric} r:300']


with open(os.path.join(ROOT, 'tools', 'seed_sentences.txt'), encoding='utf-8') as f:
	SENTENCE = f.readline().strip()

# name: (demo arguments, window); a window is ('note', share kept from the middle), ('hiss', ms, above Hz) for the
# loudest stretch above a frequency, or ('all',)
SOUNDS = {}
for v, key in ((0, 48), (2, 60)):
	for lyric in ('ah', 'ee', 'oo'):
		SOUNDS[f'{lyric}{v}'] = (sung(v, key, lyric, 1500), ('note', 0.5))
for name, lyric in (('s', 'sass'), ('sh', 'shush'), ('f', 'fluff'), ('th', 'thin'), ('h', 'ha')):
	SOUNDS[name] = (sung(0, 48, lyric, 600), ('hiss', 60, 4000))
for name, lyric in (('k', 'kick'), ('t', 'tip')):
	SOUNDS[name] = (sung(0, 48, lyric, 600), ('hiss', 20, 2000))
for v in range(8):
	SOUNDS[f'speech{v}'] = (['--voice', str(v), SENTENCE], ('all',))
SOUNDS['bingo'] = (['--sing', score(BINGO, 0)], ('all',))


def build(rate):
	target = os.path.join(ROOT, 'target', f'rate-{rate}')
	features = ['--features', 'rate-44100'] if rate == 44100 else []
	env = dict(os.environ, CARGO_TARGET_DIR=target)
	subprocess.run(['cargo', 'build', '--release', '--example', 'tts_demo'] + features, cwd=ROOT, env=env, check=True)
	return os.path.join(target, 'release', 'examples', 'tts_demo' + ('.exe' if sys.platform == 'win32' else ''))


def read(path):
	with wave.open(path) as w:
		return np.frombuffer(w.readframes(w.getnframes()), np.int16).astype(float), w.getframerate()


def power(x, sr):
	# power spectrum and its frequencies, Hann-windowed, scaled so the bins sum to the mean square: windows of any
	# length and rate compare
	w = np.hanning(len(x))
	X = np.abs(np.fft.rfft(x * w)) ** 2 * 2 / (len(x) * np.sum(w * w))
	return X, np.fft.rfftfreq(len(x), 1 / sr)


def loudest(x, sr, ms, above):
	# start (s) of the stretch of ms with the most energy above a frequency, for a hiss or a burst
	n = int(sr * ms / 1000)
	hop = max(1, int(sr * 0.002))
	best, at = -1.0, 0
	for i in range(0, len(x) - n, hop):
		X, f = power(x[i:i + n], sr)
		e = X[f > above].sum()
		if e > best:
			best, at = e, i
	return at / sr


def window(x, sr, how, start):
	# start: where a hiss window begins, picked on the 22050 Hz render so both builds cut the same moment
	if how[0] == 'note':
		# the held note: drop the 200 ms rest before it and the 300 ms after, keep the middle share
		note = x[int(sr * 0.2):len(x) - int(sr * 0.3)]
		cut = int(len(note) * (1 - how[1]) / 2)
		return note[cut:len(note) - cut]
	if how[0] == 'hiss':
		at = int(start * sr)
		return x[at:at + int(sr * how[1] / 1000)]
	return x


def band_db(X, f, lo, hi):
	return 10 * np.log10(X[(f >= lo) & (f < hi)].sum() + 1e-12)


def main():
	ap = argparse.ArgumentParser()
	ap.add_argument('--out', default=os.path.join(ROOT, 'target', 'compare_rates'))
	ap.add_argument('--sounds', default='')
	args = ap.parse_args()
	names = args.sounds.split(',') if args.sounds else list(SOUNDS)
	os.makedirs(args.out, exist_ok=True)
	demos = {rate: build(rate) for rate in RATES}
	head = ' '.join(f'{lo // 1000:>2}-{lo // 1000 + 1:<2}' for lo in EDGES[:-1])
	print(f'{"":10} {"":6} {head}  11-16k   peak')
	for name in names:
		demo_args, how = SOUNDS[name]
		levels = {}
		start = None
		for rate in RATES:
			path = os.path.join(args.out, f'{name}_{rate}.wav')
			subprocess.run([demos[rate], '--16bit', '--humanize', '0', '--wav', path] + demo_args, check=True)
			x, sr = read(path)
			if how[0] == 'hiss' and start is None:
				start = loudest(x, sr, how[1], how[2])
			X, f = power(window(x, sr, how, start), sr)
			bands = [band_db(X, f, lo, hi) for lo, hi in zip(EDGES, EDGES[1:])]
			top = band_db(X, f, *TOP) if sr > 2 * TOP[0] else None
			levels[rate] = (bands, top, int(np.abs(x).max()))
		ref = levels[22050][0][0]
		for rate in RATES:
			bands, top, peak = levels[rate]
			row = ' '.join(f'{b - ref:5.1f}' for b in bands)
			tail = f'{top - ref:6.1f}' if top is not None else '      '
			print(f'{name if rate == 22050 else "":10} {rate:6} {row}  {tail}  {peak:5}')
		diff = ' '.join(f'{b - a:5.1f}' for a, b in zip(levels[22050][0], levels[44100][0]))
		print(f'{"":10} {"diff":>6} {diff}')


if __name__ == '__main__':
	main()
