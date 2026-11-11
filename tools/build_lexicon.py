"""Build the letter-to-sound tree (src/_lts_tree.rs) and the exceptions dictionary (src/_dict.rs).

usage: python tools/build_lexicon.py ALIGNED CMUDICT [--heldout]

ALIGNED is CMUdict aligned letter by letter ("spaced letters<TAB>one phone class per letter", "_" for a silent
letter, "a|b" for a letter that makes two phones). CMUDICT is CMUdict in the synthesizer's phone notation, one
pronunciation per word. Both are in tools/, so the rebuild is

    python tools/build_lexicon.py tools/eng.fixed.aligned tools/eng.dict

Two trees pick each letter's phone class, left to right. Both see the 4 letters either side (coded with the
letters grouped by kind, so one threshold splits off the vowels or the stops), whether each of those is a vowel,
and the classes given to the 2 letters before; the second also sees the first tree's classes for the 2 letters
after. A word then gets exactly one primary stress: extra primaries become secondary, and a word with none takes
it on the letter likeliest to carry one. decode() below is what src/_lts.rs does at run time.

The dictionary holds:
  - every CMUdict word decode() gets wrong, and every CMUdict word missing from ALIGNED, which the tree was never
    checked on;
  - tools/lexicon_manual.txt, which overrides or adds entries;
  - the function words in tools/function_words.txt, flagged, with their CMUdict pronunciation. A one-syllable
    function word is given without stress and with its vowel reduced (ah -> ax, ih -> ix, er -> axr), as it is
    said unaccented in running speech; longer ones keep CMUdict's stress. A function word in
    tools/lexicon_manual.txt takes that pronunciation instead and stays flagged ("but" keeps its vowel).

--heldout trains on 90 % of ALIGNED and reports how many of the other words decode() gets exactly right; nothing
is written.
"""
import json
import os
import random
import sys

import numpy as np
from sklearn.tree import DecisionTreeClassifier

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(HERE, '..', 'src')
WINDOW = 4
HISTORY = 2
DEPTH = 25
ALPHA = 2e-6
# letter codes in this order from 1 (0 is outside the word): vowels, y, sonorants, stops, fricatives
ORDER = "'aeiouywhlrmnbpdtgkcqfvszxj"

# phone class id = position, as PHNIDToPhoneme in src/_utils.rs reads them
PHONES = ['"aa', '"ae', '"ah', '"ao', '"aw', '"ay', '"eh', '"er', '"ey', '"ih', '"iy', '"ow', '"oy', '"uh', '"uw',
          "'aa", "'ae", "'ah", "'ao", "'aw", "'ay", "'eh", "'eh|m", "'er", "'ey", "'ih", "'iy", "'ow", "'oy", "'uh",
          "'uw", '_', 'aa', 'ae', 'ao', 'aw', 'ax', 'axr', 'ay', 'b', 'b|y', 'ch', 'd', 'dh', 'eh', 'ey', 'f', 'f|y',
          'g', 'g|y', 'g|z', 'hh', 'hh|y', 'ix', 'iy', 'jh', 'k', 'k|s', 'k|y', 'l', 'lx', 'm', 'm|ax', 'm|y', 'n',
          'ng', 'ow', 'oy', 'p', 'p|y', 'r', 'rx', 's', 'sh', 's|"iy', "s|'iy", 't', 'th', 'uh', 'uw', 'v', 'w', 'y',
          'y|"uw', "y|'uw", 'y|ax', 'y|uw', 'z', 'zh', 'z|ax']
PHONE_ID = {p: i for i, p in enumerate(PHONES)}
SILENT = PHONE_ID['_']
START = len(PHONES)  # history before the first letter
PRIMARY = [("'" in p) for p in PHONES]
# an extra primary becomes the same class with secondary stress (kept when the inventory has none)
DEMOTE = [PHONE_ID.get(p.replace("'", '"'), i) for i, p in enumerate(PHONES)]
VOWELS = {'aa', 'ae', 'ah', 'ao', 'aw', 'ay', 'eh', 'er', 'ey', 'ih', 'iy', 'ow', 'oy', 'uh', 'uw', 'ax', 'axr', 'ix'}
WEAK = {'ah': 'ax', 'ih': 'ix', 'er': 'axr'}


def letter_code(ch):
    # as letter_code in src/_lts.rs: 0 outside the word, else 1 + the letter's place in ORDER
    ch = ch.lower()
    return ORDER.index(ch) + 1 if len(ch) == 1 and ch in ORDER else 0


def is_vowel(code):
    # a e i o u
    return 2 <= code <= 6


def read_pairs(path):
    out = []
    for line in open(path, encoding='utf-8'):
        line = line.rstrip('\n')
        if not line or line[0] == '#' or '\t' not in line:
            continue
        g, p = line.split('\t')
        out.append((tuple(g.split()), p.split()))
    return out


def read_cmudict(path):
    # the word unspaced, a tab, phones
    out = {}
    for line in open(path, encoding='utf-8'):
        if '\t' in line:
            w, p = line.rstrip('\n').split('\t', 1)
            out[w] = p.split()
    return out


def plain(classes):
    # classes to phones: drop silent letters, split two-phone letters
    return [q for c in classes for q in c.split('|') if q != '_']


def weak_form(pron):
    if sum(p.lstrip('\'"') in VOWELS for p in pron) != 1:
        return pron
    out = []
    for p in pron:
        bare = p.lstrip('\'"')
        out.append(WEAK.get(bare, bare) if p != bare else p)
    return out


def features(codes, x, hist, ahead=None):
    # the letters around x, the classes given before it, which of those letters are vowels, and (second tree) the
    # first tree's classes after it
    win = [codes[y] if 0 <= y < len(codes) else 0 for y in range(x - WINDOW, x + WINDOW + 1)]
    f = win + hist + [int(is_vowel(c)) for c in win]
    if ahead is not None:
        f += [ahead[x + k] if x + k < len(ahead) else START for k in range(1, HISTORY + 1)]
    return f


def first_pass(tree, words):
    """the first tree's class for every letter of every word (lists of letter codes), all words at once"""
    hists = [[START] * HISTORY for _ in words]
    out = [[] for _ in words]
    for x in range(max(len(w) for w in words)):
        idx = [i for i, w in enumerate(words) if len(w) > x]
        got = tree.predict(np.array([features(words[i], x, hists[i]) for i in idx], dtype=np.int16))
        for i, k in zip(idx, got):
            out[i].append(int(k))
            hists[i] = hists[i][1:] + [int(k)]
    return out


def train(aligned):
    words = [[letter_code(c) for c in g] for g, _ in aligned]
    targets = [[PHONE_ID[c] for c in p] for _, p in aligned]
    fit = lambda X, y: DecisionTreeClassifier(max_depth=DEPTH, ccp_alpha=ALPHA, random_state=42).fit(
        np.array(X, dtype=np.int16), np.array(y))
    X, y = [], []
    for codes, ids in zip(words, targets):
        hist = [START] * HISTORY
        for x in range(len(codes)):
            X.append(features(codes, x, hist))
            y.append(ids[x])
            hist = hist[1:] + [ids[x]]
    first = fit(X, y)
    # the second tree learns from the first one's own guesses on the right, as it gets them at run time
    ahead = first_pass(first, words)
    X = []
    for codes, ids, a in zip(words, targets, ahead):
        hist = [START] * HISTORY
        for x in range(len(codes)):
            X.append(features(codes, x, hist, a))
            hist = hist[1:] + [ids[x]]
    return first, fit(X, y)


class Tables:
    """the fitted trees as the flat tables src/_lts_tree.rs carries, one after the other; a node's left child is
    the node after it"""
    def __init__(self, trees):
        self.feat, self.thr, self.right, self.cls, self.prim, self.best, self.roots = [], [], [], [], [], [], []
        for tree in trees:
            t = tree.tree_
            base = len(self.feat)
            self.roots.append(base)
            classes = list(tree.classes_)
            prim = np.array([PRIMARY[c] for c in classes])
            v = t.value[:, 0, :]
            v = v / v.sum(1, keepdims=True)
            for i in range(t.node_count):
                leaf = t.children_left[i] < 0
                assert leaf or t.children_left[i] == i + 1
                self.feat.append(255 if leaf else int(t.feature[i]))
                self.thr.append(0 if leaf else int(np.floor(t.threshold[i])))
                self.right.append(0 if leaf else base + int(t.children_right[i]))
                self.cls.append(classes[int(np.argmax(v[i]))])
                self.prim.append(int(round(255 * v[i][prim].sum())))
                b = int(np.argmax(np.where(prim, v[i], -1.0)))
                self.best.append(classes[b] if prim.any() else 0)
        self.n = len(self.feat)

    def leaf(self, root, f):
        i = root
        while self.feat[i] != 255:
            i = i + 1 if f[self.feat[i]] <= self.thr[i] else self.right[i]
        return i


def decode(tab, letters):
    """phone class ids for a word, as the synthesizer plays them"""
    codes = [letter_code(c) for c in letters]
    hist = [START] * HISTORY
    ahead = []
    for x in range(len(codes)):
        ahead.append(tab.cls[tab.leaf(tab.roots[0], features(codes, x, hist))])
        hist = hist[1:] + [ahead[-1]]
    hist = [START] * HISTORY
    cls, prim, best = [], [], []
    for x in range(len(codes)):
        i = tab.leaf(tab.roots[1], features(codes, x, hist, ahead))
        cls.append(tab.cls[i]); prim.append(tab.prim[i]); best.append(tab.best[i])
        hist = hist[1:] + [tab.cls[i]]
    if cls and sum(PRIMARY[c] for c in cls) != 1:
        k = max(range(len(cls)), key=lambda j: (prim[j], -j))
        cls = [DEMOTE[c] if PRIMARY[c] and j != k else c for j, c in enumerate(cls)]
        if not PRIMARY[cls[k]]:
            cls[k] = best[k]
    out = []
    for c in cls:
        if c != SILENT and (not out or out[-1] != c):
            out.append(c)
    return out


def phones(ids):
    return plain([PHONES[i] for i in ids])


# what a stem's last phone makes of -s and -ed: 0 voiced, 1 t, 2 d, 3 voiceless, 4 voiceless sibilant, 5 voiced
# sibilant
def last_kind(i):
    q = PHONES[i].split('|')[-1].lstrip('\'"')
    return {'t': 1, 'd': 2, 'p': 3, 'k': 3, 'f': 3, 'th': 3, 's': 4, 'sh': 4, 'ch': 4, 'z': 5, 'zh': 5, 'jh': 5}.get(q, 0)


# suffix, how the stem was spelled before it, phones (or the -s / -ed rule); the first that fits on a known stem wins
SUFFIXES = [('ness', ('', 'i'), ['n', 'ax', 's']), ('less', ('', 'i'), ['l', 'ax', 's']), ('ful', ('', 'i'), ['f', 'ax', 'l']),
            ('ing', ('', 'e', 'double'), ['ix', 'ng']), ('ies', ('y',), ['z']), ('ied', ('y',), ['d']),
            ('ed', ('', 'e', 'double'), 'ed'), ("'s", ('',), 's'), ('ly', ('',), ['l', 'iy']),
            ('es', ('sibilant',), 's'), ('s', ('',), 's')]
MIN_STEM = 3


def stems(s, ways):
    for way in ways:
        if way == '':
            yield s
        elif way == 'e':
            yield s + 'e'
        elif way == 'y':
            yield s + 'y'
        elif way == 'i' and s.endswith('i'):
            yield s[:-1] + 'y'
        elif way == 'double' and len(s) > 1 and s[-1] == s[-2] and s[-1] not in 'aeiouy':
            yield s[:-1]
        elif way == 'sibilant' and s.endswith(('s', 'x', 'z', 'ch', 'sh')):
            yield s


def suffix_ids(rule, stem):
    k = last_kind(stem[-1])
    if rule == 's':
        rule = ['ix', 'z'] if k in (4, 5) else ['s'] if k in (1, 3) else ['z']
    elif rule == 'ed':
        rule = ['ix', 'd'] if k in (1, 2) else ['t'] if k in (3, 4) else ['d']
    return [PHONE_ID[q] for q in rule]


def compose(w, lookup, depth=2):
    """as compose in src/_morph.rs: the word itself, or a known stem and suffixes"""
    p = lookup(w)
    if p or depth == 0:
        return p
    for suf, ways, rule in SUFFIXES:
        if w.endswith(suf) and len(w) - len(suf) >= MIN_STEM:
            for s in stems(w[:-len(suf)], ways):
                q = compose(s, lookup, depth - 1)
                if q:
                    return q + suffix_ids(rule, q)
    return None


def fnv(w):
    h = 0x811C9DC5
    for b in w.encode():
        h = ((h ^ b) * 0x01000193) & 0xFFFFFFFF
    return h


def stem_lookup(tab, dictionary, known):
    # a stem is the dictionary's, or a known word the tree says right, or unknown
    return lambda w: dictionary.get(w) or (decode(tab, w) if fnv(w) in known else None)


def heldout(aligned):
    words = list(aligned)
    random.Random(1).shuffle(words)
    cut = len(words) // 10
    tab = Tables(train(words[cut:]))
    dictionary, known = {}, set()
    for g, p in words[cut:]:
        if phones(decode(tab, g)) == plain(p):
            known.add(fnv(''.join(g)))
        else:
            dictionary[''.join(g)] = [PHONE_ID[q] for q in plain(p)]
    lookup = stem_lookup(tab, dictionary, known)
    lts = morph = 0
    for g, p in words[:cut]:
        say = decode(tab, g)
        lts += phones(say) == plain(p)
        morph += phones(compose(''.join(g), lookup) or say) == plain(p)
    print(f'held-out words {cut}: LTS {100 * lts / cut:.1f} %, with suffix matching {100 * morph / cut:.1f} % '
          f'exactly right; tree {tab.n} nodes')


def write_tree(tab, samples, path):
    def table(name, ty, vals):
        per = 8 if ty.startswith('[') else 24
        rows = [', '.join(str(v) for v in vals[i:i + per]) + ',' for i in range(0, len(vals), per)]
        return [f'pub static {name}: [{ty}; {len(vals)}] = ['] + ['\t' + r for r in rows] + ['];', '']
    lines = ['// letter-to-sound trees', '',
             f'pub const START: u8 = {START};', f'pub const SILENT: u8 = {SILENT};',
             f'pub const SECOND: usize = {tab.roots[1]};\t// where the second tree starts', '']
    lines += table('PRIMARY', 'bool', ['true' if p else 'false' for p in PRIMARY])
    lines += table('DEMOTE', 'u8', DEMOTE)
    nodes = []
    for i in range(tab.n):
        if tab.feat[i] == 255:
            nodes.append((255, tab.cls[i], tab.prim[i], tab.best[i]))
        else:
            off = tab.right[i] - i
            assert 0 < off < 65536
            nodes.append((tab.feat[i], tab.thr[i], off & 255, off >> 8))
    lines += ['// 4 bytes per node. A split: the feature it splits on (0-8 the letters l4..r4, 9-10 the classes given two',
              '// and one letters back, 11-19 whether l4..r4 are vowels, 20-21 the first tree\'s classes one and two',
              '// letters on), the threshold, and how far on the right child is (u16, low byte first; the left child is',
              '// the next node). A leaf: 255, the class, how likely (0-255) some primary-stressed class is, and the',
              '// likeliest primary-stressed class.']
    lines += table('NODES', '[u8; 4]', ['[' + ', '.join(map(str, n)) + ']' for n in nodes])
    lines += ['// words and the classes the trained tree gives them, for the tests', '#[cfg(test)]',
              f'pub static SAMPLES: [(&str, &[u8]); {len(samples)}] = [']
    lines += ['\t(' + json.dumps(w) + ', &[' + ', '.join(str(c) for c in ids) + ']),' for w, ids in samples]
    lines += ['];', '']
    open(path, 'w', encoding='utf-8', newline='\n').write('\n'.join(lines))


def write_dict(entries, path):
    # entries: (letters, phones, function?) in lookup order; the first entry for a spelling wins
    parts, seen = {}, set()
    for g, p, fw in entries:
        if g in seen:
            continue
        seen.add(g)
        parts.setdefault(ord(g[0].lower()), []).append((g, p, fw))
    # an entry: how many letters it shares with the word before (0-31) + 0x20 for a function word, the rest of its
    # letters, then its phones, the last of each with 0x80 set; 0xFF ends a letter's partition
    data, jump = bytearray(), {}
    for letter in sorted(parts):
        jump[letter] = len(data)
        prev = ()
        for g, p, fw in parts[letter]:
            word = [ord(c) for c in g]
            ids = [PHONE_ID[q] for q in p]
            assert len(word) < 32 and ids and max(word + ids) < 0x80, g
            k = 0
            while k < len(word) - 1 and k < len(prev) and word[k] == prev[k]:
                k += 1
            data.append(k | (0x20 if fw else 0))
            data.extend(word[k:-1] + [word[-1] | 0x80])
            data.extend(ids[:-1] + [ids[-1] | 0x80])
            prev = word
        data.append(0xFF)
    lines = ['// EXCEPTIONS DICTIONARY and LETTERS JUMP TABLE', '',
             'pub fn getLetterOffset(x: u8) -> usize {', '\tmatch x {']
    lines += [f'\t\t{k:<3} => {v},' for k, v in jump.items() if 97 <= k <= 122]
    lines += ['\t\t_   => 0,', '\t}', '}', '', f'pub static dictdat: [u8; {len(data)}] = [']
    lines += [''.join(f'0x{b:02x},' for b in data[i:i + 16]) for i in range(0, len(data), 16)]
    lines += ['];', '']
    open(path, 'w', encoding='utf-8', newline='\n').write('\n'.join(lines))
    return len(data), len(seen)


def main():
    aligned = read_pairs(sys.argv[1])
    if '--heldout' in sys.argv:
        heldout(aligned)
        return
    cmu = read_cmudict(sys.argv[2])
    manual = read_pairs(os.path.join(HERE, 'lexicon_manual.txt'))
    fwords = [l.strip() for l in open(os.path.join(HERE, 'function_words.txt'), encoding='utf-8')
              if l.strip() and not l.startswith('#')]
    over = {g: p for g, p in manual}
    fw_pron = {}
    for w in fwords:
        if w in cmu:
            # a hand entry gives a function word its own pronunciation (unreduced: "but")
            fw_pron[tuple(w)] = over.get(tuple(w)) or weak_form(cmu[w])
        else:
            print(f'not in CMUdict, left unflagged: {w}', file=sys.stderr)

    tab = Tables(train(aligned))
    misses, samples = [], []
    for n, (g, p) in enumerate(aligned):
        ids = decode(tab, g)
        if phones(ids) != plain(p):
            misses.append((g, plain(p)))
        if n % 4000 == 0:
            samples.append((''.join(g), ids))
    write_tree(tab, samples, os.path.join(SRC, '_lts_tree.rs'))
    print(f'tree {tab.n} nodes; it gets {len(misses)} of {len(aligned)} aligned words wrong')

    # capitalized and other non-lowercase spellings first, so an exact-case match beats the lowercase entry
    entries = [(g, p, False) for g, p in manual if ''.join(g) != ''.join(g).lower()]
    listed = set()
    # CMUdict words that could not be aligned never reached the tree, so nothing vouches for its guess
    seen = {g for g, _ in aligned}
    unaligned = [(tuple(w), p) for w, p in cmu.items() if tuple(w) not in seen]
    for g, p in misses + unaligned:
        listed.add(g)
        if g in fw_pron:
            entries.append((g, fw_pron[g], True))
        else:
            entries.append((g, over.get(g, p), False))
    entries += [(g, p, False) for g, p in manual if g not in listed and g not in fw_pron and ''.join(g) == ''.join(g).lower()]
    entries += [(g, p, True) for g, p in fw_pron.items() if g not in listed]
    print('dictionary %d bytes, %d entries' % write_dict(entries, os.path.join(SRC, '_dict.rs')))

    # words the tree says right are stems suffix matching may build on without a dictionary entry
    missed = {g for g, _ in misses}
    known = sorted({fnv(''.join(g)) for g, _ in aligned if g not in missed})
    dictionary = {}
    for g, p, _ in entries:
        dictionary.setdefault(''.join(g), [PHONE_ID[q] for q in p])
    lookup = stem_lookup(tab, dictionary, set(known))
    samples = []
    for n, (g, _) in enumerate(aligned[::997]):
        w = ''.join(g) + ['s', 'ed', 'ing', 'ness', 'ly', "'s", 'ies'][n % 7]
        if w not in cmu and w not in dictionary and compose(w, lookup):
            samples.append((w, compose(w, lookup)))
    write_known(known, samples, os.path.join(SRC, '_known.rs'))
    print(f'{len(known)} known words for suffix matching')


def write_known(known, samples, path):
    lines = ['// known words and suffix tables for suffix matching', '',
             '// FNV-1a hashes of the words the letter-to-sound tree says right, sorted',
             f'pub static KNOWN: [u32; {len(known)}] = [']
    lines += ['\t' + ', '.join(f'0x{h:08x}' for h in known[i:i + 10]) + ',' for i in range(0, len(known), 10)]
    lines += ['];', '', '// what a class\'s last phone makes of -s and -ed: 0 voiced, 1 t, 2 d, 3 voiceless, 4 voiceless sibilant, 5 voiced sibilant',
              f'pub static LAST_KIND: [u8; {len(PHONES)}] = [']
    lines += ['\t' + ', '.join(str(last_kind(i)) for i in range(len(PHONES))) + ',', '];', '']
    lines += [f'pub const PH_{q.upper()}: u8 = {PHONE_ID[q]};' for q in ('ax', 'd', 'f', 'iy', 'ix', 'l', 'n', 'ng', 's', 't', 'z')]
    lines += ['', '// words not in dictionary and the classes suffix matching gives them, for the tests', '#[cfg(test)]',
              f'pub static SAMPLES: [(&str, &[u8]); {len(samples)}] = [']
    lines += ['\t(' + json.dumps(w) + ', &[' + ', '.join(str(c) for c in ids) + ']),' for w, ids in samples]
    lines += ['];', '']
    open(path, 'w', encoding='utf-8', newline='\n').write('\n'.join(lines))


if __name__ == '__main__':
    main()
