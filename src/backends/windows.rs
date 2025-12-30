use crate::AecError;

use wasapi::{
    get_default_device, initialize_mta, AcousticEchoCancellationControl, Direction, SampleType,
    ShareMode, WaveFormat,
};

/// Create WASAPI capture backend with AEC.
/// Spawns a blocking task that owns all WASAPI resources.
/// Returns (sample_rate, buffer_size) queried from the actual device format.
pub fn create_backend(sender: flume::Sender<Vec<f32>>) -> Result<(u32, usize), AecError> {
    // COM must be initialized for WASAPI
    initialize_mta().map_err(|e| AecError::BackendError(format!("COM init failed: {e:?}")))?;

    // Verify devices are available before spawning task
    get_default_device(&Direction::Capture).map_err(|_| AecError::DeviceUnavailable)?;
    get_default_device(&Direction::Render).map_err(|_| AecError::DeviceUnavailable)?;

    let (meta_tx, meta_rx) = flume::bounded::<Result<(u32, usize), AecError>>(1);

    tokio::task::spawn_blocking(move || {
        if let Err(e) = capture_loop(sender, meta_tx.clone()) {
            let _ = meta_tx.send(Err(e));
        }
    });

    // Wait for metadata from the capture thread
    meta_rx.recv().map_err(|_| {
        AecError::BackendError("capture thread died before sending metadata".to_string())
    })?
}

fn capture_loop(
    sender: flume::Sender<Vec<f32>>,
    meta_tx: flume::Sender<Result<(u32, usize), AecError>>,
) -> Result<(), AecError> {
    // Re-initialize COM on this thread
    initialize_mta().map_err(|e| AecError::BackendError(format!("COM init failed: {e:?}")))?;

    let capture_device =
        get_default_device(&Direction::Capture).map_err(|_| AecError::DeviceUnavailable)?;
    let render_device =
        get_default_device(&Direction::Render).map_err(|_| AecError::DeviceUnavailable)?;

    let desired_format = WaveFormat::new(32, 32, &SampleType::Float, 48000, 1, None);

    let mut audio_client = capture_device
        .get_iaudioclient()
        .map_err(|e| AecError::BackendError(format!("get_iaudioclient: {e:?}")))?;

    let capture_format = match audio_client.is_supported(&desired_format, &ShareMode::Shared) {
        Ok(None) => desired_format,
        Ok(Some(suggested)) => suggested,
        Err(_) => audio_client
            .get_mixformat()
            .map_err(|e| AecError::BackendError(format!("get_mixformat: {e:?}")))?,
    };

    audio_client
        .initialize_client(
            &capture_format,
            0, // buffer duration (0 = default)
            &Direction::Capture,
            &ShareMode::Shared,
            true, // use event callback
        )
        .map_err(|e| AecError::BackendError(format!("initialize_client: {e:?}")))?;

    if let Ok(aec_control) = AcousticEchoCancellationControl::get_control(&audio_client) {
        if let Ok(render_id) = render_device.get_id() {
            let _ = aec_control.set_echo_cancellation_render_endpoint(&render_id);
        }
    }

    let capture_client = audio_client
        .get_audiocaptureclient()
        .map_err(|e| AecError::BackendError(format!("get_audiocaptureclient: {e:?}")))?;

    let event_handle = audio_client
        .set_get_eventhandle()
        .map_err(|e| AecError::BackendError(format!("set_get_eventhandle: {e:?}")))?;

    audio_client
        .start()
        .map_err(|e| AecError::BackendError(format!("start: {e:?}")))?;

    let block_align = capture_format.get_blockalign() as usize;
    let native_channels = capture_format.get_nchannels() as usize;
    let bits = capture_format.get_bitspersample();
    let is_float = capture_format.get_subformat() == Some(SampleType::Float);
    let native_sample_rate = capture_format.get_samplespersec();
    let buffer_frames = (native_sample_rate / 100) as usize; // 10ms worth of frames

    let _ = meta_tx.send(Ok((native_sample_rate, buffer_frames)));

    loop {
        event_handle
            .wait(100)
            .map_err(|e| AecError::BackendError(format!("event wait failed: {e:?}")))?;

        let (data, _frames) = capture_client
            .read_from_device_to_deveice_buffer()
            .map_err(|e| AecError::BackendError(format!("capture read failed: {e:?}")))?;

        if data.is_empty() {
            continue;
        }

        if block_align == 0 || data.len() % block_align != 0 {
            return Err(AecError::BackendError(format!(
                "misaligned audio data: {} bytes, block_align {}",
                data.len(),
                block_align
            )));
        }

        let samples = convert_to_f32(&data, bits, is_float, native_channels);
        if samples.is_empty() {
            continue;
        }

        if sender.send(samples).is_err() {
            break;
        }
    }

    audio_client
        .stop()
        .map_err(|e| AecError::BackendError(format!("stop: {e:?}")))?;

    Ok(())
}

fn convert_to_f32(data: &[u8], bits: u16, is_float: bool, channels: usize) -> Vec<f32> {
    if is_float && bits == 32 {
        return convert_f32_to_mono(data, channels);
    }
    if bits == 16 {
        return convert_i16_to_mono(data, channels);
    }
    if bits == 24 {
        return convert_i24_to_mono(data, channels);
    }
    if bits == 32 && !is_float {
        return convert_i32_to_mono(data, channels);
    }
    Vec::new()
}

fn convert_f32_to_mono(data: &[u8], channels: usize) -> Vec<f32> {
    let samples: Vec<f32> = data
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();

    if channels == 1 {
        return samples;
    }

    samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

fn convert_i16_to_mono(data: &[u8], channels: usize) -> Vec<f32> {
    let samples: Vec<f32> = data
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
        .collect();

    if channels == 1 {
        return samples;
    }

    samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

fn convert_i24_to_mono(data: &[u8], channels: usize) -> Vec<f32> {
    let samples: Vec<f32> = data
        .chunks_exact(3)
        .map(|b| {
            let val =
                i32::from_le_bytes([b[0], b[1], b[2], if b[2] & 0x80 != 0 { 0xFF } else { 0 }]);
            val as f32 / 8388608.0
        })
        .collect();

    if channels == 1 {
        return samples;
    }

    samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

fn convert_i32_to_mono(data: &[u8], channels: usize) -> Vec<f32> {
    let samples: Vec<f32> = data
        .chunks_exact(4)
        .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f32 / 2147483648.0)
        .collect();

    if channels == 1 {
        return samples;
    }

    samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}
