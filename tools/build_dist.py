"""Build the release demo and gather it with the docs and licences into dist/glottis-tts-v<version>/ and a zip of it.

usage: python tools/build_dist.py <version>      e.g. python tools/build_dist.py 1.01

Other versions already in dist/ are left as they are.
"""
import os
import shutil
import subprocess
import sys

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
DIST = os.path.join(ROOT, 'dist')
DOCS = ['README.md', 'how to use.txt', 'LICENSE-MIT', 'LICENSE-APACHE']
EXE = '.exe' if sys.platform == 'win32' else ''


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    name = 'glottis-tts-v' + sys.argv[1]
    out = os.path.join(DIST, name)

    subprocess.run(['cargo', 'build', '--release', '--example', 'tts_demo'], cwd=ROOT, check=True)

    shutil.rmtree(out, ignore_errors=True)
    os.makedirs(out)
    shutil.copy2(os.path.join(ROOT, 'target', 'release', 'examples', 'tts_demo' + EXE),
                 os.path.join(out, 'glottis-tts' + EXE))
    for doc in DOCS:
        shutil.copy2(os.path.join(ROOT, doc), out)
    zip_path = shutil.make_archive(out, 'zip', DIST, name)
    print('dist ready:', out, 'and', zip_path)


if __name__ == '__main__':
    main()
