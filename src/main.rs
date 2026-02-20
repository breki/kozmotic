use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::process::ExitCode;

const SOUND_BEEP: &[u8] = include_bytes!("../assets/sounds/beep.ogg");
const SOUND_MESSAGE_SENT: &[u8] = include_bytes!("../assets/sounds/message-sent.ogg");
const SOUND_MESSAGE: &[u8] = include_bytes!("../assets/sounds/message.ogg");

const PRESET_NAMES: &[&str] = &[
    "PostToolUse",
    "Stop",
    "SubagentStop",
    "TaskCompleted",
    "Notification",
];

fn get_preset(name: &str) -> Option<&'static [u8]> {
    match name.to_lowercase().as_str() {
        "posttooluse" => Some(SOUND_BEEP),
        "stop" => Some(SOUND_MESSAGE_SENT),
        "subagentstop" => Some(SOUND_MESSAGE_SENT),
        "taskcompleted" => Some(SOUND_MESSAGE_SENT),
        "notification" => Some(SOUND_MESSAGE),
        _ => None,
    }
}

#[derive(Debug, thiserror::Error)]
enum AgentPingError {
    #[error("no sound source specified; use --sound, --frequency, or --file")]
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

#[derive(Parser)]
#[command(name = "kozmotic")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Output format (json or human)
    #[arg(short, long, value_name = "FORMAT", default_value = "json")]
    format: OutputFormat,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Clone, Debug)]
enum OutputFormat {
    Json,
    Human,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "human" => Ok(OutputFormat::Human),
            _ => Err(format!("Invalid format: {}. Use 'json' or 'human'", s)),
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Example command - will be replaced with actual tools
    Example {
        /// Example argument
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Play a notification sound (for hooks and alerts)
    #[command(name = "agent-ping")]
    AgentPing {
        /// Play a built-in preset (hook event name)
        #[arg(long, group = "source")]
        sound: Option<String>,

        /// Play a custom audio file
        #[arg(long, group = "source")]
        file: Option<String>,

        /// Play a generated tone at frequency (Hz)
        #[arg(long, group = "source")]
        frequency: Option<f32>,

        /// Tone duration in ms (--frequency only)
        #[arg(long, default_value = "200")]
        duration: u64,

        /// Volume 0.0–1.0
        #[arg(long, default_value = "0.5")]
        volume: f32,

        /// Play N times
        #[arg(long, default_value = "1")]
        repeat: u32,

        /// Gap between repeats in ms
        #[arg(long, default_value = "100")]
        interval: u64,

        /// List available presets
        #[arg(long)]
        list: bool,

        /// Report what would play, no sound
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Serialize, Deserialize)]
struct Output<T> {
    status: String,
    data: T,
    metadata: Metadata,
}

#[derive(Serialize, Deserialize)]
struct Metadata {
    timestamp: String,
    tool: String,
    version: String,
}

impl<T> Output<T> {
    fn success(tool: &str, data: T) -> Self {
        Self {
            status: "success".to_string(),
            data,
            metadata: Metadata {
                timestamp: chrono::Utc::now().to_rfc3339(),
                tool: tool.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }
}

impl Output<serde_json::Value> {
    fn error(tool: &str, code: &str, message: &str) -> Self {
        Self {
            status: "error".to_string(),
            data: serde_json::json!({
                "code": code,
                "message": message,
            }),
            metadata: Metadata {
                timestamp: chrono::Utc::now().to_rfc3339(),
                tool: tool.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
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
    let stream = rodio::OutputStreamBuilder::open_default_stream()
        .map_err(|e| AgentPingError::AudioDeviceError(e.to_string()))?;
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

    let stream = rodio::OutputStreamBuilder::open_default_stream()
        .map_err(|e| AgentPingError::AudioDeviceError(e.to_string()))?;
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
    let stream = rodio::OutputStreamBuilder::open_default_stream()
        .map_err(|e| AgentPingError::AudioDeviceError(e.to_string()))?;
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

struct AgentPingArgs {
    sound: Option<String>,
    file: Option<String>,
    frequency: Option<f32>,
    duration: u64,
    volume: f32,
    repeat: u32,
    interval: u64,
    list: bool,
    dry_run: bool,
}

fn handle_agent_ping(format: &OutputFormat, args: AgentPingArgs) -> ExitCode {
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

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Example { name }) => {
            let name = name.unwrap_or_else(|| "World".to_string());
            let data = serde_json::json!({
                "message": format!("Hello, {}!", name)
            });

            match cli.format {
                OutputFormat::Json => {
                    let output = Output::success("example", data);
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                }
                OutputFormat::Human => {
                    println!("Hello, {}!", name);
                }
            }
            ExitCode::SUCCESS
        }
        Some(Commands::AgentPing {
            sound,
            file,
            frequency,
            duration,
            volume,
            repeat,
            interval,
            list,
            dry_run,
        }) => handle_agent_ping(
            &cli.format,
            AgentPingArgs {
                sound,
                file,
                frequency,
                duration,
                volume,
                repeat,
                interval,
                list,
                dry_run,
            },
        ),
        None => {
            println!(
                "No command specified. \
                 Use --help for usage information."
            );
            ExitCode::FAILURE
        }
    }
}
