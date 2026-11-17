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
    let declared_n_frames = track.codec_params.n_frames;

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
            return Err(anyhow!(
                "File exceeds maximum supported in-memory duration of {} minutes; streaming architecture required",
                MAX_DECODED_AUDIO_MINUTES
            ));
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

    if let Some(err) = fatal_error {
        return Err(anyhow!("Corrupt audio file rejected due to premature termination: {}", err));
    }

    if !clean_eof {
        return Err(anyhow!("Corrupt audio file: stream ended prematurely without clean end-of-file indicator"));
    }

    let total_frames = interleaved.len() / 2;
    // Validate decoded frame count against declared container frame count (with 15% tolerance for VBR container headers)
    if let Some(declared_frames) = declared_n_frames {
        if declared_frames > 0 && total_frames < (declared_frames as usize * 85 / 100) {
            return Err(anyhow!(
                "Premature stream truncation: decoded {} frames, but container declared {} frames",
                total_frames, declared_frames
            ));
        }
    }

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
        3 => {
            // 3.0 Layout: 0=Left, 1=Right, 2=Center
            out.reserve(num_frames * 2);
            let norm = 0.5857f32; // 1.0 / (1.0 + 0.7071)
            for frame_idx in 0..num_frames {
                let base = frame_idx * 3;
                let left = raw_samples[base];
                let right = raw_samples[base + 1];
                let center = raw_samples.get(base + 2).copied().unwrap_or(0.0) * 0.7071f32;
                out.push(((left + center) * norm).clamp(-1.0, 1.0));
                out.push(((right + center) * norm).clamp(-1.0, 1.0));
            }
        }
        4 => {
            // Quadraphonic 4.0 Layout: 0=Left, 1=Right, 2=Left Surround, 3=Right Surround
            out.reserve(num_frames * 2);
            let norm = 0.5857f32;
            for frame_idx in 0..num_frames {
                let base = frame_idx * 4;
                let left = raw_samples[base];
                let right = raw_samples[base + 1];
                let ls = raw_samples.get(base + 2).copied().unwrap_or(0.0) * 0.7071f32;
                let rs = raw_samples.get(base + 3).copied().unwrap_or(0.0) * 0.7071f32;
                out.push(((left + ls) * norm).clamp(-1.0, 1.0));
                out.push(((right + rs) * norm).clamp(-1.0, 1.0));
            }
        }
        5 | 6 => {
            // 5.1 Surround (ITU-R BS.775): 0=Left, 1=Right, 2=Center, 3=LFE, 4=Left Surround, 5=Right Surround
            out.reserve(num_frames * 2);
            let norm = 0.41f32; // Normalization prevents digital clipping
            for frame_idx in 0..num_frames {
                let base = frame_idx * num_channels;
                let left = raw_samples[base];
                let right = raw_samples[base + 1];
                let center = raw_samples.get(base + 2).copied().unwrap_or(0.0) * 0.7071f32;
                let lfe = raw_samples.get(base + 3).copied().unwrap_or(0.0) * 0.50f32;
                let ls = raw_samples.get(base + 4).copied().unwrap_or(0.0) * 0.7071f32;
                let rs = raw_samples.get(base + 5).copied().unwrap_or(0.0) * 0.7071f32;
                out.push(((left + center + ls + lfe) * norm).clamp(-1.0, 1.0));
                out.push(((right + center + rs + lfe) * norm).clamp(-1.0, 1.0));
            }
        }
        ch => {
            // 7.1+ Surround: 0=Left, 1=Right, 2=Center, 3=LFE, 4=Ls, 5=Rs, 6=Lb, 7=Rb
            out.reserve(num_frames * 2);
            let norm = 0.35f32;
            for frame_idx in 0..num_frames {
                let base = frame_idx * ch;
                let left = raw_samples[base];
                let right = raw_samples[base + 1];
                let center = raw_samples.get(base + 2).copied().unwrap_or(0.0) * 0.7071f32;
                let lfe = raw_samples.get(base + 3).copied().unwrap_or(0.0) * 0.50f32;
                let ls = raw_samples.get(base + 4).copied().unwrap_or(0.0) * 0.7071f32;
                let rs = raw_samples.get(base + 5).copied().unwrap_or(0.0) * 0.7071f32;
                let lb = raw_samples.get(base + 6).copied().unwrap_or(0.0) * 0.50f32;
                let rb = raw_samples.get(base + 7).copied().unwrap_or(0.0) * 0.50f32;
                out.push(((left + center + ls + lb + lfe) * norm).clamp(-1.0, 1.0));
                out.push(((right + center + rs + rb + lfe) * norm).clamp(-1.0, 1.0));
            }
        }
    }
}

