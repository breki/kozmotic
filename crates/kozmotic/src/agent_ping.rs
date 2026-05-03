use std::process::ExitCode;

use crate::output::{Output, OutputFormat};

const MUTE_FILE: &str = ".mute-sounds";

fn mute_file_path() -> Option<std::path::PathBuf> {
    crate::self_install::home_dir().map(|h| h.join(".claude").join(MUTE_FILE))
}

fn is_muted() -> bool {
    mute_file_path().is_some_and(|p| p.exists())
}

const SOUND_STOP_CHIME: &[u8] = include_bytes!("../assets/sounds/stop-chime.mp3");
const SOUND_NOTIFICATION_CHIME: &[u8] = include_bytes!("../assets/sounds/notification-chime.mp3");
const SOUND_ERROR: &[u8] = include_bytes!("../assets/sounds/error.mp3");

const PRESET_NAMES: &[&str] = &["Stop", "StopFailure", "Notification"];

fn get_preset(name: &str) -> Option<&'static [u8]> {
    match name.to_lowercase().as_str() {
        "stop" => Some(SOUND_STOP_CHIME),
        "stopfailure" => Some(SOUND_ERROR),
        "notification" => Some(SOUND_NOTIFICATION_CHIME),
        _ => None,
    }
}

#[derive(Debug, thiserror::Error)]
enum AgentPingError {
    #[error(
        "no sound source specified; \
         use --sound, --frequency, or --file"
    )]
    MissingSoundSource,
    #[error("unknown preset: {0}")]
    UnknownPreset(String),
    #[error("frequency must be between 20 and 20000 Hz, got {0}")]
    InvalidFrequency(f32),
    #[error("volume must be between 0.0 and 1.0, got {0}")]
    InvalidVolume(f32),
    #[error("file not found: {0}")]
    FileNotFound(String),
    #[error("unsupported audio format: {0}")]
    UnsupportedFormat(String),
    #[error("audio device error: {0}")]
    AudioDeviceError(String),
}

impl AgentPingError {
    fn code(&self) -> &'static str {
        match self {
            AgentPingError::MissingSoundSource => "MISSING_SOUND_SOURCE",
            AgentPingError::UnknownPreset(_) => "UNKNOWN_PRESET",
            AgentPingError::InvalidFrequency(_) => "INVALID_FREQUENCY",
            AgentPingError::InvalidVolume(_) => "INVALID_VOLUME",
            AgentPingError::FileNotFound(_) => "FILE_NOT_FOUND",
            AgentPingError::UnsupportedFormat(_) => "UNSUPPORTED_FORMAT",
            AgentPingError::AudioDeviceError(_) => "AUDIO_DEVICE_ERROR",
        }
    }

    fn exit_code(&self) -> u8 {
        match self {
            AgentPingError::AudioDeviceError(_) => 2,
            _ => 1,
        }
    }
}

fn emit_error(format: &OutputFormat, err: &AgentPingError) -> ExitCode {
    match format {
        OutputFormat::Json => {
            let output = Output::error("agent-ping", err.code(), &err.to_string());
            eprintln!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Human => {
            eprintln!("Error [{}]: {}", err.code(), err);
        }
    }
    ExitCode::from(err.exit_code())
}

fn play_sound(
    data: &'static [u8],
    volume: f32,
    repeat: u32,
    interval: u64,
) -> Result<(), AgentPingError> {
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

fn play_frequency(
    freq: f32,
    duration: u64,
    volume: f32,
    repeat: u32,
    interval: u64,
) -> Result<(), AgentPingError> {
    use rodio::source::Source;

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

fn play_file(path: &str, volume: f32, repeat: u32, interval: u64) -> Result<(), AgentPingError> {
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

pub struct AgentPingArgs {
    pub sound: Option<String>,
    pub file: Option<String>,
    pub frequency: Option<f32>,
    pub duration: u64,
    pub volume: f32,
    pub repeat: u32,
    pub interval: u64,
    pub list: bool,
    pub dry_run: bool,
}

pub fn handle_agent_ping(format: &OutputFormat, args: AgentPingArgs) -> ExitCode {
    let AgentPingArgs {
        sound,
        file,
        frequency,
        duration,
        volume,
        repeat,
        interval,
        list,
        dry_run,
    } = args;

    // --list: output preset names
    if list {
        let presets: Vec<&str> = PRESET_NAMES.to_vec();
        match format {
            OutputFormat::Json => {
                let data = serde_json::json!({
                    "presets": presets,
                });
                let output = Output::success("agent-ping", data);
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            }
            OutputFormat::Human => {
                println!("Available presets:");
                for name in &presets {
                    println!("  {name}");
                }
            }
        }
        return ExitCode::SUCCESS;
    }

    // Check mute file — skip playback silently
    if is_muted() {
        match format {
            OutputFormat::Json => {
                let data = serde_json::json!({
                    "sound": sound.as_deref()
                        .or(file.as_deref())
                        .unwrap_or("unknown"),
                    "played": false,
                    "muted": true,
                });
                let output = Output::success("agent-ping", data);
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            }
            OutputFormat::Human => {
                println!("[muted] Sounds are muted");
            }
        }
        return ExitCode::SUCCESS;
    }

    // Validate: need at least one source
    if sound.is_none() && file.is_none() && frequency.is_none() {
        return emit_error(format, &AgentPingError::MissingSoundSource);
    }

    // Validate volume
    if !(0.0..=1.0).contains(&volume) {
        return emit_error(format, &AgentPingError::InvalidVolume(volume));
    }

    // Validate frequency
    if let Some(freq) = frequency
        && !(20.0..=20000.0).contains(&freq)
    {
        return emit_error(format, &AgentPingError::InvalidFrequency(freq));
    }

    // Validate preset name
    if let Some(ref name) = sound
        && get_preset(name).is_none()
    {
        return emit_error(format, &AgentPingError::UnknownPreset(name.clone()));
    }

    // Validate file exists
    if let Some(ref path) = file
        && !std::path::Path::new(path).exists()
    {
        return emit_error(format, &AgentPingError::FileNotFound(path.clone()));
    }

    // Build description for output
    let source_label = if let Some(ref name) = sound {
        name.clone()
    } else if let Some(freq) = frequency {
        format!("{freq} Hz tone")
    } else if let Some(ref path) = file {
        path.clone()
    } else {
        unreachable!()
    };

    // --dry-run
    if dry_run {
        match format {
            OutputFormat::Json => {
                let mut details = serde_json::json!({
                    "volume": volume,
                    "repeat": repeat,
                });
                if let Some(ref name) = sound {
                    details["sound"] = name.clone().into();
                }
                if let Some(freq) = frequency {
                    details["frequency"] = freq.into();
                    details["duration_ms"] = duration.into();
                }
                if let Some(ref path) = file {
                    details["file"] = path.clone().into();
                }
                let data = serde_json::json!({
                    "sound": source_label,
                    "played": false,
                    "details": details,
                });
                let output = Output::success("agent-ping", data);
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            }
            OutputFormat::Human => {
                println!("[dry-run] Would play: {source_label}");
            }
        }
        return ExitCode::SUCCESS;
    }

    // Play sound
    let play_result = if let Some(ref name) = sound {
        let data = get_preset(name).unwrap();
        play_sound(data, volume, repeat, interval)
    } else if let Some(freq) = frequency {
        play_frequency(freq, duration, volume, repeat, interval)
    } else if let Some(ref path) = file {
        play_file(path, volume, repeat, interval)
    } else {
        unreachable!()
    };

    if let Err(e) = play_result {
        return emit_error(format, &e);
    }

    // Output success
    match format {
        OutputFormat::Json => {
            let mut details = serde_json::json!({
                "volume": volume,
                "repeat": repeat,
            });
            if let Some(ref name) = sound {
                details["sound"] = name.clone().into();
            }
            if let Some(freq) = frequency {
                details["frequency"] = freq.into();
                details["duration_ms"] = duration.into();
            }
            if let Some(ref path) = file {
                details["file"] = path.clone().into();
            }
            let data = serde_json::json!({
                "sound": source_label,
                "played": true,
                "details": details,
            });
            let output = Output::success("agent-ping", data);
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Human => {
            println!("Played: {source_label}");
        }
    }
    ExitCode::SUCCESS
}
