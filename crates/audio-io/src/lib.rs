//! Decode compressed/uncompressed audio into planar f32 and resample it.
//! Pure Rust (symphonia + rubato) so the same code runs on desktop and in wasm.

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymErr;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

#[derive(Debug)]
pub struct Decoded {
    /// planar channels
    pub channels: Vec<Vec<f32>>,
    pub sample_rate: u32,
}

impl Decoded {
    pub fn frames(&self) -> usize {
        self.channels.first().map_or(0, |c| c.len())
    }
    pub fn duration_s(&self) -> f64 {
        self.frames() as f64 / self.sample_rate as f64
    }
    pub fn to_mono(&self) -> Vec<f32> {
        let n = self.channels.len().max(1) as f32;
        (0..self.frames())
            .map(|i| self.channels.iter().map(|c| c[i]).sum::<f32>() / n)
            .collect()
    }
}

#[derive(Debug)]
pub enum DecodeError {
    Unsupported(String),
    Corrupt(String),
}

/// Decode a whole file held in memory. `ext` is a hint such as "m4a", "mp3", "wav".
pub fn decode(bytes: Vec<u8>, ext: Option<&str>) -> Result<Decoded, DecodeError> {
    let mss = MediaSourceStream::new(Box::new(std::io::Cursor::new(bytes)), Default::default());
    let mut hint = Hint::new();
    if let Some(e) = ext {
        hint.with_extension(e);
    }
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| DecodeError::Unsupported(e.to_string()))?;
    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| DecodeError::Unsupported("no audio track".into()))?;
    let track_id = track.id;
    let sr = track.codec_params.sample_rate.unwrap_or(44_100);
    let mut dec = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| DecodeError::Unsupported(e.to_string()))?;
    let mut chans: Vec<Vec<f32>> = Vec::new();
    let mut buf: Option<SampleBuffer<f32>> = None;
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymErr::IoError(_)) => break, // end of stream
            Err(SymErr::ResetRequired) => break,
            Err(e) => return Err(DecodeError::Corrupt(e.to_string())),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let audio = match dec.decode(&packet) {
            Ok(a) => a,
            Err(SymErr::DecodeError(_)) => continue, // skip a bad frame
            Err(e) => return Err(DecodeError::Corrupt(e.to_string())),
        };
        let spec = *audio.spec();
        let nch = spec.channels.count();
        let b = buf.get_or_insert_with(|| SampleBuffer::<f32>::new(audio.capacity() as u64, spec));
        if (b.capacity()) < audio.capacity() * nch {
            *b = SampleBuffer::<f32>::new(audio.capacity() as u64, spec);
        }
        b.copy_interleaved_ref(audio);
        if chans.is_empty() {
            chans = vec![Vec::new(); nch];
        }
        for (i, s) in b.samples().iter().enumerate() {
            chans[i % nch].push(*s);
        }
    }
    Ok(Decoded {
        channels: chans,
        sample_rate: sr,
    })
}

/// High-quality sinc resampling of planar audio.
pub fn resample(input: &[Vec<f32>], from: u32, to: u32) -> Vec<Vec<f32>> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    use rubato::{
        Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
    };
    let params = SincInterpolationParameters {
        sinc_len: 128,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 128,
        window: WindowFunction::BlackmanHarris2,
    };
    let chunk = 1024;
    let mut r =
        SincFixedIn::<f32>::new(to as f64 / from as f64, 1.0, params, chunk, input.len()).unwrap();
    let mut out: Vec<Vec<f32>> = vec![Vec::new(); input.len()];
    let n = input[0].len();
    let mut pos = 0;
    while pos < n {
        let end = (pos + chunk).min(n);
        let block: Vec<Vec<f32>> = input
            .iter()
            .map(|c| {
                let mut v = c[pos..end].to_vec();
                v.resize(chunk, 0.0);
                v
            })
            .collect();
        let res = r.process(&block, None).unwrap();
        for (o, c) in out.iter_mut().zip(res) {
            o.extend_from_slice(&c);
        }
        pos = end;
    }
    let expected = (n as f64 * to as f64 / from as f64).round() as usize;
    for o in out.iter_mut() {
        // drop filter delay at the start, trim to expected length
        let delay = r.output_delay();
        let v: Vec<f32> = o.iter().skip(delay).copied().take(expected).collect();
        *o = v;
    }
    out
}
