//! Minimal WAV writer (16-bit PCM and 32-bit float), no dependencies.

use std::io::Write;

fn header(w: &mut impl Write, sr: u32, frames: usize, bits: u16, fmt: u16) -> std::io::Result<()> {
    let ch = 2u16;
    let block_align = ch * bits / 8;
    let data_len = frames as u32 * block_align as u32;
    w.write_all(b"RIFF")?;
    w.write_all(&(36 + data_len).to_le_bytes())?;
    w.write_all(b"WAVEfmt ")?;
    w.write_all(&16u32.to_le_bytes())?;
    w.write_all(&fmt.to_le_bytes())?;
    w.write_all(&ch.to_le_bytes())?;
    w.write_all(&sr.to_le_bytes())?;
    w.write_all(&(sr * block_align as u32).to_le_bytes())?;
    w.write_all(&block_align.to_le_bytes())?;
    w.write_all(&bits.to_le_bytes())?;
    w.write_all(b"data")?;
    w.write_all(&data_len.to_le_bytes())
}

pub fn write_wav16(path: &str, sr: u32, l: &[f32], r: &[f32]) -> std::io::Result<()> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    header(&mut f, sr, l.len(), 16, 1)?;
    for i in 0..l.len() {
        for x in [l[i], r[i]] {
            let s = (x.clamp(-1.0, 1.0) * 32767.0).round() as i16;
            f.write_all(&s.to_le_bytes())?;
        }
    }
    Ok(())
}

#[allow(dead_code)]
pub fn write_wav_f32(path: &str, sr: u32, l: &[f32], r: &[f32]) -> std::io::Result<()> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    header(&mut f, sr, l.len(), 32, 3)?;
    for i in 0..l.len() {
        f.write_all(&l[i].to_le_bytes())?;
        f.write_all(&r[i].to_le_bytes())?;
    }
    Ok(())
}
