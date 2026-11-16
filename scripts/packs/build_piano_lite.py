#!/usr/bin/env python3
"""Build the "Lite" grand piano sample pack (#9, ADR-0005, ADR-0008).

Source: Splendid Grand Piano (AKAI Steinway samples, public domain), via
https://github.com/smpldsnds/sfzinstruments-splendid-grand-piano (AAC .m4a files).

The pack keeps about one sample every three semitones in two velocity layers and
copies the selected files unchanged. Trimming to MAX_SECONDS with a fade-out happens
when the pack is loaded (crate `sample-packs`), so no audio encoder is needed here.

Usage: python scripts/packs/build_piano_lite.py --src <checkout> --out <dir> [--commit SHA]
"""
import argparse
import json
import os
import re
import shutil
import subprocess

NAME = 'piano-lite'
SOURCE_REPO = 'https://github.com/smpldsnds/sfzinstruments-splendid-grand-piano'
SOURCE_COMMIT = '009793ebf849aee57f9a213aeaf8f2d1e2178f4f'
# (upstream layer file, lowest velocity, highest velocity). The default UI velocity
# (100) uses the softer MP layer, which sounds less harsh than FF (#36).
LAYERS = [('MP', 1, 108), ('FF', 109, 127)]
SPACING = 3
LOWEST_KEY, HIGHEST_KEY = 21, 108  # A0..C8
MAX_SECONDS = 6.0
FADE_SECONDS = 0.5
RELEASE_SECONDS = 0.35


def select_roots(centers, spacing=SPACING):
    """Keep root notes at least `spacing` semitones apart, always keeping both ends."""
    centers = sorted(set(centers))
    roots = [centers[0]]
    for c in centers[1:-1]:
        if c - roots[-1] >= spacing and centers[-1] - c >= spacing:
            roots.append(c)
    if centers[-1] != roots[-1]:
        roots.append(centers[-1])
    return roots


def key_ranges(roots, lo=LOWEST_KEY, hi=HIGHEST_KEY):
    """Split [lo, hi] at midpoints between neighbouring roots."""
    ranges = []
    for i, r in enumerate(roots):
        start = lo if i == 0 else (roots[i - 1] + r) // 2 + 1
        end = hi if i == len(roots) - 1 else (r + roots[i + 1]) // 2
        ranges.append((start, end))
    return ranges


def read_regions(path):
    """{keycenter: sample file name} from an upstream SFZ region list."""
    out = {}
    with open(path, encoding='utf-8') as f:
        for line in f:
            if '<region>' not in line:
                continue
            center = int(re.search(r'pitch_keycenter=(\d+)', line).group(1))
            sample = re.search(r'sample=(.+?\.\$EXT)', line).group(1).replace('$EXT', 'm4a')
            out[center] = sample
    return out


def build(src, out, source_commit=SOURCE_COMMIT):
    os.makedirs(os.path.join(out, 'samples'), exist_ok=True)
    samples, zones = [], []
    for layer, lo_vel, hi_vel in LAYERS:
        regions = read_regions(os.path.join(src, 'data', f'{layer}.txt'))
        roots = select_roots(list(regions))
        for root, (lo, hi) in zip(roots, key_ranges(roots)):
            file = regions[root]
            shutil.copyfile(os.path.join(src, 'samples', file), os.path.join(out, 'samples', file))
            zones.append({'sample_id': len(samples), 'root_note': root, 'lo_note': lo, 'hi_note': hi,
                          'lo_vel': lo_vel, 'hi_vel': hi_vel, 'gain': 1.0})
            samples.append({'file': f'samples/{file}', 'layer': layer, 'root_note': root})
    manifest = {
        'name': NAME,
        'version': 1,
        'license': 'Public domain',
        'credit': 'AKAI Steinway samples (Splendid Grand Piano)',
        'source': {'repo': SOURCE_REPO, 'commit': source_commit},
        'max_seconds': MAX_SECONDS,
        'fade_seconds': FADE_SECONDS,
        'release_s': RELEASE_SECONDS,
        'samples': samples,
        'zones': zones,
    }
    with open(os.path.join(out, 'manifest.json'), 'w', encoding='utf-8') as f:
        json.dump(manifest, f, ensure_ascii=False, indent=1)
        f.write('\n')
    return manifest


def default_source_dir():
    """Upstream checkout location: outside target/ (restored by the Rust CI cache) and
    outside public/ (bundled with the app)."""
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
    return os.path.join(repo_root, '.cache', 'pack-src', 'splendid-grand-piano')


def source_is_usable(src, commit):
    """True only for a git checkout at `commit` with every tracked file present and unmodified."""
    if not os.path.isdir(os.path.join(src, '.git')):
        return False
    git = lambda *a: subprocess.run(['git', '-C', src, *a], capture_output=True, text=True)  # noqa: E731
    if git('rev-parse', 'HEAD').stdout.strip() != commit:
        return False
    return git('diff', '--quiet', 'HEAD').returncode == 0


def ensure_source(src, commit=SOURCE_COMMIT):
    if source_is_usable(src, commit):
        return
    if os.path.exists(src):
        shutil.rmtree(src)
    os.makedirs(os.path.dirname(src), exist_ok=True)
    subprocess.run(['git', 'clone', '--quiet', '--filter=blob:none', SOURCE_REPO, src], check=True)
    subprocess.run(['git', '-C', src, 'checkout', '--quiet', commit], check=True)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--src', help='upstream checkout; cloned at the pinned commit if missing')
    ap.add_argument('--out', required=True)
    a = ap.parse_args()
    src = a.src or default_source_dir()
    if not a.src:
        ensure_source(src)
    commit = subprocess.run(['git', '-C', src, 'rev-parse', 'HEAD'], capture_output=True, text=True).stdout.strip() or SOURCE_COMMIT
    if os.path.isdir(a.out):
        shutil.rmtree(a.out)
    m = build(src, a.out, source_commit=commit)
    size = sum(os.path.getsize(os.path.join(a.out, s['file'])) for s in m['samples'])
    print(f"{NAME}: {len(m['samples'])} samples, {len(m['zones'])} zones, {size / 1e6:.1f} MB -> {a.out}")


if __name__ == '__main__':
    main()
