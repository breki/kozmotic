// Hardware-bound audio playback. Excluded from coverage
// because it requires a real output device.
//
// Test harness escape hatch: when `KOZMOTIC_TEST_AUDIO`
// is set, we skip rodio entirely and return either Ok or
// an AudioDeviceError. Tests use this to cover the
// post-playback success and error branches in
// handle_agent_ping.

use super::AgentPingError;

fn test_override() -> Option<Result<(), AgentPingError>> {
    match std::env::var("KOZMOTIC_TEST_AUDIO").as_deref() {
        Ok("ok") => Some(Ok(())),
        Ok("err") => Some(Err(AgentPingError::AudioDeviceError(
            "test override".into(),
        ))),
        _ => None,
    }
}

pub fn play_sound(
    data: &'static [u8],
    volume: f32,
    repeat: u32,
    interval: u64,
) -> Result<(), AgentPingError> {
    if let Some(r) = test_override() {
        return r;
    }
    let mut stream = rodio::OutputStreamBuilder::open_default_stream()
        .map_err(|e| AgentPingError::AudioDeviceError(e.to_string()))?;
    stream.log_on_drop(false);
    let sink = rodio::Sink::connect_new(stream.mixer());
    sink.set_volume(volume);

    for i in 0..repeat {
        let cursor = std::io::Cursor::new(data);
        let source = rodio::Decoder::new(cursor)
            .map_err(|e| AgentPingError::UnsupportedFormat(e.to_string()))?;
        sink.append(source);
        sink.sleep_until_end();
        if i + 1 < repeat {
            std::thread::sleep(std::time::Duration::from_millis(interval));
        }
    }
    Ok(())
}

pub fn play_frequency(
    freq: f32,
    duration: u64,
    volume: f32,
    repeat: u32,
    interval: u64,
) -> Result<(), AgentPingError> {
    use rodio::source::Source;

    if let Some(r) = test_override() {
        return r;
    }
    let mut stream = rodio::OutputStreamBuilder::open_default_stream()
        .map_err(|e| AgentPingError::AudioDeviceError(e.to_string()))?;
    stream.log_on_drop(false);
    let sink = rodio::Sink::connect_new(stream.mixer());
    sink.set_volume(volume);

    for i in 0..repeat {
        let source = rodio::source::SineWave::new(freq)
            .take_duration(std::time::Duration::from_millis(duration));
        sink.append(source);
        sink.sleep_until_end();
        if i + 1 < repeat {
            std::thread::sleep(std::time::Duration::from_millis(interval));
        }
    }
    Ok(())
}

pub fn play_file(
    path: &str,
    volume: f32,
    repeat: u32,
    interval: u64,
) -> Result<(), AgentPingError> {
    if let Some(r) = test_override() {
        return r;
    }
    let mut stream = rodio::OutputStreamBuilder::open_default_stream()
        .map_err(|e| AgentPingError::AudioDeviceError(e.to_string()))?;
    stream.log_on_drop(false);
    let sink = rodio::Sink::connect_new(stream.mixer());
    sink.set_volume(volume);

    for i in 0..repeat {
        let file = std::fs::File::open(path)
            .map_err(|_| AgentPingError::FileNotFound(path.to_string()))?;
        let reader = std::io::BufReader::new(file);
        let source = rodio::Decoder::new(reader)
            .map_err(|e| AgentPingError::UnsupportedFormat(e.to_string()))?;
        sink.append(source);
        sink.sleep_until_end();
        if i + 1 < repeat {
            std::thread::sleep(std::time::Duration::from_millis(interval));
        }
    }
    Ok(())
}
