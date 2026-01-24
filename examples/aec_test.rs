//! AEC Test Tool - Validates that OS-level Acoustic Echo Cancellation is working.
//!
//! Run with: cargo run --example aec_test
//!
//! This tool plays a 440Hz test tone through the AEC-enabled playback path while
//! recording from the microphone. If AEC is working correctly, the recording should
//! contain your voice but NOT the test tone.

use hound::{SampleFormat, WavSpec, WavWriter};
use std::f32::consts::PI;
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
    println!("1. Playing 440Hz test tone through AEC playback path");
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

    // Start streaming playback through the AEC-enabled path
    let stream = handle.start_playback_stream(SAMPLE_RATE)?;

    // Spawn thread to generate and send tone chunks
    let playback_handle = std::thread::spawn(move || {
        let chunk_size = 4800; // 100ms at 48kHz - larger chunks reduce timing sensitivity
        let total_chunks = (DURATION_SECS as usize * SAMPLE_RATE as usize) / chunk_size;
        let mut phase = 0.0f32;
        let phase_increment = TONE_FREQ / SAMPLE_RATE as f32;

        for _ in 0..total_chunks {
            let mut chunk = Vec::with_capacity(chunk_size);
            for _ in 0..chunk_size {
                chunk.push((phase * 2.0 * PI).sin() * TONE_VOLUME);
                phase = (phase + phase_increment) % 1.0;
            }
            // Channel backpressure naturally throttles - no sleep needed
            if stream.send(chunk).is_err() {
                break;
            }
        }
    });

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

    // Wait for playback thread to finish
    let _ = playback_handle.join();

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
