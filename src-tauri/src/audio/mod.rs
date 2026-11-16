pub mod clock;
pub mod deck;
pub mod decoder;
pub mod device;
pub mod dsp;
pub mod telemetry;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use crossbeam_channel::{Receiver, Sender};
use tracing::{error, info};

use crate::audio::clock::AudioClock;
use crate::audio::deck::{TransportState, VirtualDeck};
use crate::audio::device::AudioDeviceManager;
use crate::audio::dsp::crossfader::{CrossfaderCurve, CrossfaderUnit};
use crate::audio::dsp::limiter::LookaheadLimiter;
use crate::audio::telemetry::{DeckTelemetry, MasterTelemetry};

#[derive(Debug, Clone)]
pub enum AudioCommand {
    LoadDeck { deck_id: usize, track_id: String, samples: Arc<Vec<f32>>, sample_rate: u32, bpm: f64 },
    Play { deck_id: usize },
    Pause { deck_id: usize },
    Stop { deck_id: usize },
    Seek { deck_id: usize, position_ms: u64 },
    SetTempoRate { deck_id: usize, rate: f64 },
    SetGain { deck_id: usize, gain: f32 },
    SetEq { deck_id: usize, low_db: f32, mid_db: f32, high_db: f32 },
    SetFilter { deck_id: usize, position: f32 },
    SetCrossfader { position: f32, curve: CrossfaderCurve },
    StartTransition { target_pos: f32, duration_ms: u32, curve: CrossfaderCurve },
    EmergencyFade { duration_ms: u32 },
    SetMasterVolume { volume: f32 },
}

pub struct AudioEngine {
    pub cmd_tx: Sender<AudioCommand>,
    telemetry_rx: Receiver<MasterTelemetry>,
    latest_telemetry: Arc<std::sync::Mutex<MasterTelemetry>>,
    pub has_device_error: Arc<AtomicBool>,
    _is_running: Arc<AtomicBool>,
    _stream: Option<Stream>,
}

// CPAL Stream contains WASAPI COM raw pointers on Windows (*mut ()).
// All real-time audio interaction happens strictly through lock-free SPSC channels.
// AudioEngine is safe to send and share between threads.
unsafe impl Send for AudioEngine {}
unsafe impl Sync for AudioEngine {}

impl AudioEngine {
    pub fn new(device_name: Option<&str>) -> Result<Self> {
        let (cmd_tx, cmd_rx) = crossbeam_channel::bounded::<AudioCommand>(256);
        let (telemetry_tx, telemetry_rx) = crossbeam_channel::bounded::<MasterTelemetry>(16);
        let is_running = Arc::new(AtomicBool::new(true));
        let has_device_error = Arc::new(AtomicBool::new(false));

        let dev_manager = AudioDeviceManager::new();
        let device = dev_manager
            .get_output_device(device_name)
            .ok_or_else(|| anyhow!("No audio output device found on system"))?;

        let supported_config = device.default_output_config()?;
        let sample_rate = supported_config.sample_rate().0;
        let channels = supported_config.channels();
        let sample_format = supported_config.sample_format();
        let config: StreamConfig = supported_config.into();

        info!("Initializing CPAL audio stream at {}Hz, {} channels, format: {:?}", sample_rate, channels, sample_format);

        // Pre-allocate mixing state
        let mut decks = vec![
            VirtualDeck::new(0, sample_rate),
            VirtualDeck::new(1, sample_rate),
            VirtualDeck::new(2, sample_rate),
            VirtualDeck::new(3, sample_rate),
        ];

        let mut crossfader = CrossfaderUnit::new();
        let mut limiter = LookaheadLimiter::new(sample_rate as f32, 1.0, -0.1);
        let mut clock = AudioClock::new(sample_rate);
        let mut master_volume: f32 = 1.0;
        let mut emergency_fade_samples_remaining: u32 = 0;
        let mut emergency_fade_total_samples: u32 = 0;

        let mut telemetry_counter = 0;
        let mut underrun_counter = 0u64;

        const SCRATCH_CHUNK_SIZE: usize = 4096;

        let err_cb_f32 = has_device_error.clone();
        let err_cb_i16 = has_device_error.clone();
        let err_cb_u16 = has_device_error.clone();

        let stream = match sample_format {
            SampleFormat::F32 => device.build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    process_audio_block(
                        data,
                        channels as usize,
                        &mut decks,
                        &mut crossfader,
                        &mut limiter,
                        &mut clock,
                        &cmd_rx,
                        &telemetry_tx,
                        &mut master_volume,
                        &mut emergency_fade_samples_remaining,
                        &mut emergency_fade_total_samples,
                        &mut telemetry_counter,
                        &mut underrun_counter,
                        sample_rate,
                    );
                },
                move |err| {
                    error!("Audio stream error / hardware disconnect: {}", err);
                    err_cb_f32.store(true, std::sync::atomic::Ordering::SeqCst);
                },
                None,
            )?,
            SampleFormat::I16 => {
                // Fixed-size scratch buffer: strictly zero heap allocations during real-time callback
                let mut float_buf = vec![0.0f32; SCRATCH_CHUNK_SIZE];
                device.build_output_stream(
                    &config,
                    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                        for chunk in data.chunks_mut(SCRATCH_CHUNK_SIZE) {
                            let chunk_len = chunk.len();
                            let slice = &mut float_buf[..chunk_len];
                            slice.fill(0.0);
                            process_audio_block(
                                slice,
                                channels as usize,
                                &mut decks,
                                &mut crossfader,
                                &mut limiter,
                                &mut clock,
                                &cmd_rx,
                                &telemetry_tx,
                                &mut master_volume,
                                &mut emergency_fade_samples_remaining,
                                &mut emergency_fade_total_samples,
                                &mut telemetry_counter,
                                &mut underrun_counter,
                                sample_rate,
                            );
                            for (i, &sample) in slice.iter().enumerate() {
                                chunk[i] = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                            }
                        }
                    },
                    move |err| {
                        error!("Audio stream error / hardware disconnect: {}", err);
                        err_cb_i16.store(true, std::sync::atomic::Ordering::SeqCst);
                    },
                    None,
                )?
            }
            SampleFormat::U16 => {
                // Fixed-size scratch buffer: strictly zero heap allocations during real-time callback
                let mut float_buf = vec![0.0f32; SCRATCH_CHUNK_SIZE];
                device.build_output_stream(
                    &config,
                    move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                        for chunk in data.chunks_mut(SCRATCH_CHUNK_SIZE) {
                            let chunk_len = chunk.len();
                            let slice = &mut float_buf[..chunk_len];
                            slice.fill(0.0);
                            process_audio_block(
                                slice,
                                channels as usize,
                                &mut decks,
                                &mut crossfader,
                                &mut limiter,
                                &mut clock,
                                &cmd_rx,
                                &telemetry_tx,
                                &mut master_volume,
                                &mut emergency_fade_samples_remaining,
                                &mut emergency_fade_total_samples,
                                &mut telemetry_counter,
                                &mut underrun_counter,
                                sample_rate,
                            );
                            for (i, &sample) in slice.iter().enumerate() {
                                chunk[i] = ((sample.clamp(-1.0, 1.0) * 0.5 + 0.5) * u16::MAX as f32) as u16;
                            }
                        }
                    },
                    move |err| {
                        error!("Audio stream error / hardware disconnect: {}", err);
                        err_cb_u16.store(true, std::sync::atomic::Ordering::SeqCst);
                    },
                    None,
                )?
            }
            _ => return Err(anyhow!("Unsupported audio sample format: {:?}", sample_format)),
        };

        stream.play()?;

        Ok(Self {
            cmd_tx,
            telemetry_rx,
            latest_telemetry: Arc::new(std::sync::Mutex::new(MasterTelemetry::empty())),
            has_device_error,
            _is_running: is_running,
            _stream: Some(stream),
        })
    }

    /// Creates a safe fallback audio engine when no hardware output device is connected.
    pub fn new_dummy() -> Self {
        let (cmd_tx, _cmd_rx) = crossbeam_channel::bounded::<AudioCommand>(256);
        let (_telemetry_tx, telemetry_rx) = crossbeam_channel::bounded::<MasterTelemetry>(16);
        Self {
            cmd_tx,
            telemetry_rx,
            latest_telemetry: Arc::new(std::sync::Mutex::new(MasterTelemetry::empty())),
            has_device_error: Arc::new(AtomicBool::new(false)),
            _is_running: Arc::new(AtomicBool::new(true)),
            _stream: None,
        }
    }

    /// Checks if a hardware disconnect or stream error was reported, and attempts hot-recovery
    pub fn check_and_recover_stream(&mut self) -> Result<bool> {
        if self.has_device_error.load(std::sync::atomic::Ordering::SeqCst) {
            self.has_device_error.store(false, std::sync::atomic::Ordering::SeqCst);
            if self._stream.is_none() {
                // Headless or dummy fallback mode: reset flag safely without invoking WASAPI handles
                return Ok(false);
            }

            tracing::warn!("Audio hardware error detected. Hot-recovering CPAL output stream to system default...");
            match AudioEngine::new(None) {
                Ok(recovered) => {
                    *self = recovered;
                    tracing::info!("Audio engine stream successfully recovered to system default hardware.");
                    Ok(true)
                }
                Err(e) => {
                    tracing::error!("Automatic stream recovery failed ({}). Maintaining dummy mode.", e);
                    *self = AudioEngine::new_dummy();
                    Ok(false)
                }
            }
        } else {
            Ok(false)
        }
    }

    pub fn send_command(&self, cmd: AudioCommand) -> Result<()> {
        self.cmd_tx.try_send(cmd).map_err(|e| anyhow!("Failed to send audio command: {}", e))
    }

    pub fn poll_telemetry(&self) -> Option<MasterTelemetry> {
        let mut latest = None;
        while let Ok(t) = self.telemetry_rx.try_recv() {
            latest = Some(t);
        }
        if let Some(ref l) = latest {
            *self.latest_telemetry.lock().unwrap() = l.clone();
            latest
        } else {
            Some(self.latest_telemetry.lock().unwrap().clone())
        }
    }

    pub fn get_deck_playhead(&self, deck_id: usize) -> Option<(u64, bool)> {
        self.poll_telemetry();
        let guard = self.latest_telemetry.lock().unwrap();
        guard.decks.iter().find(|d| d.deck_id == deck_id).map(|d| (d.position_ms, d.is_playing))
    }
}

#[inline(always)]
fn process_audio_block(
    data: &mut [f32],
    channels: usize,
    decks: &mut [VirtualDeck],
    crossfader: &mut CrossfaderUnit,
    limiter: &mut LookaheadLimiter,
    clock: &mut AudioClock,
    cmd_rx: &Receiver<AudioCommand>,
    telemetry_tx: &Sender<MasterTelemetry>,
    master_volume: &mut f32,
    fade_remaining: &mut u32,
    fade_total: &mut u32,
    telemetry_counter: &mut usize,
    underrun_counter: &mut u64,
    sample_rate: u32,
) {
    let start_time = std::time::Instant::now();
    // 1. Drain incoming commands without locking or allocating
    while let Ok(cmd) = cmd_rx.try_recv() {
        match cmd {
            AudioCommand::LoadDeck { deck_id, track_id, samples, sample_rate, bpm } => {
                if deck_id < decks.len() {
                    decks[deck_id].load_audio(track_id, samples, sample_rate, bpm);
                }
            }
            AudioCommand::Play { deck_id } => {
                if deck_id < decks.len() {
                    decks[deck_id].play();
                }
            }
            AudioCommand::Pause { deck_id } => {
                if deck_id < decks.len() {
                    decks[deck_id].pause();
                }
            }
            AudioCommand::Stop { deck_id } => {
                if deck_id < decks.len() {
                    decks[deck_id].stop();
                }
            }
            AudioCommand::Seek { deck_id, position_ms } => {
                if deck_id < decks.len() {
                    decks[deck_id].seek_to_ms(position_ms);
                }
            }
            AudioCommand::SetTempoRate { deck_id, rate } => {
                if deck_id < decks.len() {
                    decks[deck_id].set_tempo_rate(rate);
                }
            }
            AudioCommand::SetGain { deck_id, gain } => {
                if deck_id < decks.len() {
                    decks[deck_id].gain = gain.clamp(0.0, 2.0);
                }
            }
            AudioCommand::SetEq { deck_id, low_db, mid_db, high_db } => {
                if deck_id < decks.len() {
                    decks[deck_id].eq.set_gains(low_db, mid_db, high_db);
                }
            }
            AudioCommand::SetFilter { deck_id, position } => {
                if deck_id < decks.len() {
                    decks[deck_id].filter.set_position(position);
                }
            }
            AudioCommand::SetCrossfader { position, curve } => {
                crossfader.set_position(position);
                crossfader.set_curve(curve);
            }
            AudioCommand::StartTransition { target_pos, duration_ms, curve } => {
                crossfader.start_ramp(target_pos, duration_ms, sample_rate, curve);
            }
            AudioCommand::EmergencyFade { duration_ms } => {
                let frames = ((duration_ms as f64 / 1000.0) * (sample_rate as f64)).round() as u32;
                *fade_total = frames.max(1);
                *fade_remaining = *fade_total;
            }
            AudioCommand::SetMasterVolume { volume } => {
                *master_volume = volume.clamp(0.0, 1.5);
            }
        }
    }

    let frames = data.len() / channels;
    let mut peak_master_l: f32 = 0.0;
    let mut peak_master_r: f32 = 0.0;

    for frame_idx in 0..frames {
        // Continuous smooth crossfader interpolation
        crossfader.tick_frame();
        let (gain_a, gain_b) = crossfader.gains();
        let (bass_a_db, bass_b_db) = crossfader.bass_attenuations_db();

        // Apply clean Smart Bass Swap without mutating user EQ state
        decks[0].eq.set_smart_bass_attenuation(bass_a_db);
        decks[2].eq.set_smart_bass_attenuation(bass_a_db);
        decks[1].eq.set_smart_bass_attenuation(bass_b_db);
        decks[3].eq.set_smart_bass_attenuation(bass_b_db);

        // Emergency fade multiplier ramp
        let fade_mult = if *fade_remaining > 0 {
            let mult = *fade_remaining as f32 / *fade_total as f32;
            *fade_remaining -= 1;
            if *fade_remaining == 0 {
                for d in decks.iter_mut() {
                    d.pause();
                }
            }
            mult
        } else {
            1.0
        };

        // Render each deck
        let (d0_l, d0_r) = decks[0].next_frame();
        let (d1_l, d1_r) = decks[1].next_frame();
        let (d2_l, d2_r) = decks[2].next_frame();
        let (d3_l, d3_r) = decks[3].next_frame();

        // 4-Deck Crossfading:
        // Side A: Decks 0 (A) and 2 (C)
        // Side B: Decks 1 (B) and 3 (D)
        let ch_a_l = d0_l + d2_l;
        let ch_a_r = d0_r + d2_r;
        let ch_b_l = d1_l + d3_l;
        let ch_b_r = d1_r + d3_r;

        let mixed_l = (ch_a_l * gain_a + ch_b_l * gain_b) * *master_volume * fade_mult;
        let mixed_r = (ch_a_r * gain_a + ch_b_r * gain_b) * *master_volume * fade_mult;

        // Master Limiter Protection
        let (limited_l, limited_r) = limiter.process_sample(mixed_l, mixed_r);

        peak_master_l = peak_master_l.max(limited_l.abs());
        peak_master_r = peak_master_r.max(limited_r.abs());

        // Interleaved output to all available channels
        data[frame_idx * channels] = limited_l;
        if channels > 1 {
            data[frame_idx * channels + 1] = limited_r;
        }
        for ch in 2..channels {
            data[frame_idx * channels + ch] = if ch % 2 == 0 { limited_l } else { limited_r };
        }
    }

    clock.tick(frames);

    // 2. Telemetry emission throttled (~30 times a second)
    *telemetry_counter += frames;
    if *telemetry_counter >= (sample_rate as usize / 30) {
        *telemetry_counter = 0;

        let to_db = |lin: f32| -> f32 {
            if lin <= 0.00001 {
                -96.0
            } else {
                (20.0 * lin.log10()).max(-96.0)
            }
        };

        let elapsed_secs = start_time.elapsed().as_secs_f64();
        let budget_secs = frames as f64 / sample_rate as f64;
        if elapsed_secs > budget_secs {
            *underrun_counter += 1;
        }
        let cpu_load = ((elapsed_secs / budget_secs.max(0.0001)) * 100.0).clamp(0.0, 100.0) as f32;

        let deck_telemetries = vec![
            decks[0].telemetry(),
            decks[1].telemetry(),
            decks[2].telemetry(),
            decks[3].telemetry(),
        ];

        let telemetry = MasterTelemetry {
            vu_master_l_db: to_db(peak_master_l),
            vu_master_r_db: to_db(peak_master_r),
            crossfader_pos: crossfader.position(),
            underrun_count: *underrun_counter,
            cpu_load_pct: cpu_load,
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            decks: deck_telemetries,
        };

        let _ = telemetry_tx.try_send(telemetry);
    }
}
