"""Turn a --trace dump into the per-phone duration table for src/_dur.rs.

usage: python tools/dur_means.py trace.txt > table.rs

Each TRACE line is "TRACE <phone> <stress> dur=<ms> f0=<hz>". The mean duration of a phone becomes its inherent
duration and 45 % of it its minimum. Phones never seen get the mean of their class. Stop minimums are raised to the
length the synthesizer plays a stop at least (burst onset + minimum closure).
"""
import re
import sys
from collections import defaultdict

NAMES = {
    1: 'aa', 2: 'ae', 3: 'ah', 4: 'ao', 5: 'aw', 6: 'ax', 7: 'axr', 8: 'ay', 9: 'b', 10: 'ch', 11: 'd', 12: 'dh',
    13: 'eh', 14: 'er', 15: 'ey', 16: 'f', 17: 'g', 18: 'hh', 19: 'ih', 20: 'ix', 21: 'iy', 22: 'jh', 23: 'k',
    24: 'l', 25: 'lx', 26: 'm', 27: 'n', 28: 'ng', 29: 'ow', 30: 'oy', 31: 'p', 32: 'r', 33: 'rx', 34: 's', 35: 'sh',
    36: 't', 37: 'th', 38: 'uh', 39: 'uw', 40: 'v', 41: 'w', 42: 'y', 43: 'z', 44: 'zh', 47: 'dx',
}
CLASSES = {
    'vowel': [1, 2, 3, 4, 5, 6, 7, 8, 13, 14, 15, 19, 20, 21, 29, 30, 38, 39],
    'stop': [9, 11, 17, 23, 31, 36, 47],
    'fricative': [10, 12, 16, 18, 22, 34, 35, 37, 40, 43, 44],
    'sonorant': [24, 25, 26, 27, 28, 32, 33, 41, 42],
}
STOP_FLOOR = {31: 88, 36: 101, 23: 103, 9: 45, 11: 53, 17: 70}
MIN_SHARE = 0.45


def main(path):
    durs = defaultdict(list)
    for line in open(path, encoding='utf-8'):
        m = re.match(r'TRACE (\d+) \d+ dur=(-?\d+)', line)
        if m and int(m.group(1)) in NAMES:
            durs[int(m.group(1))].append(int(m.group(2)))
    mean = {p: sum(v) / len(v) for p, v in durs.items()}
    class_mean = {}
    for name, members in CLASSES.items():
        seen = [mean[p] for p in members if p in mean]
        class_mean[name] = sum(seen) / len(seen)
    print('\t\t// phone => inherent, min (ms); seeded by tools/dur_means.py, then tuned by ear')
    for p in sorted(NAMES):
        cls = next(c for c, members in CLASSES.items() if p in members)
        inh = round(mean.get(p, class_mean[cls]))
        mn = max(round(MIN_SHARE * inh), STOP_FLOOR.get(p, 0))
        inh = max(inh, mn)
        n = len(durs.get(p, []))
        print(f'\t\t{p:<2} => PhoneDur {{ inherent: {inh:>3}.0, min: {mn:>3}.0 }},\t// {NAMES[p]} ({n} seen)')


if __name__ == '__main__':
    main(sys.argv[1])
