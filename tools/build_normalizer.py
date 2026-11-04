"""Build the text normalizer: src/_text_rules.rs from tools/normalizer_rules.txt, then src/_text.rs from
src/_textskeleton.rs with re2c.

usage: python tools/build_normalizer.py [RE2C]     (RE2C defaults to re2c on the PATH)

tools/normalizer_rules.txt holds the fixed-text rules (homographs, abbreviations, units, emoji), one per line:
    PATTERN<TAB>"REPLACEMENT"
in the lexer's own pattern syntax: "..." matches exactly, '...' matches letters in either case, `space` or
`wbound` is a space, tab or newline, `period` and `comma` are themselves, `?` makes the piece before it
optional and `|` separates alternatives. In the replacement, {s} (or any {...}) is added only after a plural
number, as with "5 km". Lines starting with # are comments.

At each position the normalizer takes the longest pattern that matches there, the earlier rule on a tie, unless
one of the lexer's own rules matches longer (see table_rule in src/_textskeleton.rs).
"""
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(HERE, '..', 'src')
TOKEN = re.compile(r'"(?:[^"\\]|\\.)*"|\'(?:[^\'\\]|\\.)*\'|[A-Za-z_][A-Za-z_0-9]*|[?|]')
SPACE = 0x01   # pattern byte: any space
EITHER = 0x02  # pattern byte: the next (lowercase) letter in either case


def unescape(s):
    # the lexer's string escapes, with UTF-8 on: \xHH, \uHHHH and \UHHHHHHHH are code points
    out, i = [], 0
    while i < len(s):
        if s[i] == '\\':
            n = s[i + 1]
            width = {'x': 2, 'u': 4, 'U': 8}.get(n)
            if width:
                out.append(chr(int(s[i + 2:i + 2 + width], 16))); i += 2 + width; continue
            out.append({'n': '\n', 't': '\t'}.get(n, n)); i += 2; continue
        out.append(s[i]); i += 1
    return ''.join(out)


def pieces(tok):
    if tok[0] == '"':
        return [bytes([b]) for b in unescape(tok[1:-1]).encode('utf-8')]
    if tok[0] == "'":
        out = []
        for b in unescape(tok[1:-1]).encode('utf-8'):
            out.append(bytes([EITHER, b | 0x20]) if 0x41 <= b <= 0x5A or 0x61 <= b <= 0x7A else bytes([b]))
        return out
    named = {'space': [bytes([SPACE])], 'wbound': [bytes([SPACE])], 'period': [b'.'], 'comma': [b',']}
    if tok in named:
        return named[tok]
    raise ValueError(f'unknown pattern piece {tok}')


def expand(pattern):
    """the pattern's alternatives as pattern byte strings"""
    toks = TOKEN.findall(pattern)
    if ''.join(toks).replace(' ', '') != pattern.replace(' ', ''):
        raise ValueError(f'unsupported syntax in {pattern}')
    alts, cur, i = [], [b''], 0
    while i < len(toks):
        if toks[i] == '|':
            alts += cur; cur = [b'']; i += 1; continue
        piece = b''.join(pieces(toks[i]))
        if i + 1 < len(toks) and toks[i + 1] == '?':
            cur = [c + piece for c in cur] + cur; i += 2
        else:
            cur = [c + piece for c in cur]; i += 1
    return alts + cur


def read_rules(path):
    rules = []
    for n, line in enumerate(open(path, encoding='utf-8'), 1):
        line = line.rstrip('\n')
        if not line.strip() or line.lstrip().startswith('#'):
            continue
        pattern, repl = line.split('\t')
        text = unescape(repl.strip()[1:-1])
        m = re.search(r'\{([^}]*)\}', text)
        head, plural, tail = (text[:m.start()], m.group(1), text[m.end():]) if m else (text, '', '')
        try:
            rules.append((expand(pattern), head, plural, tail))
        except ValueError as e:
            sys.exit(f'{path}:{n}: {e}')
    return rules


def rust_bytes(b):
    return 'b"' + ''.join(chr(c) if 0x20 <= c < 0x7F and c not in (0x22, 0x5C) else f'\\x{c:02x}' for c in b) + '"'


def rust_str(s):
    return '"' + ''.join(c if 0x20 <= ord(c) < 0x7F and c not in '"\\' else f'\\u{{{ord(c):x}}}' for c in s) + '"'


def write_tables(rules, path):
    entries = [(p, h, pl, t) for alts, h, pl, t in rules for p in alts]
    first = [[] for _ in range(256)]
    for i, (p, _, _, _) in enumerate(entries):
        if p[0] == SPACE:
            starts = [0x20, 0x08, 0x0A]
        elif p[0] == EITHER:
            starts = [p[1], p[1] - 0x20]
        else:
            starts = [p[0]]
        for s in starts:
            first[s].append(i)
    lines = ['// fixed-text normalizer rules', '',
             f'// pattern bytes: {SPACE:#04x} any space (space, tab, newline), {EITHER:#04x} then a lowercase letter that',
             '// matches either case; then the replacement, the plural ending added after a plural number, the rest',
             f'pub static RULES: [(&[u8], &str, &str, &str); {len(entries)}] = [']
    lines += [f'\t({rust_bytes(p)}, {rust_str(h)}, {rust_str(pl)}, {rust_str(t)}),' for p, h, pl, t in entries]
    lines += ['];', '', '// the rules that can start with each byte, in rule order', 'pub static FIRST: [&[u16]; 256] = [']
    lines += ['\t&[' + ', '.join(map(str, f)) + '],' for f in first]
    lines += ['];', '']
    open(path, 'w', encoding='utf-8', newline='\n').write('\n'.join(lines))
    return len(entries)


def main():
    rules = read_rules(os.path.join(HERE, 'normalizer_rules.txt'))
    n = write_tables(rules, os.path.join(SRC, '_text_rules.rs'))
    print(f'{len(rules)} rules, {n} patterns')
    re2c = sys.argv[1] if len(sys.argv) > 1 else 're2c'
    subprocess.run([re2c, '--lang', 'rust', '--no-unsafe', '--no-generation-date', os.path.join(SRC, '_textskeleton.rs'),
                    '-o', os.path.join(SRC, '_text.rs')], check=True)
    print('src/_text.rs regenerated')


if __name__ == '__main__':
    main()
