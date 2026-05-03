use std::path::PathBuf;
use std::process::ExitCode;

use crate::output::{Output, OutputFormat};

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

impl SelfInstallError {
    fn code(&self) -> &'static str {
        match self {
            SelfInstallError::HomeNotFound => "HOME_NOT_FOUND",
            SelfInstallError::CreateDir(_) => "CREATE_DIR",
            SelfInstallError::CopyBinary(_) => "COPY_BINARY",
            SelfInstallError::CurrentExe(_) => "CURRENT_EXE",
        }
    }
}

fn emit_error(format: &OutputFormat, err: &SelfInstallError) -> ExitCode {
    match format {
        OutputFormat::Json => {
            let output = Output::error("self-install", err.code(), &err.to_string());
            eprintln!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Human => {
            eprintln!("Error [{}]: {}", err.code(), err);
        }
    }
    ExitCode::FAILURE
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

pub fn handle_self_install(format: &OutputFormat, target_dir: Option<PathBuf>) -> ExitCode {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            return emit_error(format, &SelfInstallError::CurrentExe(e));
        }
    };

    let target_dir = match target_dir {
        Some(d) => d,
        None => {
            let Some(home) = home_dir() else {
                return emit_error(format, &SelfInstallError::HomeNotFound);
            };
            home.join(".claude").join("bin")
        }
    };

    if let Err(e) = std::fs::create_dir_all(&target_dir) {
        return emit_error(format, &SelfInstallError::CreateDir(e));
    }

    let binary_name = if cfg!(windows) {
        "kozmotic.exe"
    } else {
        "kozmotic"
    };
    let dest = target_dir.join(binary_name);

    if let Err(e) = std::fs::copy(&exe, &dest) {
        return emit_error(format, &SelfInstallError::CopyBinary(e));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        if let Err(e) = std::fs::set_permissions(&dest, perms) {
            return emit_error(format, &SelfInstallError::CopyBinary(e));
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
            let output = Output::success("self-install", data);
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
