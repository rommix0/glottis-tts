"""Build src/_dict.rs, the exceptions dictionary the synthesizer looks words up in before letter-to-sound.

usage: python tools/build_lexicon.py ALIGNED CMUDICT [--check-exceptions EXCEPTIONS FUNCTION_WORDS]

ALIGNED is CMUdict aligned letter by letter ("spaced letters<TAB>one phone class per letter", "_" for a silent
letter, "a|b" for a letter that makes two phones). CMUDICT is CMUdict in the synthesizer's phone notation, one
pronunciation per word.

The dictionary holds:
  - every CMUdict word the letter-to-sound tree gets wrong (the tree is trained here with the same recipe as the
    one compiled into src/_lts.rs, so it makes the same mistakes), and every CMUdict word missing from ALIGNED,
    which the tree was never checked on;
  - tools/lexicon_manual.txt, which overrides or adds entries;
  - the function words in tools/function_words.txt, flagged, with their CMUdict pronunciation. A one-syllable
    function word is given without stress and with its vowel reduced (ah -> ax, ih -> ix, er -> axr), as it is
    said unaccented in running speech; longer ones keep CMUdict's stress.

--check-exceptions writes the dictionary straight from an exceptions file and a function-word list instead, to
confirm this writer reproduces an existing src/_dict.rs byte for byte.
"""
import os
import sys

import numpy as np
from sklearn.model_selection import train_test_split
from sklearn.tree import DecisionTreeClassifier

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, '..', 'src', '_dict.rs')
WINDOW = 4

# phone id = position, as PHNIDToPhoneme in src/_utils.rs reads them
PHONES = ['"aa', '"ae', '"ah', '"ao', '"aw', '"ay', '"eh', '"er', '"ey', '"ih', '"iy', '"ow', '"oy', '"uh', '"uw',
          "'aa", "'ae", "'ah", "'ao", "'aw", "'ay", "'eh", "'eh|m", "'er", "'ey", "'ih", "'iy", "'ow", "'oy", "'uh",
          "'uw", '_', 'aa', 'ae', 'ao', 'aw', 'ax', 'axr', 'ay', 'b', 'b|y', 'ch', 'd', 'dh', 'eh', 'ey', 'f', 'f|y',
          'g', 'g|y', 'g|z', 'hh', 'hh|y', 'ix', 'iy', 'jh', 'k', 'k|s', 'k|y', 'l', 'lx', 'm', 'm|ax', 'm|y', 'n',
          'ng', 'ow', 'oy', 'p', 'p|y', 'r', 'rx', 's', 'sh', 's|"iy', "s|'iy", 't', 'th', 'uh', 'uw', 'v', 'w', 'y',
          'y|"uw', "y|'uw", 'y|ax', 'y|uw', 'z', 'zh', 'z|ax']
PHONE_ID = {p: i for i, p in enumerate(PHONES)}
VOWELS = {'aa', 'ae', 'ah', 'ao', 'aw', 'ay', 'eh', 'er', 'ey', 'ih', 'iy', 'ow', 'oy', 'uh', 'uw', 'ax', 'axr', 'ix'}
WEAK = {'ah': 'ax', 'ih': 'ix', 'er': 'axr'}


def read_pairs(path):
    out = []
    for line in open(path, encoding='utf-8'):
        line = line.rstrip('\n')
        if not line or line[0] == '#' or '\t' not in line:
            continue
        g, p = line.split('\t')
        out.append((tuple(g.split()), p.split()))
    return out


def plain(classes):
    # aligned classes to phones: drop silent letters, split two-phone letters
    return [q for c in classes for q in c.split('|') if q != '_']


def weak_form(pron):
    if sum(p.lstrip('\'"') in VOWELS for p in pron) != 1:
        return pron
    out = []
    for p in pron:
        bare = p.lstrip('\'"')
        out.append(WEAK.get(bare, bare) if p != bare else p)
    return out


def tree_misses(aligned):
    # the recipe of src/_lts.rs: 9-letter window, unique windows, 90 % of them, depth-25 tree
    vecs = {}
    for g, p in aligned:
        for x in range(len(g)):
            vecs[tuple(g[y] if 0 <= y < len(g) else '#' for y in range(x - WINDOW, x + WINDOW + 1))] = p[x]
    feat = {k: v for v, k in enumerate(sorted({t for k in vecs for t in k}))}
    targ = {k: v for v, k in enumerate(sorted(set(vecs.values())))}
    X = np.array([[feat[t] for t in k] for k in vecs])
    y = np.array([targ[t] for t in vecs.values()])
    X_train, _, y_train, _ = train_test_split(X, y, test_size=0.1, random_state=42)
    tree = DecisionTreeClassifier(max_depth=25, ccp_alpha=4e-6, random_state=42).fit(X_train, y_train)
    names = sorted(targ)
    misses = []
    for g, p in aligned:
        V = np.array([[feat.get(g[y] if 0 <= y < len(g) else '#', -1) for y in range(x - WINDOW, x + WINDOW + 1)]
                      for x in range(len(g))])
        if [names[i] for i in tree.predict(V)] != p:
            misses.append((g, plain(p)))
    return misses


def write_dict(entries, path):
    # entries: (letters, phones, function?) in lookup order; the first entry for a spelling wins
    parts, seen = {}, set()
    for g, p, fw in entries:
        if g in seen:
            continue
        seen.add(g)
        parts.setdefault(ord(g[0].lower()), []).append((g, p, fw))
    data, jump = bytearray(), {}
    for letter in sorted(parts):
        jump[letter] = len(data)
        for g, p, fw in parts[letter]:
            data.append(1 if fw else 0)
            data.extend(ord(c) for c in g)
            data.append(0xFE)
            data.extend(PHONE_ID[q] for q in p)
            data.append(0xFF)
        data.append(0xFD)
    lines = ['/*', '', 'EXCEPTIONS DICTIONARY and LETTERS JUMP TABLE', '', '*/',
             'pub fn getLetterOffset(x: u8) -> usize {', '\tmatch x {']
    lines += [f'\t\t{k:<3} => {v},' for k, v in jump.items() if 97 <= k <= 122]
    lines += ['\t\t_   => 0,', '\t}', '}', '', f'pub static dictdat: [u8; {len(data)}] = [']
    lines += [''.join(f'0x{b:02x},' for b in data[i:i + 16]) for i in range(0, len(data), 16)]
    lines += ['];', '']
    open(path, 'w', encoding='utf-8', newline='\n').write('\n'.join(lines))
    return len(data), len(seen)


def main():
    if len(sys.argv) >= 2 and sys.argv[1] == '--check-exceptions':
        fws = {l.strip() for l in open(sys.argv[3], encoding='utf-8') if l.strip()}
        entries = [(g, p, ''.join(g) in fws) for g, p in read_pairs(sys.argv[2])]
        print('%d bytes, %d entries' % write_dict(entries, OUT))
        return
    aligned = read_pairs(sys.argv[1])
    cmu = {''.join(g): p for g, p in read_pairs_plain(sys.argv[2])}
    manual = read_pairs(os.path.join(HERE, 'lexicon_manual.txt'))
    fwords = [l.strip() for l in open(os.path.join(HERE, 'function_words.txt'), encoding='utf-8')
              if l.strip() and not l.startswith('#')]
    over = {g: p for g, p in manual}
    fw_pron = {}
    for w in fwords:
        if w in cmu:
            fw_pron[tuple(w)] = weak_form(cmu[w])
        else:
            print(f'not in CMUdict, left unflagged: {w}', file=sys.stderr)
    # capitalized and other non-lowercase spellings first, so an exact-case match beats the lowercase entry
    entries = [(g, p, False) for g, p in manual if ''.join(g) != ''.join(g).lower()]
    listed = set()
    # CMUdict words that could not be aligned never reached the tree, so nothing vouches for its guess
    seen = {g for g, _ in aligned}
    unaligned = [(tuple(w), p) for w, p in cmu.items() if tuple(w) not in seen]
    for g, p in tree_misses(aligned) + unaligned:
        listed.add(g)
        if g in fw_pron:
            entries.append((g, fw_pron[g], True))
        else:
            entries.append((g, over.get(g, p), False))
    entries += [(g, p, False) for g, p in manual if g not in listed and ''.join(g) == ''.join(g).lower()]
    entries += [(g, p, True) for g, p in fw_pron.items() if g not in listed]
    print('%d bytes, %d entries' % write_dict(entries, OUT))


def read_pairs_plain(path):
    # CMUdict in this notation: the word unspaced, a tab, phones
    out = []
    for line in open(path, encoding='utf-8'):
        if '\t' in line:
            w, p = line.rstrip('\n').split('\t', 1)
            out.append((tuple(w), p.split()))
    return out


if __name__ == '__main__':
    main()
