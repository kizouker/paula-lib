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
