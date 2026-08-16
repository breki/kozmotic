use std::process::ExitCode;

use crate::output::{CliError, Output, OutputFormat, Tool, emit_error};

mod playback;

const MUTE_FILE: &str = ".mute-sounds";

fn mute_file_path() -> Option<std::path::PathBuf> {
    crate::self_install::home_dir().map(|h| h.join(".claude").join(MUTE_FILE))
}

fn is_muted() -> bool {
    mute_file_path().is_some_and(|p| p.exists())
}

const SOUND_STOP_CHIME: &[u8] =
    include_bytes!("../assets/sounds/stop-chime.mp3");
const SOUND_NOTIFICATION_CHIME: &[u8] =
    include_bytes!("../assets/sounds/notification-chime.mp3");
const SOUND_ERROR: &[u8] = include_bytes!("../assets/sounds/error.mp3");

/// A built-in sound, named after the Claude Code hook event it is
/// meant for.
///
/// One enum rather than a name list plus a lookup `match`: those were
/// two unlinked places, so adding a preset to one and not the other
/// produced either an undiscoverable sound or a listed name that
/// errored on use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    Stop,
    StopFailure,
    Notification,
}

impl Preset {
    pub const ALL: &'static [Preset] =
        &[Preset::Stop, Preset::StopFailure, Preset::Notification];

    fn bytes(self) -> &'static [u8] {
        match self {
            Preset::Stop => SOUND_STOP_CHIME,
            Preset::StopFailure => SOUND_ERROR,
            Preset::Notification => SOUND_NOTIFICATION_CHIME,
        }
    }
}

impl std::fmt::Display for Preset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Preset::Stop => "Stop",
            Preset::StopFailure => "StopFailure",
            Preset::Notification => "Notification",
        })
    }
}

impl std::str::FromStr for Preset {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "stop" => Ok(Preset::Stop),
            "stopfailure" => Ok(Preset::StopFailure),
            "notification" => Ok(Preset::Notification),
            _ => Err(()),
        }
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
    #[error("duration must be between 1 and {MAX_DURATION_MS} ms, got {0}")]
    InvalidDuration(u64),
    #[error("repeat must be between 1 and {MAX_REPEAT}, got {0}")]
    InvalidRepeat(u32),
    #[error("interval must be at most {MAX_INTERVAL_MS} ms, got {0}")]
    InvalidInterval(u64),
    #[error("file not found: {0}")]
    FileNotFound(String),
    // The rodio error is kept as `#[source]` rather than flattened
    // into the message: the Display text stays library-free (so no
    // dependency type leaks into the JSON `message`), while the
    // causal chain remains walkable for diagnostics.
    #[error("unsupported audio format")]
    UnsupportedFormat(#[source] BoxedCause),
    #[error("audio device error")]
    AudioDeviceError(#[source] BoxedCause),
}

impl CliError for AgentPingError {
    fn code(&self) -> &'static str {
        match self {
            AgentPingError::MissingSoundSource => "MISSING_SOUND_SOURCE",
            AgentPingError::UnknownPreset(_) => "UNKNOWN_PRESET",
            AgentPingError::InvalidFrequency(_) => "INVALID_FREQUENCY",
            AgentPingError::InvalidVolume(_) => "INVALID_VOLUME",
            AgentPingError::InvalidDuration(_) => "INVALID_DURATION",
            AgentPingError::InvalidRepeat(_) => "INVALID_REPEAT",
            AgentPingError::InvalidInterval(_) => "INVALID_INTERVAL",
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

/// Upper bounds on the timing arguments.
///
/// `agent-ping` is the documented Stop/Notification hook command, and
/// these values feed `sleep_until_end` and `thread::sleep` while
/// holding the default audio device. Unbounded, a typo like
/// `--duration 999999999` wedges the hook for days. Frequency and
/// volume were already bounded; these three were not.
const MAX_DURATION_MS: u64 = 10_000;
const MAX_REPEAT: u32 = 10;
const MAX_INTERVAL_MS: u64 = 5_000;

/// The validated sound to play.
///
/// Parsing the three mutually-exclusive `--sound`/`--file`/
/// `--frequency` flags into one value immediately after validation
/// means the rest of the handler matches once and exhaustively. The
/// previous shape re-inspected the raw `Option`s three times and
/// needed two `unreachable!()` and an `unwrap()` whose correctness
/// rested on an `if` sixty lines earlier.
enum SoundSource {
    Preset(Preset),
    Tone { hz: f32, duration_ms: u64 },
    File(String),
}

impl SoundSource {
    fn label(&self) -> String {
        match self {
            SoundSource::Preset(p) => p.to_string(),
            SoundSource::Tone { hz, .. } => format!("{hz} Hz tone"),
            SoundSource::File(path) => path.clone(),
        }
    }
}

/// Opaque cause carried by the audio errors.
pub type BoxedCause = Box<dyn std::error::Error + Send + Sync>;

/// Derives `clap::Args` directly -- see the note on
/// [`crate::sessions::PromptsArgs`].
#[derive(clap::Args)]
pub struct AgentPingArgs {
    /// Play a built-in preset (hook event name)
    #[arg(long, group = "source")]
    pub sound: Option<String>,

    /// Play a custom audio file
    #[arg(long, group = "source")]
    pub file: Option<String>,

    /// Play a generated tone at frequency (Hz)
    #[arg(long, group = "source")]
    pub frequency: Option<f32>,

    /// Tone duration in ms (--frequency only)
    #[arg(long, default_value = "200")]
    pub duration: u64,

    /// Volume 0.0-1.0
    #[arg(long, default_value = "0.5")]
    pub volume: f32,

    /// Play N times
    #[arg(long, default_value = "1")]
    pub repeat: u32,

    /// Gap between repeats in ms
    #[arg(long, default_value = "100")]
    pub interval: u64,

    /// List available presets
    #[arg(long)]
    pub list: bool,

    /// Report what would play, no sound
    #[arg(long)]
    pub dry_run: bool,
}

/// Validate the three mutually-exclusive source flags into one value.
///
/// clap enforces that at most one is present (`group = "source"`);
/// this enforces that at least one is, and that its parameters are in
/// range.
fn parse_source(
    sound: Option<&str>,
    file: Option<&str>,
    frequency: Option<f32>,
    duration: u64,
) -> Result<SoundSource, AgentPingError> {
    if let Some(name) = sound {
        return name
            .parse::<Preset>()
            .map(SoundSource::Preset)
            .map_err(|()| AgentPingError::UnknownPreset(name.to_string()));
    }
    if let Some(hz) = frequency {
        if !(20.0..=20000.0).contains(&hz) {
            return Err(AgentPingError::InvalidFrequency(hz));
        }
        if duration == 0 || duration > MAX_DURATION_MS {
            return Err(AgentPingError::InvalidDuration(duration));
        }
        return Ok(SoundSource::Tone {
            hz,
            duration_ms: duration,
        });
    }
    if let Some(path) = file {
        if !std::path::Path::new(path).exists() {
            return Err(AgentPingError::FileNotFound(path.to_string()));
        }
        return Ok(SoundSource::File(path.to_string()));
    }
    Err(AgentPingError::MissingSoundSource)
}

pub fn handle_agent_ping(
    format: OutputFormat,
    args: AgentPingArgs,
) -> ExitCode {
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
        let presets: Vec<String> =
            Preset::ALL.iter().map(ToString::to_string).collect();
        match format {
            OutputFormat::Json => {
                let data = serde_json::json!({
                    "presets": presets,
                });
                let output = Output::success(Tool::AgentPing, data);
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

    // Validation runs before the mute check, deliberately: hook
    // configs are validated by running them, and with sounds muted a
    // typo'd preset would otherwise exit 0 and only fail later when
    // the user unmutes.
    let source = match parse_source(
        sound.as_deref(),
        file.as_deref(),
        frequency,
        duration,
    ) {
        Ok(source) => source,
        Err(e) => return emit_error(format, Tool::AgentPing, &e),
    };
    if !(0.0..=1.0).contains(&volume) {
        return emit_error(
            format,
            Tool::AgentPing,
            &AgentPingError::InvalidVolume(volume),
        );
    }
    if repeat == 0 || repeat > MAX_REPEAT {
        return emit_error(
            format,
            Tool::AgentPing,
            &AgentPingError::InvalidRepeat(repeat),
        );
    }
    if interval > MAX_INTERVAL_MS {
        return emit_error(
            format,
            Tool::AgentPing,
            &AgentPingError::InvalidInterval(interval),
        );
    }

    // Muted: report the same payload shape as a real play, with
    // `played: false`. Emitting a different shape here made
    // `data.sound` and `data.details` present or absent depending on
    // unrelated user state.
    if is_muted() {
        emit_played(format, Outcome::Muted, &source, volume, repeat);
        return ExitCode::SUCCESS;
    }

    // --dry-run
    if dry_run {
        emit_played(format, Outcome::DryRun, &source, volume, repeat);
        return ExitCode::SUCCESS;
    }

    // Play sound
    let play_result = match &source {
        SoundSource::Preset(p) => {
            playback::play_sound(p.bytes(), volume, repeat, interval)
        }
        SoundSource::Tone { hz, duration_ms } => playback::play_frequency(
            *hz,
            *duration_ms,
            volume,
            repeat,
            interval,
        ),
        SoundSource::File(path) => {
            playback::play_file(path, volume, repeat, interval)
        }
    };

    if let Err(e) = play_result {
        return emit_error(format, Tool::AgentPing, &e);
    }

    emit_played(format, Outcome::Played, &source, volume, repeat);
    ExitCode::SUCCESS
}

/// Why no sound came out, when none did.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Played,
    Muted,
    DryRun,
}

/// Report what was (or would have been) played.
///
/// Takes the parsed [`SoundSource`] rather than the raw flags: the
/// source already knows which of `sound`/`frequency`/`file` applies,
/// so the caller no longer threads six arguments through just for
/// this function to re-derive them.
fn emit_played(
    format: OutputFormat,
    outcome: Outcome,
    source: &SoundSource,
    volume: f32,
    repeat: u32,
) {
    let source_label = source.label();
    match format {
        OutputFormat::Json => {
            let mut details = serde_json::json!({
                "volume": volume,
                "repeat": repeat,
            });
            match source {
                SoundSource::Preset(p) => {
                    details["sound"] = p.to_string().into();
                }
                SoundSource::Tone { hz, duration_ms } => {
                    details["frequency"] = (*hz).into();
                    details["duration_ms"] = (*duration_ms).into();
                }
                SoundSource::File(path) => {
                    details["file"] = path.clone().into();
                }
            }
            let mut data = serde_json::json!({
                "sound": source_label,
                "played": outcome == Outcome::Played,
                "details": details,
            });
            if outcome == Outcome::Muted {
                data["muted"] = true.into();
            }
            let output = Output::success(Tool::AgentPing, data);
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Human => match outcome {
            Outcome::Played => println!("Played: {source_label}"),
            Outcome::Muted => {
                println!("[muted] Sounds are muted: {source_label}");
            }
            Outcome::DryRun => {
                println!("[dry-run] Would play: {source_label}");
            }
        },
    }
}
