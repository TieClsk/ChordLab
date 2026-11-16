"""Tests for the Lite piano pack builder (#9). Run: python -m unittest discover scripts/packs"""
import json
import subprocess
import os
import tempfile
import unittest

import build_piano_lite as b

# keycenters of the upstream "MP" layer (subset is enough to exercise the logic)
CENTERS = [23, 27, 29, 31, 33, 35, 37, 38, 40, 41, 43, 45, 47, 48, 50, 52, 53, 55, 56, 57, 58, 59, 60,
           62, 64, 65, 67, 69, 71, 72, 74, 76, 77, 79, 80, 81, 82, 83, 85, 86, 87, 88, 89, 90, 91, 92,
           93, 94, 95, 96, 97, 98, 100, 101, 103, 105, 106, 107]


class SelectRoots(unittest.TestCase):
    def test_roots_are_at_least_three_semitones_apart(self):
        roots = b.select_roots(CENTERS, spacing=3)
        gaps = [b2 - a for a, b2 in zip(roots, roots[1:])]
        self.assertTrue(all(g >= 3 for g in gaps), gaps)

    def test_keeps_the_lowest_and_highest_sample(self):
        roots = b.select_roots(CENTERS, spacing=3)
        self.assertEqual(roots[0], CENTERS[0])
        self.assertEqual(roots[-1], CENTERS[-1])

    def test_about_one_sample_per_three_semitones(self):
        roots = b.select_roots(CENTERS, spacing=3)
        self.assertLessEqual(len(roots), (CENTERS[-1] - CENTERS[0]) // 3 + 2)
        self.assertGreaterEqual(len(roots), 20)


class KeyRanges(unittest.TestCase):
    def test_every_piano_key_is_covered_exactly_once(self):
        roots = b.select_roots(CENTERS, spacing=3)
        ranges = b.key_ranges(roots, lo=21, hi=108)
        covered = [k for lo, hi in ranges for k in range(lo, hi + 1)]
        self.assertEqual(covered, list(range(21, 109)))

    def test_no_key_is_pitch_shifted_more_than_three_semitones(self):
        roots = b.select_roots(CENTERS, spacing=3)
        for root, (lo, hi) in zip(roots, b.key_ranges(roots, lo=21, hi=108)):
            worst = max(abs(lo - root), abs(hi - root))
            # the ends of the keyboard stretch the outermost samples
            if lo != 21 and hi != 108:
                self.assertLessEqual(worst, 3, (root, lo, hi))


class Layers(unittest.TestCase):
    def test_velocity_layers_cover_1_to_127_exactly_once(self):
        vels = [v for _, lo, hi in b.LAYERS for v in range(lo, hi + 1)]
        self.assertEqual(vels, list(range(1, 128)))

    def test_default_ui_velocity_uses_the_softer_layer(self):
        # UI clicks use velocity 100; the softer layer sounds less harsh (#36)
        layer = next(name for name, lo, hi in b.LAYERS if lo <= 100 <= hi)
        self.assertEqual(layer, 'MP')


class Build(unittest.TestCase):
    def test_builds_manifest_and_copies_only_selected_files(self):
        with tempfile.TemporaryDirectory() as src, tempfile.TemporaryDirectory() as out:
            os.makedirs(os.path.join(src, 'data'))
            os.makedirs(os.path.join(src, 'samples'))
            for layer in ['MP', 'FF', 'PP', 'MF']:
                lines = []
                for i, c in enumerate(CENTERS):
                    name = f'{layer.title()} K{c}.$EXT'
                    lines.append(f'<region> region_label={i} lokey={c} hikey={c} pitch_keycenter={c:03d} sample={name}')
                    with open(os.path.join(src, 'samples', name.replace('$EXT', 'm4a')), 'wb') as f:
                        f.write(b'x' * 10)
                with open(os.path.join(src, 'data', f'{layer}.txt'), 'w') as f:
                    f.write('\n'.join(lines))
            m = b.build(src, out, source_commit='abc123')
            with open(os.path.join(out, 'manifest.json')) as f:
                on_disk = json.load(f)
            self.assertEqual(m, on_disk)
            files = sorted(os.listdir(os.path.join(out, 'samples')))
            self.assertEqual(len(files), len(m['samples']))
            self.assertTrue(all(f.startswith(('Mp ', 'Ff ')) for f in files), files[:3])
            self.assertEqual(m['license'], 'Public domain')
            self.assertEqual(m['source']['commit'], 'abc123')
            self.assertEqual(m['max_seconds'], 6.0)
            for z in m['zones']:
                self.assertIn(z['sample_id'], range(len(m['samples'])))


if __name__ == '__main__':
    unittest.main()


class SourceCheckout(unittest.TestCase):
    """A cached upstream checkout is reused only if it is complete and at the pinned commit
    (a CI cache once restored a checkout with its files missing)."""

    def _repo(self, d):
        run = lambda *a: subprocess.run(['git', '-C', d, *a], check=True, capture_output=True)  # noqa: E731
        os.makedirs(os.path.join(d, 'data'))
        with open(os.path.join(d, 'data', 'MP.txt'), 'w') as f:
            f.write('x')
        run('init', '-q')
        run('-c', 'user.email=t@t', '-c', 'user.name=t', 'add', '.')
        run('-c', 'user.email=t@t', '-c', 'user.name=t', 'commit', '-q', '-m', 'x')
        return subprocess.run(['git', '-C', d, 'rev-parse', 'HEAD'], capture_output=True, text=True).stdout.strip()

    def test_complete_checkout_at_the_pinned_commit_is_reused(self):
        with tempfile.TemporaryDirectory() as d:
            sha = self._repo(d)
            self.assertTrue(b.source_is_usable(d, sha))

    def test_checkout_with_missing_files_is_not_reused(self):
        with tempfile.TemporaryDirectory() as d:
            sha = self._repo(d)
            os.remove(os.path.join(d, 'data', 'MP.txt'))
            self.assertFalse(b.source_is_usable(d, sha))

    def test_checkout_at_another_commit_is_not_reused(self):
        with tempfile.TemporaryDirectory() as d:
            self._repo(d)
            self.assertFalse(b.source_is_usable(d, '0' * 40))

    def test_missing_or_non_git_directory_is_not_reused(self):
        with tempfile.TemporaryDirectory() as d:
            self.assertFalse(b.source_is_usable(os.path.join(d, 'nope'), 'x'))
            os.makedirs(os.path.join(d, 'data'))
            self.assertFalse(b.source_is_usable(d, 'x'))

    def test_default_checkout_location_is_outside_target_and_public(self):
        path = b.default_source_dir()
        self.assertNotIn(os.sep + 'target' + os.sep, path)
        self.assertNotIn(os.sep + 'public' + os.sep, path)
