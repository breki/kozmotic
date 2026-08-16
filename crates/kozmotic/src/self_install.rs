use std::path::PathBuf;
use std::process::ExitCode;

use crate::output::{CliError, Output, OutputFormat, Tool, emit_error};

#[derive(Debug, thiserror::Error)]
enum SelfInstallError {
    #[error("cannot determine home directory")]
    HomeNotFound,
    #[error("failed to create directory: {0}")]
    CreateDir(std::io::Error),
    #[error("failed to copy binary: {0}")]
    CopyBinary(std::io::Error),
    #[error("cannot resolve own executable path: {0}")]
    CurrentExe(std::io::Error),
}

impl CliError for SelfInstallError {
    fn code(&self) -> &'static str {
        match self {
            SelfInstallError::HomeNotFound => "HOME_NOT_FOUND",
            SelfInstallError::CreateDir(_) => "CREATE_DIR",
            SelfInstallError::CopyBinary(_) => "COPY_BINARY",
            SelfInstallError::CurrentExe(_) => "CURRENT_EXE",
        }
    }
}

pub fn home_dir() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE").ok().map(PathBuf::from)
    }
}

/// Derives `clap::Args` directly -- see the note on
/// [`crate::sessions::PromptsArgs`].
#[derive(clap::Args)]
pub struct SelfInstallArgs {
    /// Override the install directory
    #[arg(long)]
    pub target_dir: Option<PathBuf>,
}

pub fn handle_self_install(
    format: OutputFormat,
    args: SelfInstallArgs,
) -> ExitCode {
    // Every failure here reports the same way; naming it once keeps
    // each branch to a single readable line.
    let fail = |e: SelfInstallError| emit_error(format, Tool::SelfInstall, &e);
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            return fail(SelfInstallError::CurrentExe(e));
        }
    };

    let target_dir = if let Some(d) = args.target_dir {
        d
    } else {
        let Some(home) = home_dir() else {
            return fail(SelfInstallError::HomeNotFound);
        };
        home.join(".claude").join("bin")
    };

    if let Err(e) = std::fs::create_dir_all(&target_dir) {
        return fail(SelfInstallError::CreateDir(e));
    }

    let binary_name = if cfg!(windows) {
        "kozmotic.exe"
    } else {
        "kozmotic"
    };
    let dest = target_dir.join(binary_name);

    if let Err(e) = std::fs::copy(&exe, &dest) {
        return fail(SelfInstallError::CopyBinary(e));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        if let Err(e) = std::fs::set_permissions(&dest, perms) {
            return fail(SelfInstallError::CopyBinary(e));
        }
    }

    let installed_path = dest.display().to_string();
    let tilde_path = if let Some(home) = home_dir() {
        installed_path.replace(&home.display().to_string(), "~")
    } else {
        installed_path.clone()
    };
    let hook_example = format!("{tilde_path} agent-ping --sound Stop");

    match format {
        OutputFormat::Json => {
            let data = serde_json::json!({
                "installed_path": installed_path,
                "hook_example": hook_example,
            });
            let output = Output::success(Tool::SelfInstall, data);
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Human => {
            println!("Installed to {tilde_path}");
            println!();
            println!("Use in Claude Code hooks:");
            println!("  {hook_example}");
        }
    }
    ExitCode::SUCCESS
}
