//! Play an `Engine` on the default sound output with cpal. The engine stays headless:
//! cpal's callback pulls frames from `Engine::render` (NFR-006, the library owns no device).

use crate::Engine;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use std::sync::{Arc, Mutex};

fn device() -> Result<cpal::Device, String> {
    cpal::default_host()
        .default_output_device()
        .ok_or_else(|| "I/O failure: no default sound output device".to_string())
}

/// Sample rate of the default output, clamped to what the engine accepts.
pub fn device_rate() -> Result<u32, String> {
    let cfg = device()?.default_output_config().map_err(|e| format!("I/O failure: {e}"))?;
    Ok(cfg.sample_rate().0.clamp(crate::engine::MIN_RATE, crate::engine::MAX_RATE))
}

/// Start playback. Keep the returned stream alive for as long as audio should play.
pub fn start(engine: Arc<Mutex<Engine>>) -> Result<cpal::Stream, String> {
    let dev = device()?;
    let supported = dev.default_output_config().map_err(|e| format!("I/O failure: {e}"))?;
    let rate = engine.lock().unwrap().rate();
    let mut cfg: cpal::StreamConfig = supported.config();
    cfg.sample_rate = cpal::SampleRate(rate);
    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => build::<f32>(&dev, &cfg, engine),
        cpal::SampleFormat::I16 => build::<i16>(&dev, &cfg, engine),
        cpal::SampleFormat::U16 => build::<u16>(&dev, &cfg, engine),
        f => return Err(format!("unsupported format: sound output sample format {f:?}")),
    }?;
    stream.play().map_err(|e| format!("I/O failure: {e}"))?;
    Ok(stream)
}

fn build<T>(dev: &cpal::Device, cfg: &cpal::StreamConfig, engine: Arc<Mutex<Engine>>) -> Result<cpal::Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = cfg.channels as usize;
    let mut scratch: Vec<i16> = Vec::new();
    dev.build_output_stream(
        cfg,
        move |out: &mut [T], _| {
            let frames = out.len() / channels;
            scratch.resize(frames * 2, 0);
            if let Ok(mut e) = engine.lock() {
                let _ = e.render(&mut scratch);
            } else {
                scratch.fill(0);
            }
            for (f, frame) in out.chunks_mut(channels).enumerate() {
                let l = scratch[f * 2] as f32 / 32768.0;
                let r = scratch[f * 2 + 1] as f32 / 32768.0;
                for (c, s) in frame.iter_mut().enumerate() {
                    let v = match (channels, c) {
                        (1, _) => (l + r) * 0.5,
                        (_, 0) => l,
                        (_, 1) => r,
                        _ => 0.0,
                    };
                    *s = T::from_sample(v);
                }
            }
        },
        |err| eprintln!("paula: audio stream error: {err}"),
        None,
    )
    .map_err(|e| format!("I/O failure: {e}"))
}
