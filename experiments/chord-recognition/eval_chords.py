import sys, os, json, time, glob, warnings
import numpy as np, librosa, mir_eval
warnings.filterwarnings('ignore')
D = sys.argv[1]
meta = {m['name']: m for m in json.load(open(f"{D}/meta.json"))}
NAMES = ['C','C#','D','Eb','E','F','F#','G','Ab','A','Bb','B']

# ---------- Method A: librosa chroma + 24 templates + Viterbi ----------
LABELS = [f"{n}:maj" for n in NAMES] + [f"{n}:min" for n in NAMES] + ['N']
T = np.zeros((25, 12))
for i in range(12):
    T[i, [i, (i+4)%12, (i+7)%12]] = 1
    T[12+i, [i, (i+3)%12, (i+7)%12]] = 1
T[24] = 0.2
T = T / np.linalg.norm(T, axis=1, keepdims=True)

def frames_to_lab(lbl, times, end):
    segs = []; s = 0
    for i in range(1, len(lbl)+1):
        if i == len(lbl) or lbl[i] != lbl[s]:
            segs.append((times[s], times[i] if i < len(lbl) else end, lbl[s])); s = i
    return segs

def librosa_tmpl(path):
    y, sr = librosa.load(path, sr=22050)
    hop = 2048
    yh = librosa.effects.harmonic(y)
    C = librosa.feature.chroma_cqt(y=yh, sr=sr, hop_length=hop, bins_per_octave=36)
    C = librosa.decompose.nn_filter(C, aggregate=np.median, metric='cosine')
    sim = T @ (C / (np.linalg.norm(C, axis=0, keepdims=True) + 1e-9))
    P = np.exp(sim * 12); P /= P.sum(0, keepdims=True)
    trans = librosa.sequence.transition_loop(25, 0.97)
    path_ = librosa.sequence.viterbi_discriminative(P, trans)
    times = librosa.frames_to_time(np.arange(C.shape[1]), sr=sr, hop_length=hop)
    return frames_to_lab([LABELS[i] for i in path_], times, len(y)/sr)

# ---------- Method B: Essentia HPCP + ChordsDetection ----------
def essentia_chords(path):
    import essentia.standard as es
    audio = es.MonoLoader(filename=path, sampleRate=44100)()
    fs, hs = 4096, 2048
    w = es.Windowing(type='blackmanharris62'); spec = es.Spectrum()
    peaks = es.SpectralPeaks(orderBy='magnitude', magnitudeThreshold=1e-5, minFrequency=40, maxFrequency=5000, maxPeaks=60)
    hpcp = es.HPCP(size=12, referenceFrequency=440, harmonics=8, nonLinear=False, normalized='unitMax')
    H = []
    for fr in es.FrameGenerator(audio, frameSize=fs, hopSize=hs, startFromZero=True):
        f, m = peaks(spec(w(fr))); H.append(hpcp(f, m))
    H = np.array(H, dtype=np.float32)
    chords, strength = es.ChordsDetection(hopSize=hs, sampleRate=44100, windowSize=1.0)(H)
    times = np.arange(len(chords)) * hs / 44100
    def conv(c):
        if c.endswith('m'): return c[:-1].replace('A#','Bb').replace('D#','Eb').replace('G#','Ab') + ':min'
        return c.replace('A#','Bb').replace('D#','Eb').replace('G#','Ab') + ':maj'
    return frames_to_lab([conv(c) for c in chords], times, len(audio)/44100)

def essentia_key(path):
    import essentia.standard as es
    audio = es.MonoLoader(filename=path, sampleRate=44100)()
    k, scale, strength = es.KeyExtractor(profileType='edma')(audio)
    return f"{k}:{scale}"

def essentia_bpm(path):
    import essentia.standard as es
    audio = es.MonoLoader(filename=path, sampleRate=44100)()
    return es.RhythmExtractor2013(method='multifeature')(audio)[0]

# ---------- Method C/D: madmom ----------
def madmom_cnn_crf(path):
    from madmom.audio.chroma import DeepChromaProcessor
    from madmom.features.chords import CNNChordFeatureProcessor, CRFChordRecognitionProcessor
    feats = CNNChordFeatureProcessor()(path)
    out = CRFChordRecognitionProcessor()(feats)
    return [(float(a), float(b), str(c)) for a, b, c in out]

def madmom_deepchroma(path):
    from madmom.audio.chroma import DeepChromaProcessor
    from madmom.features.chords import DeepChromaChordRecognitionProcessor
    out = DeepChromaChordRecognitionProcessor()(DeepChromaProcessor()(path))
    return [(float(a), float(b), str(c)) for a, b, c in out]

METHODS = {'A_librosa_template_viterbi': librosa_tmpl,
           'B_essentia_ChordsDetection': essentia_chords,
           'C_madmom_CNN_CRF': madmom_cnn_crf,
           'D_madmom_DeepChroma_CRF': madmom_deepchroma}
sel = sys.argv[2:] or list(METHODS)

def score(ref_path, est):
    ri, rl = mir_eval.io.load_labeled_intervals(ref_path)
    ei = np.array([[a, b] for a, b, _ in est]); el = [c for _, _, c in est]
    ei, el = mir_eval.util.adjust_intervals(ei, el, ri.min(), ri.max(), mir_eval.chord.NO_CHORD, mir_eval.chord.NO_CHORD)
    iv, r, e = mir_eval.util.merge_labeled_intervals(ri, rl, ei, el)
    dur = mir_eval.util.intervals_to_durations(iv)
    majmin = mir_eval.chord.weighted_accuracy(mir_eval.chord.majmin(r, e), dur)
    root = mir_eval.chord.weighted_accuracy(mir_eval.chord.root(r, e), dur)
    seg = mir_eval.chord.seg(ri, ei)
    return majmin, root, seg

results = {}
for m in sel:
    rows = []
    for wav in sorted(glob.glob(f"{D}/*.wav")):
        name = os.path.basename(wav)[:-4]
        t = time.time()
        try:
            est = METHODS[m](wav)
        except Exception as ex:
            print(m, name, 'ERROR', repr(ex)[:300]); rows.append(None); continue
        dt = time.time() - t
        mm, rt, sg = score(f"{D}/{name}.lab", est)
        rows.append({'song': name, 'majmin': mm, 'root': rt, 'seg': sg, 'sec': dt, 'audio_sec': meta[name]['dur']})
        print(f"{m:28s} {name:30s} majmin={mm:.3f} root={rt:.3f} seg={sg:.3f} t={dt:.1f}s")
    results[m] = rows
json.dump(results, open(f"{D}/../results_{'_'.join(s[0] for s in sel)}.json", 'w'), indent=1)
for m, rows in results.items():
    ok = [r for r in rows if r]
    if not ok: continue
    print(f"SUMMARY {m}: majmin={np.mean([r['majmin'] for r in ok]):.3f} root={np.mean([r['root'] for r in ok]):.3f} seg={np.mean([r['seg'] for r in ok]):.3f} RTF={sum(r['sec'] for r in ok)/sum(r['audio_sec'] for r in ok):.3f} n={len(ok)}")

if 'key' in os.environ.get('EXTRA', ''):
    for wav in sorted(glob.glob(f"{D}/*.wav")):
        name = os.path.basename(wav)[:-4]
        k = essentia_key(wav); b = essentia_bpm(wav)
        print(f"KEY {name:30s} ref={meta[name]['key']:10s} est={k:10s} | BPM ref={meta[name]['bpm']} est={b:.1f}")
