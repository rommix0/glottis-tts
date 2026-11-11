"""Build the release demo and gather it with the docs and licences into dist/.

usage: python tools/build_dist.py
"""
import os
import shutil
import subprocess
import sys

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
DIST = os.path.join(ROOT, 'dist')
DOCS = ['README.md', 'how to use.txt', 'LICENSE-MIT', 'LICENSE-APACHE']


def main():
    subprocess.run(['cargo', 'build', '--release', '--example', 'tts_demo'], cwd=ROOT, check=True)
    exe = 'tts_demo.exe' if sys.platform == 'win32' else 'tts_demo'

    shutil.rmtree(DIST, ignore_errors=True)
    os.makedirs(DIST)
    shutil.copy2(os.path.join(ROOT, 'target', 'release', 'examples', exe), DIST)
    for name in DOCS:
        shutil.copy2(os.path.join(ROOT, name), DIST)
    print('dist ready:', DIST)


if __name__ == '__main__':
    main()
