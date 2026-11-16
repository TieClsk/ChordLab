// Spike: tonal (theory core), next-chord recommender, essentia.js (WASM) chord detection in a JS runtime
import { Chord, Key, Progression, RomanNumeral, Note, Interval } from 'tonal';
import fs from 'fs';
import { createRequire } from 'module';
const require = createRequire(import.meta.url);

const log = (...a) => console.log(...a);
log('=== 1. tonal: chord parsing & theory ===');
for (const c of ['C', 'Am7', 'G7/B', 'Fmaj7', 'Dm7b5', 'Csus4', 'E7#9', 'Bbadd9'])
  log(c.padEnd(8), '->', Chord.get(c).notes.join(' ').padEnd(14), '| type:', Chord.get(c).type);
log('detect [E,G,C]     ->', Chord.detect(['E', 'G', 'C']));
log('detect [A,C,E,G]   ->', Chord.detect(['A', 'C', 'E', 'G']));
log('detect [F,A,C,E]   ->', Chord.detect(['F', 'A', 'C', 'E']));
log('C major diatonic 7ths ->', Key.majorKey('C').chords.join(' '));
log('A minor natural triads ->', Key.minorKey('A').natural.triads.join(' '));
log('I V vi IV in Eb ->', Progression.fromRomanNumerals('Eb', ['I', 'V', 'vim', 'IV']).join(' '));
log('[C,G,Am,F] -> roman in C ->', Progression.toRomanNumerals('C', ['C', 'G', 'Am', 'F']).join(' '));
log('voicing C4 + Am7 notes w/ octave ->', Chord.getChord('m7', 'A4').notes.join(' '), 'midi', Chord.getChord('m7', 'A4').notes.map(Note.midi).join(','));

// micro-benchmark
let t = performance.now();
for (let i = 0; i < 100000; i++) { Chord.get('Am7'); Chord.detect(['A', 'C', 'E', 'G']); }
log(`100k Chord.get + Chord.detect: ${(performance.now() - t).toFixed(0)} ms`);

log('\n=== 2. next-chord recommender (1st/2nd-order Markov over roman numerals) ===');
// tiny seed corpus of well-known pop progressions (would be replaced by Chordonomicon stats)
const corpus = [
  'I V vi IV', 'I vi IV V', 'vi IV I V', 'I IV vi V', 'IV V iii vi', 'I V vi iii IV I IV V',
  'ii V I', 'I iii IV V', 'I IV V I', 'vi V IV V', 'I V IV V', 'IV I V vi', 'I vi ii V', 'vi ii V I',
  'I IV I V', 'IV V I vi', 'I V vi IV', 'I V vi IV', 'vi IV I V', 'I vi IV V',
].map(s => s.split(' '));
const bigram = {}, trigram = {};
for (const seq of corpus) {
  const loop = [...seq, ...seq.slice(0, 2)];
  for (let i = 0; i < seq.length; i++) {
    const a = loop[i], b = loop[i + 1], c = loop[i + 2];
    (bigram[a] ??= {})[b] = (bigram[a][b] || 0) + 1;
    (trigram[a + ' ' + b] ??= {})[c] = (trigram[a + ' ' + b][c] || 0) + 1;
  }
}
const top = (m, n = 3) => { const s = Object.values(m).reduce((x, y) => x + y, 0); return Object.entries(m).sort((a, b) => b[1] - a[1]).slice(0, n).map(([k, v]) => `${k} ${(100 * v / s).toFixed(0)}%`); };
function recommend(history, key = 'C') {
  const h = history.slice(-2);
  const dist = (h.length === 2 && trigram[h.join(' ')]) || bigram[h.at(-1)] || {};
  return top(dist).map(r => { const [rn, p] = r.split(' '); return `${rn}(${Progression.fromRomanNumerals(key, [rn.replace(/^(vi|iii|ii)$/, x => x + 'm')])[0]}) ${p}`; });
}
log('after I        ->', recommend(['I']).join(', '));
log('after I V      ->', recommend(['I', 'V']).join(', '));
log('after I V vi   ->', recommend(['I', 'V', 'vi']).join(', '));
log('after ii V (G) ->', recommend(['ii', 'V'], 'G').join(', '));

log('\n=== 3. essentia.js (WASM) chord detection on test WAVs ===');
const { Essentia, EssentiaWASM } = require('essentia.js');
const ess = new Essentia(EssentiaWASM);
log('essentia.js version', ess.version);
function readWav(p) {
  const b = fs.readFileSync(p); let o = 12, fmt, data;
  while (o < b.length) { const id = b.toString('ascii', o, o + 4), sz = b.readUInt32LE(o + 4);
    if (id === 'fmt ') fmt = { ch: b.readUInt16LE(o + 10), sr: b.readUInt32LE(o + 12), bits: b.readUInt16LE(o + 22) };
    if (id === 'data') data = b.subarray(o + 8, o + 8 + sz); o += 8 + sz + (sz % 2); }
  const n = data.length / 2, x = new Float32Array(n);
  for (let i = 0; i < n; i++) x[i] = data.readInt16LE(i * 2) / 32768;
  return { x, sr: fmt.sr };
}
const D = process.argv[2];
const names = ['C', 'C#', 'D', 'Eb', 'E', 'F', 'F#', 'G', 'Ab', 'A', 'Bb', 'B'];
const norm = c => { const m = c.endsWith('m'); let r = m ? c.slice(0, -1) : c; r = { 'A#': 'Bb', 'D#': 'Eb', 'G#': 'Ab', 'Db': 'C#', 'Gb': 'F#' }[r] || r; return r + (m ? ':min' : ':maj'); };
let totA = 0, totT = 0, acc = [];
for (const f of fs.readdirSync(D).filter(f => f.endsWith('.wav')).sort()) {
  const { x, sr } = readWav(`${D}/${f}`);
  const t0 = performance.now();
  const fs_ = 4096, hs = 2048;
  const frames = ess.FrameGenerator(x, fs_, hs);
  const pcp = new EssentiaWASM.VectorVectorFloat();
  for (let i = 0; i < frames.size(); i++) {
    const w = ess.Windowing(frames.get(i), true, fs_, 'blackmanharris62').frame;
    const sp = ess.Spectrum(w, fs_).spectrum;
    const pk = ess.SpectralPeaks(sp, 1e-5, 5000, 60, 40, 'magnitude', sr);
    const h = ess.HPCP(pk.frequencies, pk.magnitudes, true, 500, 0, 5000, false, 40, false, 'unitMax', 440, sr, 12);
    pcp.push_back(h.hpcp);
  }
  const cd = ess.ChordsDetection(pcp, hs, sr, 1.0);
  const chords = ess.vectorToArray ? [] : [];
  const n = cd.chords.size(); const est = []; for (let i = 0; i < n; i++) est.push(norm(cd.chords.get(i)));
  const dt = performance.now() - t0;
  // frame-level accuracy vs .lab
  const lab = fs.readFileSync(`${D}/${f.replace('.wav', '.lab')}`, 'utf8').trim().split('\n').map(l => l.split('\t')).map(([a, b, c]) => [+a, +b, c]);
  let ok = 0, tot = 0;
  est.forEach((c, i) => { const tt = i * hs / sr; const r = lab.find(([a, b]) => tt >= a && tt < b); if (r && r[2] !== 'N') { tot++; if (r[2] === c) ok++; } });
  acc.push(ok / tot); totA += x.length / sr; totT += dt / 1000;
  log(`${f.padEnd(34)} frame-acc=${(ok / tot).toFixed(3)}  ${dt.toFixed(0)} ms for ${(x.length / sr).toFixed(0)} s audio`);
}
log(`MEAN frame-acc=${(acc.reduce((a, b) => a + b) / acc.length).toFixed(3)}  RTF=${(totT / totA).toFixed(3)}`);
const kx = readWav(`${D}/axis_I-V-vi-IV.wav`).x;
try { const k = ess.KeyExtractor(ess.arrayToVector(kx)); log('KeyExtractor(axis, ref C major) ->', k.key, k.scale); } catch (e) { log('KeyExtractor error', String(e).slice(0, 200)); }
