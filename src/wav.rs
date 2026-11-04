//! WAV writer: stereo, 16-bit PCM or 32-bit IEEE float, little-endian RIFF.
//!
//! PCM files use the canonical 44-byte header (`RIFF`, `fmt ` of 16 bytes, `data`). Float files
//! use an 18-byte `fmt ` (with `cbSize` = 0) and a `fact` chunk holding the frame count, as the
//! format requires for non-PCM data.

use crate::error::Error;
use crate::render::Mix;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFormat {
    Pcm16,
    Float32,
}

impl SampleFormat {
    pub fn describe(self) -> &'static str {
        match self {
            SampleFormat::Pcm16 => "16-bit PCM",
            SampleFormat::Float32 => "32-bit float",
        }
    }
}

const CHANNELS: u16 = 2;

/// Encode a mix as a complete WAV file in memory.
pub fn encode(mix: &Mix, format: SampleFormat) -> Result<Vec<u8>, Error> {
    if mix.left.len() != mix.right.len() {
        return Err(Error::msg(format!(
            "channel lengths differ ({} left, {} right)",
            mix.left.len(),
            mix.right.len()
        )));
    }
    let frames = mix.left.len();
    let (tag, bits, fmt_len, fact): (u16, u16, u32, bool) = match format {
        SampleFormat::Pcm16 => (1, 16, 16, false),
        SampleFormat::Float32 => (3, 32, 18, true),
    };
    let block_align = CHANNELS * bits / 8;
    let data_len = frames as u64 * block_align as u64;
    let riff_len = 4 + (8 + fmt_len as u64) + if fact { 12 } else { 0 } + 8 + data_len;
    if riff_len > u32::MAX as u64 {
        return Err(Error::msg("the render is too long for a WAV file (4 GB limit)"));
    }

    let mut b = Vec::with_capacity(riff_len as usize + 8);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(riff_len as u32).to_le_bytes());
    b.extend_from_slice(b"WAVE");
    b.extend_from_slice(b"fmt ");
    b.extend_from_slice(&fmt_len.to_le_bytes());
    b.extend_from_slice(&tag.to_le_bytes());
    b.extend_from_slice(&CHANNELS.to_le_bytes());
    b.extend_from_slice(&mix.rate.to_le_bytes());
    b.extend_from_slice(&(mix.rate * block_align as u32).to_le_bytes());
    b.extend_from_slice(&block_align.to_le_bytes());
    b.extend_from_slice(&bits.to_le_bytes());
    if fmt_len == 18 {
        b.extend_from_slice(&0u16.to_le_bytes()); // cbSize
    }
    if fact {
        b.extend_from_slice(b"fact");
        b.extend_from_slice(&4u32.to_le_bytes());
        b.extend_from_slice(&(frames as u32).to_le_bytes());
    }
    b.extend_from_slice(b"data");
    b.extend_from_slice(&(data_len as u32).to_le_bytes());

    for i in 0..frames {
        for x in [mix.left[i], mix.right[i]] {
            if !x.is_finite() {
                return Err(Error::msg(format!("sample at frame {i} is not finite ({x})")));
            }
            if x.abs() > 1.0 {
                return Err(Error::msg(format!("sample at frame {i} is out of range ({x}); it would clip")));
            }
            match format {
                SampleFormat::Pcm16 => b.extend_from_slice(&((x * 32767.0).round() as i16).to_le_bytes()),
                SampleFormat::Float32 => b.extend_from_slice(&(x as f32).to_le_bytes()),
            }
        }
    }
    debug_assert_eq!(b.len() as u64, riff_len + 8);
    Ok(b)
}

/// Encode and write a WAV file. The bytes go to `<path>.tmp` first and are renamed into place,
/// so a failed write never leaves a half-written file where an older one was.
pub fn write(path: &Path, mix: &Mix, format: SampleFormat) -> Result<(), Error> {
    let bytes = encode(mix, format)?;
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = std::path::PathBuf::from(tmp);
    std::fs::write(&tmp, &bytes).map_err(|e| Error::msg(format!("cannot write {}: {e}", tmp.display())))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        Error::msg(format!("cannot move {} to {}: {e}", tmp.display(), path.display()))
    })
}
