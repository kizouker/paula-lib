//! Minimal 16-bit stereo PCM WAV writer and reader (for tests).

pub fn to_bytes(pcm: &[i16], rate: u32) -> Vec<u8> {
    let data_len = (pcm.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&2u16.to_le_bytes()); // stereo
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 4).to_le_bytes());
    out.extend_from_slice(&4u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in pcm {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// Parse a WAV written by `to_bytes`: returns (rate, channels, samples).
pub fn parse(b: &[u8]) -> Option<(u32, u16, Vec<i16>)> {
    if b.len() < 44 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return None;
    }
    let channels = u16::from_le_bytes([b[22], b[23]]);
    let rate = u32::from_le_bytes(b[24..28].try_into().ok()?);
    let len = u32::from_le_bytes(b[40..44].try_into().ok()?) as usize;
    let data = b.get(44..44 + len)?;
    Some((rate, channels, data.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect()))
}

/// Read a PCM WAV from any tool (afconvert, SoX, a DAW) as one mono channel of 8-bit signed
/// samples: walks the RIFF chunks (skips LIST, FLLR, ...), takes 8-bit unsigned or 16/24-bit
/// signed PCM, averages stereo to mono. Returns (sample rate, samples).
pub fn read_mono8(b: &[u8]) -> Result<(u32, Vec<i8>), String> {
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return Err("not a RIFF/WAVE file".into());
    }
    let (mut fmt, mut data) = (None, None);
    let mut i = 12;
    while i + 8 <= b.len() {
        let id = &b[i..i + 4];
        let len = u32::from_le_bytes([b[i + 4], b[i + 5], b[i + 6], b[i + 7]]) as usize;
        let body = b.get(i + 8..(i + 8 + len).min(b.len())).unwrap_or(&[]);
        match id {
            b"fmt " => fmt = Some(body),
            b"data" => data = Some(body),
            _ => {}
        }
        i += 8 + len + (len & 1);
    }
    let fmt = fmt.ok_or("no fmt chunk")?;
    let data = data.ok_or("no data chunk")?;
    if fmt.len() < 16 {
        return Err("short fmt chunk".into());
    }
    let format = u16::from_le_bytes([fmt[0], fmt[1]]);
    let channels = u16::from_le_bytes([fmt[2], fmt[3]]).max(1) as usize;
    let rate = u32::from_le_bytes([fmt[4], fmt[5], fmt[6], fmt[7]]);
    let bits = u16::from_le_bytes([fmt[14], fmt[15]]);
    if format != 1 && format != 0xFFFE {
        return Err(format!("WAV format {format} is not integer PCM"));
    }
    let width = match bits {
        8 => 1,
        16 => 2,
        24 => 3,
        _ => return Err(format!("{bits}-bit WAV is not supported (8, 16 or 24)")),
    };
    // one sample as -1.0..1.0
    let value = |s: &[u8]| -> f64 {
        match width {
            1 => (s[0] as f64 - 128.0) / 128.0,
            2 => i16::from_le_bytes([s[0], s[1]]) as f64 / 32768.0,
            _ => (((s[2] as i32) << 24 | (s[1] as i32) << 16 | (s[0] as i32) << 8) >> 8) as f64 / 8_388_608.0,
        }
    };
    let frame = width * channels;
    let out = data
        .chunks_exact(frame)
        .map(|f| {
            let sum: f64 = f.chunks_exact(width).map(value).sum();
            (sum / channels as f64 * 128.0).round().clamp(-128.0, 127.0) as i8
        })
        .collect();
    Ok((rate, out))
}
