"""Render the README's audio samples into samples/ as MP3 (needs ffmpeg on the PATH).

usage: python tools/build_samples.py

Each sample is a few lines said or sung one after another: trailing silence trimmed, 550 ms between lines of the
same voice, 950 ms when the voice changes.
"""
import array
import os
import subprocess
import sys
import tempfile
import wave

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
OUT = os.path.join(ROOT, 'samples')
RATE = 22050
LEAD_MS, SAME_MS, CHANGE_MS, TAIL_MS = 250, 550, 950, 500
QUIET = 30		# a trailing sample at or under this is silence

# sample name: (voice, rate, text) lines
SAMPLES = {
	'tour': [
		(0, 1.0, 'Hello! This is the standard voice of the speech synthesizer, and this is a short tour of what it '
			'can do.'),
		(0, 1.0, 'It reads numbers, dates and money. On March 3rd, 2026, we shipped 1,250 units for $12.50 each.'),
		(0, 1.0, 'It asks questions. Did you get my message? Where should we meet for lunch?'),
		(0, 1.0, 'And it handles long sentences, so the children painted pictures of their favorite animals and hung '
			'them along the wall because everyone wanted to see them.'),
	],
	'voices': [
		(1, 1.0, 'This is voice one, a little lower and fuller. Would you like a simple bottle of apple juice, or a '
			'full cup of tea?'),
		(2, 1.0, 'This is voice two. She sells seashells by the seashore, and the shells she sells are surely '
			'seashells.'),
		(3, 1.0, 'This is voice three. Can we go to the zoo on Friday? I really want to see the penguins!'),
		(4, 1.0, 'This is voice four. The thieves fished in the deep, dark river until midnight.'),
		(5, 1.0, 'This is voice five. Twelve thousand people gathered in the hall to hear the final announcement.'),
		(6, 1.0, 'This is voice six, stretched and pulled apart, for a sound like nobody else.'),
		(7, 1.0, 'This is voice seven. I am a little nervous, but I will keep on talking.'),
	],
	'rate': [
		(0, 1.6, 'The standard voice can also speak quickly, when there is not much time.'),
		(0, 0.75, 'Or slowly, and carefully.'),
		(0, 1.0, 'Thank you for listening.'),
	],
}

# songs as (MIDI key, ms, lyric), Happy Birthday in G and Bingo in C; a key of None is a rest, a lyric of "-" carries on
# the word
BIRTHDAY = [
	(None, 400, ''), (55, 340, 'happy'), (55, 110, '-'), (57, 450, 'birthday'), (55, 450, '-'), (60, 450, 'to'),
	(59, 900, 'you'), (None, 450, ''),
	(55, 340, 'happy'), (55, 110, '-'), (57, 450, 'birthday'), (55, 450, '-'), (62, 450, 'to'), (60, 900, 'you'),
	(None, 450, ''),
	(55, 340, 'happy'), (55, 110, '-'), (67, 450, 'birthday'), (64, 450, '-'), (60, 450, 'dear'), (59, 450, 'friend'),
	(57, 900, '-'), (None, 450, ''),
	(65, 340, 'happy'), (65, 110, '-'), (64, 450, 'birthday'), (60, 450, '-'), (62, 450, 'to'), (60, 1350, 'you'),
	(None, 300, ''),
]
BINGO = [
	(None, 500, ''), (55, 500, 'there'),
	(60, 500, 'was'), (60, 500, 'a'), (55, 500, 'farmer'), (55, 500, '-'),
	(57, 500, 'had'), (57, 500, 'a'), (55, 750, 'dog'), (None, 250, ''),
	(55, 500, 'and'), (60, 500, 'bingo'), (60, 500, '-'), (62, 500, 'was'), (62, 500, 'his'),
	(64, 1000, 'name'), (60, 750, 'oh'), (None, 250, ''),
	(64, 500, 'b'), (64, 500, 'i'), (65, 250, 'n'), (65, 250, 'g'), (65, 250, 'o'), (None, 250, ''),
	(62, 500, 'b'), (62, 500, 'i'), (64, 250, 'n'), (64, 250, 'g'), (64, 250, 'o'), (None, 250, ''),
	(60, 500, 'b'), (60, 500, 'i'), (62, 250, 'n'), (62, 250, 'g'), (62, 250, 'o'), (None, 250, ''),
	(59, 500, 'and'), (55, 500, 'bingo'), (57, 500, '-'), (59, 250, 'was'), (59, 250, 'his'),
	(60, 1000, 'name'), (60, 1000, 'oh'), (None, 500, ''),
]


def score(song, shift):
	# the demo's --sing score, moved by shift semitones
	return ' '.join(f'r:{ms}' if key is None else f'{key + shift}:{ms}:{lyric}' for key, ms, lyric in song)


# song name: (voice, demo options) lines
SONGS = {
	'birthday': [
		(0, ['--sing', score(BIRTHDAY, 0)]),
	],
	'bingo': [
		(0, ['--sing', score(BINGO, 0)]),
		(0, ['--rap', '--sing', score(BINGO, 0)]),
	],
}


def render(demo, tmp, voice, options):
	path = os.path.join(tmp, 'line.wav')
	subprocess.run([demo, '--16bit', '--voice', str(voice), '--wav', path] + options, check=True)
	with wave.open(path) as w:
		x = array.array('h', w.readframes(w.getnframes()))
	end = len(x)
	while end > 0 and abs(x[end - 1]) <= QUIET:
		end -= 1
	return x[:end]


def silence(ms):
	return array.array('h', bytes(2 * (RATE * ms // 1000)))


def main():
	subprocess.run(['cargo', 'build', '--release', '--example', 'tts_demo'], cwd=ROOT, check=True)
	demo = os.path.join(ROOT, 'target', 'release', 'examples', 'tts_demo' + ('.exe' if sys.platform == 'win32' else ''))
	os.makedirs(OUT, exist_ok=True)
	takes = {name: [(voice, ['--rate', str(rate), text]) for voice, rate, text in lines]
		for name, lines in SAMPLES.items()}
	takes.update(SONGS)
	with tempfile.TemporaryDirectory() as tmp:
		for name, lines in takes.items():
			x = silence(LEAD_MS)
			for k, (voice, options) in enumerate(lines):
				if k > 0:
					x += silence(SAME_MS if voice == lines[k - 1][0] else CHANGE_MS)
				x += render(demo, tmp, voice, options)
			x += silence(TAIL_MS)
			wav = os.path.join(tmp, name + '.wav')
			with wave.open(wav, 'wb') as w:
				w.setnchannels(1)
				w.setsampwidth(2)
				w.setframerate(RATE)
				w.writeframes(x.tobytes())
			mp3 = os.path.join(OUT, name + '.mp3')
			subprocess.run(['ffmpeg', '-y', '-loglevel', 'error', '-i', wav, '-codec:a', 'libmp3lame', '-b:a', '96k',
				'-ac', '1', mp3], check=True)
			print(f'{mp3}: {len(x) / RATE:.1f} s')


if __name__ == '__main__':
	main()
