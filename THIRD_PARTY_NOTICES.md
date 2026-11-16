# Third-party notices / 第三方声明

ChordLab's own code is licensed under **MIT OR Apache-2.0** (see `LICENSE-MIT`, `LICENSE-APACHE`).
The following third-party materials have their own terms. See [ADR-0007](docs/adr/0007-licensing.md).

| Material | License | How ChordLab uses it | Shipped in the main app? |
| --- | --- | --- | --- |
| Rust crates (Cargo.lock) | Mostly MIT / Apache-2.0; symphonia family and a few others MPL-2.0 | Linked into the engine and desktop app, unmodified | Yes (MPL-2.0 source is available at crates.io) |
| Splendid Grand Piano samples (AKAI Steinway, via [smpldsnds/sfzinstruments-splendid-grand-piano](https://github.com/smpldsnds/sfzinstruments-splendid-grand-piano) @ 009793e) | Public domain | Default piano sound | Yes ("Lite" pack: 52 of the AAC files, unmodified; trimmed when loaded) |
| madmom pre-trained models | CC BY-NC-SA 4.0 (non-commercial) | Optional "precise analysis" component, downloaded on the user's machine | **No** |
| Chordonomicon dataset (derived chord-transition statistics) | CC BY-NC 4.0 (non-commercial), attribution: Kantarelis et al., 2024 | Next-chord recommendation table in `data/third_party/` | Yes, as data, under CC BY-NC 4.0 |
| essentia / essentia.js | AGPL-3.0 | Benchmark scripts in `experiments/` only | **No** |

Because two components are non-commercial, ChordLab as distributed is for **non-commercial use**. The code itself remains MIT OR Apache-2.0.
