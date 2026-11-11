"""Build the text normalizer: src/_text_rules.rs from tools/normalizer_rules.txt, then src/_text.rs from
src/_textskeleton.rs with re2c.

usage: python tools/build_normalizer.py [RE2C]     (RE2C defaults to re2c on the PATH)

tools/normalizer_rules.txt holds the fixed-text rules (homographs, abbreviations, units, emoji), one per line:
    PATTERN<TAB>"REPLACEMENT"
in the lexer's own pattern syntax: "..." matches exactly, '...' matches letters in either case, `space` or
`wbound` is a space, tab or newline, `period` and `comma` are themselves, `?` makes the piece before it
optional and `|` separates alternatives. In the replacement, {s} (or any {...}) is added only after a plural
number, as with "5 km". Lines starting with # are comments.

Each alternative becomes one table entry, matched either exactly or in any case: any '...' piece makes the
whole alternative match in any case. A space in a pattern matches any space.

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


def piece(tok):
    """a pattern piece as (text, any case)"""
    if tok[0] == '"':
        return unescape(tok[1:-1]), False
    if tok[0] == "'":
        return unescape(tok[1:-1]).lower(), True
    named = {'space': ' ', 'wbound': ' ', 'period': '.', 'comma': ','}
    if tok in named:
        return named[tok], False
    raise ValueError(f'unknown pattern piece {tok}')


def expand(pattern):
    """the pattern's alternatives as (text, any case)"""
    toks = TOKEN.findall(pattern)
    if ''.join(toks).replace(' ', '') != pattern.replace(' ', ''):
        raise ValueError(f'unsupported syntax in {pattern}')
    # each alternative so far: (text, any case, the exact text in it)
    alts, cur, i = [], [('', False, '')], 0
    while i < len(toks):
        if toks[i] == '|':
            alts += cur
            cur = [('', False, '')]
            i += 1
            continue
        text, any_case = piece(toks[i])
        grown = [(t + text, a or any_case, e + ('' if any_case else text)) for t, a, e in cur]
        if i + 1 < len(toks) and toks[i + 1] == '?':
            cur = grown + cur
            i += 2
        else:
            cur = grown
            i += 1
    out = []
    for text, any_case, exact in alts + cur:
        # an any-case alternative is matched lowercased, so its exact parts must already be lowercase
        if any_case and exact != exact.lower():
            raise ValueError(f'uppercase exact text {exact!r} in an any-case pattern')
        out.append((text, any_case))
    return out


def read_rules(path):
    rules = []
    for n, line in enumerate(open(path, encoding='utf-8'), 1):
        line = line.rstrip('\n')
        if not line.strip() or line.lstrip().startswith('#'):
            continue
        pattern, repl = line.split('\t')
        text = unescape(repl.strip()[1:-1])
        try:
            rules.append((expand(pattern), text))
        except ValueError as e:
            sys.exit(f'{path}:{n}: {e}')
    return rules


def rust_str(s):
    def char(c):
        if c in '"\\':
            return '\\' + c
        return c if 0x20 <= ord(c) < 0x7F else f'\\u{{{ord(c):x}}}'
    return '"' + ''.join(char(c) for c in s) + '"'


def write_tables(rules, path):
    entries = [(p, any_case, t) for alts, t in rules for p, any_case in alts]
    first = [[] for _ in range(256)]
    for i, (p, any_case, _) in enumerate(entries):
        b = p.encode('utf-8')[0]
        if b == 0x20:
            starts = [0x20, 0x09, 0x0A]
        elif any_case and 0x61 <= b <= 0x7A:
            starts = [b, b - 0x20]
        else:
            starts = [b]
        for s in starts:
            first[s].append(i)
    lines = ['// fixed-text normalizer rules', '',
             '// (pattern, any case, replacement): a space in a pattern matches any space (space, tab, newline);',
             '// an any-case pattern is lowercase and matches letters in either case; in the replacement,',
             '// {s} (any {...}) is added only after a plural number',
             f'pub static RULES: [(&str, bool, &str); {len(entries)}] = [']
    lines += [f'\t({rust_str(p)}, {str(any_case).lower()}, {rust_str(t)}),' for p, any_case, t in entries]
    lines += ['];', '', '// the rules that can start with each byte, in rule order', 'pub static FIRST: [&[u16]; 256] = [']
    lines += ['\t&[' + ', '.join(map(str, f)) + '],' for f in first]
    lines += ['];', '']
    open(path, 'w', encoding='utf-8', newline='\n').write('\n'.join(lines))
    return len(entries)


def check_capitals(rules, manual):
    # a capitalized word in a replacement picks the other reading of a homograph ("to USE"): the dictionary must have
    # it in that exact case (tools/lexicon_manual.txt), or it silently falls back to the lowercase word
    have = {line.split('	')[0].replace(' ', '') for line in open(manual, encoding='utf-8') if '	' in line}
    missing = sorted({w for _, t in rules for w in re.findall(r"[A-Za-z']+", t) if len(w) > 1 and w.isupper()} - have)
    if missing:
        sys.exit(f'no exact-case entry in {manual} for: {", ".join(missing)}')


def main():
    rules = read_rules(os.path.join(HERE, 'normalizer_rules.txt'))
    check_capitals(rules, os.path.join(HERE, 'lexicon_manual.txt'))
    n = write_tables(rules, os.path.join(SRC, '_text_rules.rs'))
    print(f'{len(rules)} rules, {n} patterns')
    re2c = sys.argv[1] if len(sys.argv) > 1 else 're2c'
    # -c: the lexer's rules are in a condition, for the check that runs before each rule's action
    subprocess.run([re2c, '-c', '--lang', 'rust', '--no-unsafe', '--no-generation-date',
                    os.path.join(SRC, '_textskeleton.rs'), '-o', os.path.join(SRC, '_text.rs')], check=True)
    print('src/_text.rs regenerated')


if __name__ == '__main__':
    main()
