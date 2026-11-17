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
                fatal_error = Some(format!("Decoder payload truncated mid-frame: {}", e));
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

    // 1. Enforce minimum decodable audio duration (at least 1.0 second) to reject truncated snippets or scraps
    if total_frames < sample_rate as usize {
        return Err(anyhow!(
            "Corrupt or truncated audio file: contains less than 1.0 second of audio ({} frames at {} Hz)",
            total_frames, sample_rate
        ));
    }

    // 2. Validate decoded frame count against declared container frame count (with 15% tolerance for VBR container headers)
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
/// with universal calibrated ITU-R BS.775 layout-aware downmixing/upmixing into standardized stereo [L, R]
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

    if num_channels == 1 {
        // Mono to stereo duplicate [M, M]
        out.reserve(num_frames * 2);
        for &sample in raw_samples.iter().take(num_frames) {
            out.push(sample);
            out.push(sample);
        }
        return;
    }

    let channels = buf_ref.spec().channels;

    if num_channels == 2
        && channels.contains(symphonia::core::audio::Channels::FRONT_LEFT)
        && channels.contains(symphonia::core::audio::Channels::FRONT_RIGHT)
    {
        // Direct stereo [L, R]
        let total_samples = num_frames * 2;
        out.extend_from_slice(&raw_samples[..total_samples.min(raw_samples.len())]);
        return;
    }

    // Layout-aware channel mapping: inspect the declared Channels bitflags to find exact channel positions
    let mut left_idx = None;
    let mut right_idx = None;
    let mut center_idx = None;
    let mut lfe_idx = None;
    let mut surround_left_idx = None;
    let mut surround_right_idx = None;
    let mut rear_left_idx = None;
    let mut rear_right_idx = None;

    for (idx, ch) in channels.iter().enumerate() {
        if ch.contains(symphonia::core::audio::Channels::FRONT_LEFT) {
            left_idx = Some(idx);
        } else if ch.contains(symphonia::core::audio::Channels::FRONT_RIGHT) {
            right_idx = Some(idx);
        } else if ch.contains(symphonia::core::audio::Channels::FRONT_CENTRE) {
            center_idx = Some(idx);
        } else if ch.contains(symphonia::core::audio::Channels::LFE1) {
            lfe_idx = Some(idx);
        } else if ch.contains(symphonia::core::audio::Channels::SIDE_LEFT) {
            surround_left_idx = Some(idx);
        } else if ch.contains(symphonia::core::audio::Channels::SIDE_RIGHT) {
            surround_right_idx = Some(idx);
        } else if ch.contains(symphonia::core::audio::Channels::REAR_LEFT) {
            if surround_left_idx.is_none() {
                surround_left_idx = Some(idx);
            } else {
                rear_left_idx = Some(idx);
            }
        } else if ch.contains(symphonia::core::audio::Channels::REAR_RIGHT) {
            if surround_right_idx.is_none() {
                surround_right_idx = Some(idx);
            } else {
                rear_right_idx = Some(idx);
            }
        } else if ch.contains(symphonia::core::audio::Channels::REAR_CENTRE) {
            if center_idx.is_none() {
                center_idx = Some(idx);
            }
        }
    }

    let l_i = left_idx.unwrap_or(0);
    let r_i = right_idx.unwrap_or(1.min(num_channels - 1));

    // Dynamic normalization factor based on present channel components to prevent digital clipping
    let mut max_sum = 1.0f32;
    if center_idx.is_some() { max_sum += 0.7071; }
    if surround_left_idx.is_some() || surround_right_idx.is_some() { max_sum += 0.7071; }
    if rear_left_idx.is_some() || rear_right_idx.is_some() { max_sum += 0.5000; }
    if lfe_idx.is_some() { max_sum += 0.5000; }
    let norm = 1.0f32 / max_sum;

    out.reserve(num_frames * 2);
    for frame_idx in 0..num_frames {
        let base = frame_idx * num_channels;
        let l_val = raw_samples.get(base + l_i).copied().unwrap_or(0.0);
        let r_val = raw_samples.get(base + r_i).copied().unwrap_or(0.0);
        let c_val = center_idx.and_then(|idx| raw_samples.get(base + idx)).copied().unwrap_or(0.0) * 0.7071;
        let lfe_val = lfe_idx.and_then(|idx| raw_samples.get(base + idx)).copied().unwrap_or(0.0) * 0.5000;
        let ls_val = surround_left_idx.and_then(|idx| raw_samples.get(base + idx)).copied().unwrap_or(0.0) * 0.7071;
        let rs_val = surround_right_idx.and_then(|idx| raw_samples.get(base + idx)).copied().unwrap_or(0.0) * 0.7071;
        let lb_val = rear_left_idx.and_then(|idx| raw_samples.get(base + idx)).copied().unwrap_or(0.0) * 0.5000;
        let rb_val = rear_right_idx.and_then(|idx| raw_samples.get(base + idx)).copied().unwrap_or(0.0) * 0.5000;

        let mixed_l = ((l_val + c_val + ls_val + lb_val + lfe_val) * norm).clamp(-1.0, 1.0);
        let mixed_r = ((r_val + c_val + rs_val + rb_val + lfe_val) * norm).clamp(-1.0, 1.0);

        out.push(mixed_l);
        out.push(mixed_r);
    }
}

