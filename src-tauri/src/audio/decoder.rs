use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use anyhow::{anyhow, Result};
use symphonia::core::audio::{AudioBufferRef, SampleBuffer, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub const MAX_DECODED_AUDIO_MINUTES: u64 = 30;

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

    let dec_opts: DecoderOptions = Default::default();
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &dec_opts)
        .map_err(|e| anyhow!("Failed to instantiate codec: {}", e))?;

    let mut interleaved = Vec::with_capacity(sample_rate as usize * 60 * 2); // Initial pre-allocation for ~1 min stereo
    let max_samples = (sample_rate as usize * 60 * MAX_DECODED_AUDIO_MINUTES as usize) * 2;
    let mut sample_buf: Option<SampleBuffer<f32>> = None;
    let mut clean_eof = false;
    let mut consecutive_decode_errors = 0;
    let mut fatal_error: Option<String> = None;

    loop {
        if interleaved.len() >= max_samples {
            tracing::info!(
                "Reached maximum decoded audio memory budget ({} minutes); capping memory allocation.",
                MAX_DECODED_AUDIO_MINUTES
            );
            clean_eof = true;
            break;
        }

        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                clean_eof = true;
                break; // Clean EOF
            }
            Err(SymphoniaError::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(e) => {
                fatal_error = Some(format!("Fatal packet read issue: {}", e));
                break;
            }
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                consecutive_decode_errors = 0;
                append_audio_buffer(&decoded, &mut sample_buf, &mut interleaved);
            }
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                clean_eof = true;
                break;
            }
            Err(SymphoniaError::DecodeError(e)) => {
                consecutive_decode_errors += 1;
                tracing::debug!("Decode frame skip ({}): {}", consecutive_decode_errors, e);
                // Allow isolated frame corruption, but abort on sustained stream corruption
                if consecutive_decode_errors > 20 {
                    fatal_error = Some(format!("Exceeded tolerable consecutive decode errors: {}", e));
                    break;
                }
                continue;
            }
            Err(e) => {
                fatal_error = Some(format!("Unrecoverable decoder error: {}", e));
                break;
            }
        }
    }

    // Fail if zero decodable samples were produced
    if interleaved.is_empty() {
        return Err(anyhow!(
            "Failed to decode audio: stream contains zero valid audio frames ({})",
            fatal_error.unwrap_or_else(|| "empty payload".to_string())
        ));
    }

    // If an unrecoverable read or decode error caused loop termination before normal EOF,
    // reject the file to prevent corrupted/truncated tracks from entering the performance queue
    if let Some(err) = fatal_error {
        if !clean_eof {
            return Err(anyhow!("Corrupt audio file rejected due to premature termination: {}", err));
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

/// Robust conversion of any Symphonia sample format (U8, S16, S24, S32, F32, F64)
/// with universal calibrated ITU-R BS.775 downmixing/upmixing into standardized stereo [L, R]
fn append_audio_buffer(
    buf_ref: &AudioBufferRef,
    sample_buf: &mut Option<SampleBuffer<f32>>,
    out: &mut Vec<f32>,
) {
    let num_channels = buf_ref.spec().channels.count();
    let num_frames = buf_ref.frames();
    if num_frames == 0 || num_channels == 0 {
        return;
    }

    // Reuse or create the SampleBuffer<f32> converter
    if sample_buf.is_none() || sample_buf.as_ref().map(|b| b.capacity() < num_frames * num_channels).unwrap_or(false) {
        *sample_buf = Some(SampleBuffer::<f32>::new(buf_ref.capacity() as u64, *buf_ref.spec()));
    }

    let sbuf = sample_buf.as_mut().unwrap();
    sbuf.copy_interleaved_ref(buf_ref.clone());
    let raw_samples = sbuf.samples();

    match num_channels {
        1 => {
            // Mono to stereo duplicate [M, M]
            out.reserve(num_frames * 2);
            for &sample in raw_samples.iter().take(num_frames) {
                out.push(sample);
                out.push(sample);
            }
        }
        2 => {
            // Direct stereo [L, R]
            let total_samples = num_frames * 2;
            out.extend_from_slice(&raw_samples[..total_samples.min(raw_samples.len())]);
        }
        ch => {
            // Calibrated ITU-R BS.775 multi-channel downmix to stereo (e.g. 5.1 surround)
            // L_out = (L + 0.7071 * Center + 0.7071 * Ls + 0.50 * LFE) * Norm
            // R_out = (R + 0.7071 * Center + 0.7071 * Rs + 0.50 * LFE) * Norm
            out.reserve(num_frames * 2);
            let center_weight = 0.7071f32;
            let surround_weight = 0.7071f32;

            for frame_idx in 0..num_frames {
                let base = frame_idx * ch;
                if base + 1 >= raw_samples.len() {
                    break;
                }
                let left = raw_samples[base];
                let right = raw_samples[base + 1];

                let (mix_l, mix_r) = if ch >= 6 {
                    // Standard 5.1 surround: 0=Left, 1=Right, 2=Center, 3=LFE (subwoofer), 4=Left Surround, 5=Right Surround
                    let center = raw_samples.get(base + 2).copied().unwrap_or(0.0) * center_weight;
                    let lfe = raw_samples.get(base + 3).copied().unwrap_or(0.0) * 0.50f32;
                    let ls = raw_samples.get(base + 4).copied().unwrap_or(0.0) * surround_weight;
                    let rs = raw_samples.get(base + 5).copied().unwrap_or(0.0) * surround_weight;
                    let norm = 0.41f32; // Calibrated normalization preventing inter-sample peaks
                    ((left + center + ls + lfe) * norm, (right + center + rs + lfe) * norm)
                } else if ch >= 3 {
                    // 3.0 / quadraphonic: 0=L, 1=R, 2=Center/Rear
                    let extra = raw_samples.get(base + 2).copied().unwrap_or(0.0) * 0.7071f32;
                    let norm = 0.58f32;
                    ((left + extra) * norm, (right + extra) * norm)
                } else {
                    (left, right)
                };

                out.push(mix_l.clamp(-1.0, 1.0));
                out.push(mix_r.clamp(-1.0, 1.0));
            }
        }
    }
}

