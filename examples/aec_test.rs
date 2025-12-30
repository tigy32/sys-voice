//! AEC Test Tool - Validates that OS-level Acoustic Echo Cancellation is working.
//!
//! Run with: cargo run --example aec_test
//!
//! This tool plays a 440Hz test tone through speakers while recording from the
//! microphone with AEC enabled. If AEC is working correctly, the recording should
//! contain your voice but NOT the test tone.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::f32::consts::PI;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use sys_voice::{AecConfig, CaptureHandle, Channels};

const SAMPLE_RATE: u32 = 48000;
const DURATION_SECS: u64 = 10;
const TONE_FREQ: f32 = 440.0;
const TONE_VOLUME: f32 = 0.3;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("AEC Test Tool");
    println!("=============");
    println!();
    println!("1. Playing 440Hz test tone through speakers");
    println!(
        "2. Recording with AEC enabled for {} seconds",
        DURATION_SECS
    );
    println!("3. Speak into the mic while the tone plays");
    println!();
    println!("Expected result:");
    println!("- Your voice should be clearly audible in the recording");
    println!("- The 440Hz tone should be ABSENT or significantly reduced");
    println!();
    println!("Recording to: aec_recording.wav");
    println!();

    let config = AecConfig {
        sample_rate: SAMPLE_RATE,
        channels: Channels::Mono,
    };

    let handle = CaptureHandle::new(config)?;
    let mut recorded_samples: Vec<f32> = Vec::new();

    let host = cpal::default_host();
    let output_device = host
        .default_output_device()
        .ok_or("No output device available")?;

    let output_config = output_device.default_output_config()?;
    let output_sample_rate = output_config.sample_rate().0 as f32;
    let output_channels = output_config.channels() as usize;

    let phase_increment = TONE_FREQ / output_sample_rate;
    let phase = Arc::new(Mutex::new(0.0f32));
    let phase_clone = phase.clone();

    let output_stream = output_device.build_output_stream(
        &output_config.into(),
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let Ok(mut current_phase) = phase_clone.lock() else {
                return;
            };
            for frame in data.chunks_mut(output_channels) {
                let sample = (*current_phase * 2.0 * PI).sin() * TONE_VOLUME;
                *current_phase = (*current_phase + phase_increment) % 1.0;
                for channel_sample in frame.iter_mut() {
                    *channel_sample = sample;
                }
            }
        },
        |err| eprintln!("Output stream error: {err:?}"),
        None,
    )?;

    output_stream.play()?;

    println!("Recording... speak now!");
    println!();

    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(DURATION_SECS) {
        while let Some(result) = handle.try_recv() {
            match result {
                Ok(samples) => recorded_samples.extend_from_slice(&samples),
                Err(e) => eprintln!("Audio error: {e}"),
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    // Drain remaining samples
    while let Some(result) = handle.try_recv() {
        match result {
            Ok(samples) => recorded_samples.extend_from_slice(&samples),
            Err(e) => eprintln!("Audio error: {e}"),
        }
    }

    drop(output_stream);

    println!("Recording complete!");
    println!();

    drop(handle);

    let samples = &recorded_samples;
    let spec = WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };

    let mut writer = WavWriter::create("aec_recording.wav", spec)?;
    for sample in samples.iter() {
        writer.write_sample(*sample)?;
    }
    writer.finalize()?;

    println!(
        "Saved: aec_recording.wav ({} samples, {:.1} seconds)",
        samples.len(),
        samples.len() as f32 / SAMPLE_RATE as f32
    );
    println!();
    println!("To verify AEC is working:");
    println!("- Play aec_recording.wav");
    println!("- You should hear your voice but NOT the 440Hz test tone");
    println!("- If you hear the tone clearly, AEC may not be active on your system");

    Ok(())
}
