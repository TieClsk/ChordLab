"""Render a synthetic test set of pop-style chord progressions with ground-truth labels.
Each "song": piano-like additive synth (inharmonic partials, decay), bass, melody, light noise percussion.
Outputs WAV + .lab (MIREX format) + key into OUT dir.
"""
import numpy as np, soundfile as sf, json, os, sys

SR = 22050
OUT = sys.argv[1]
os.makedirs(OUT, exist_ok=True)
rng = np.random.default_rng(7)
NAMES = ['C','C#','D','Eb','E','F','F#','G','Ab','A','Bb','B']

def midi_hz(m): return 440.0 * 2 ** ((m - 69) / 12)

def piano_note(m, dur, vel=0.6):
    n = int(dur * SR); t = np.arange(n) / SR
    f0 = midi_hz(m); B = 0.0004  # inharmonicity
    y = np.zeros(n)
    for k in range(1, 9):
        fk = k * f0 * np.sqrt(1 + B * k * k)
        if fk > SR / 2 - 500: break
        amp = (0.9 ** k) / k * (1 + 0.3 * rng.standard_normal() * 0.1)
        decay = np.exp(-t * (1.2 + 0.6 * k) * (f0 / 261) ** 0.3)
        y += amp * np.sin(2 * np.pi * fk * t + rng.uniform(0, 6.28)) * decay
    att = np.minimum(1, t / 0.005); rel = np.minimum(1, (dur - t) / 0.05).clip(0, 1)
    return vel * y * att * rel

def triad(root, quality, sev=False):
    iv = [0, 4, 7] if quality == 'maj' else [0, 3, 7]
    if sev: iv = iv + ([10] if quality == 'maj' else [10])
    return [root + i for i in iv]

# Roman-numeral progressions in major (degree, quality)
MAJ = {1:(0,'maj'),2:(2,'min'),3:(4,'min'),4:(5,'maj'),5:(7,'maj'),6:(9,'min')}
SONGS = [
    ('axis_I-V-vi-IV',        'C',  [1,5,6,4], 2, 112, 'block'),
    ('canon_I-V-vi-iii-IV-I-IV-V', 'D', [1,5,6,3,4,1,4,5], 2, 96, 'arp'),
    ('royal_IV-V-iii-vi',      'G', [4,5,3,6], 2, 104, 'block'),
    ('doowop_I-vi-IV-V',       'A', [1,6,4,5], 2, 120, 'arp'),
    ('jazz_ii-V-I-vi',         'Bb',[2,5,1,6], 2, 100, 'block7'),
    ('sad_vi-IV-I-V',          'E', [6,4,1,5], 2, 88,  'arp'),
    ('fast_I-IV-vi-V',         'F', [1,4,6,5], 1, 128, 'block'),
    ('ballad_I-iii-IV-V',      'Eb',[1,3,4,5], 4, 72,  'arp'),
]

meta = []
for name, key, prog, bars_per_chord, bpm, style in SONGS:
    tonic = NAMES.index(key)
    beat = 60 / bpm; chord_dur = 4 * beat * bars_per_chord / 2  # half-bars => chord every 2 beats * bars
    chord_dur = 2 * beat * bars_per_chord
    loops = max(2, int(np.ceil(40 / (chord_dur * len(prog)))))
    total = chord_dur * len(prog) * loops + 1.0
    y = np.zeros(int(total * SR) + SR)
    lab = []
    t0 = 0.5
    for L in range(loops):
        for deg in prog:
            off, q = MAJ[deg]
            root_pc = (tonic + off) % 12
            root = 48 + root_pc  # C3..B3
            notes = triad(root, q, sev=(style == 'block7'))
            inv = rng.integers(0, 3)
            notes = sorted([n + (12 if i < inv else 0) for i, n in enumerate(notes)])
            s = int(t0 * SR)
            # bass
            b = piano_note(root - 12, chord_dur * 0.95, 0.5); y[s:s+len(b)] += b
            if style == 'arp':
                step = beat / 2; k = 0; tt = 0
                while tt < chord_dur - 1e-6:
                    nn = notes[k % len(notes)] + (12 if (k // len(notes)) % 2 else 0)
                    p = piano_note(nn, min(step * 3, chord_dur - tt), 0.35)
                    ss = s + int(tt * SR); y[ss:ss+len(p)] += p
                    tt += step; k += 1
            else:
                nb = int(round(chord_dur / beat))
                for bi in range(nb):
                    for nn in notes:
                        p = piano_note(nn, beat * 0.95, 0.28)
                        ss = s + int(bi * beat * SR); y[ss:ss+len(p)] += p
            # melody: chord tones + occasional passing tone (non-chord), octave 5
            nb = int(round(chord_dur / beat))
            for bi in range(nb):
                if rng.random() < 0.7:
                    pc = rng.choice(notes) % 12 if rng.random() < 0.75 else (tonic + rng.choice([2, 5, 9, 11])) % 12
                    p = piano_note(72 + pc, beat * 0.9, 0.3)
                    ss = s + int(bi * beat * SR); y[ss:ss+len(p)] += p
            # percussion: noise hats on 8ths, kick on beats 1,3
            for bi in range(nb * 2):
                ss = s + int(bi * beat / 2 * SR); n = int(0.03 * SR)
                y[ss:ss+n] += 0.05 * rng.standard_normal(n) * np.exp(-np.arange(n) / (0.008 * SR))
            lab.append((t0, t0 + chord_dur, f"{NAMES[root_pc]}:{q}"))
            t0 += chord_dur
    y += 0.003 * rng.standard_normal(len(y))
    y = y / np.max(np.abs(y)) * 0.9
    end = t0 + 0.5
    y = y[:int(end * SR)]
    sf.write(f"{OUT}/{name}.wav", y, SR)
    with open(f"{OUT}/{name}.lab", 'w') as f:
        f.write(f"0.000\t0.500\tN\n")
        for a, b, c in lab: f.write(f"{a:.3f}\t{b:.3f}\t{c}\n")
        f.write(f"{t0:.3f}\t{end:.3f}\tN\n")
    meta.append({'name': name, 'key': key + ':major', 'bpm': bpm, 'dur': round(end, 1), 'style': style})
json.dump(meta, open(f"{OUT}/meta.json", 'w'), indent=1)
print(json.dumps(meta, indent=1))
