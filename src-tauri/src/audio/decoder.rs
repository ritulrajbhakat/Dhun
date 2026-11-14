use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use anyhow::{anyhow, Result};
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

#[derive(Debug, Clone)]
pub struct DecodedAudio {
    pub samples: Arc<Vec<f32>>, // Interleaved stereo [L, R, L, R, ...]
    pub sample_rate: u32,
    pub channels: u32,
    pub duration_ms: u64,
}

pub fn decode_audio_file<P: AsRef<Path>>(path: P) -> Result<DecodedAudio> {
    let path_ref = path.as_ref();
    let file = File::open(path_ref).map_err(|e| anyhow!("Failed to open audio file {:?}: {}", path_ref, e))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path_ref.extension().and_then(|s| s.to_str()) {
        hint.with_extension(ext);
    }

    let format_opts = FormatOptions {
        enable_gapless: true,
        ..Default::default()
    };
    let meta_opts: MetadataOptions = Default::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &meta_opts)
        .map_err(|e| anyhow!("Failed to probe audio file: {}", e))?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow!("No supported audio track found"))?;

    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let _channels = track.codec_params.channels.map(|c| c.count() as u32).unwrap_or(2);

    let dec_opts: DecoderOptions = Default::default();
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &dec_opts)
        .map_err(|e| anyhow!("Failed to instantiate codec: {}", e))?;

    let mut interleaved = Vec::with_capacity(sample_rate as usize * 60 * 2); // Initial pre-allocation for ~1 min stereo

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break; // Normal EOF
            }
            Err(SymphoniaError::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(e) => {
                tracing::warn!("Non-fatal packet read issue: {}", e);
                break;
            }
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                append_audio_buffer(&decoded, &mut interleaved);
            }
            Err(SymphoniaError::IoError(_)) => break,
            Err(SymphoniaError::DecodeError(e)) => {
                tracing::debug!("Decode frame skip: {}", e);
                continue;
            }
            Err(e) => {
                tracing::warn!("Decoder error: {}", e);
                break;
            }
        }
    }

    let total_frames = interleaved.len() / 2;
    let duration_ms = ((total_frames as f64 / sample_rate as f64) * 1000.0).round() as u64;

    Ok(DecodedAudio {
        samples: Arc::new(interleaved),
        sample_rate,
        channels: 2, // Always normalized to stereo
        duration_ms,
    })
}

fn append_audio_buffer(buf_ref: &AudioBufferRef, out: &mut Vec<f32>) {
    match buf_ref {
        AudioBufferRef::F32(buf) => {
            let spec = buf.spec();
            let frames = buf.frames();
            if spec.channels.count() >= 2 {
                for i in 0..frames {
                    out.push(buf.chan(0)[i]);
                    out.push(buf.chan(1)[i]);
                }
            } else if spec.channels.count() == 1 {
                // Mono to stereo upmix
                for i in 0..frames {
                    let s = buf.chan(0)[i];
                    out.push(s);
                    out.push(s);
                }
            }
        }
        AudioBufferRef::S16(buf) => {
            let spec = buf.spec();
            let frames = buf.frames();
            let norm = 1.0 / 32768.0;
            if spec.channels.count() >= 2 {
                for i in 0..frames {
                    out.push(buf.chan(0)[i] as f32 * norm);
                    out.push(buf.chan(1)[i] as f32 * norm);
                }
            } else if spec.channels.count() == 1 {
                for i in 0..frames {
                    let s = buf.chan(0)[i] as f32 * norm;
                    out.push(s);
                    out.push(s);
                }
            }
        }
        AudioBufferRef::U8(buf) => {
            let spec = buf.spec();
            let frames = buf.frames();
            if spec.channels.count() >= 2 {
                for i in 0..frames {
                    out.push((buf.chan(0)[i] as f32 - 128.0) / 128.0);
                    out.push((buf.chan(1)[i] as f32 - 128.0) / 128.0);
                }
            } else if spec.channels.count() == 1 {
                for i in 0..frames {
                    let s = (buf.chan(0)[i] as f32 - 128.0) / 128.0;
                    out.push(s);
                    out.push(s);
                }
            }
        }
        _ => {
            // Other formats (e.g. S32, S24)
            tracing::warn!("Encountered unhandled buffer format, skipped frame chunk");
        }
    }
}

