//! Locating Claude Code's on-disk session transcripts.
//!
//! Transcripts live at `<config>/projects/<slug>/<session-id>.jsonl`,
//! where `<slug>` is the project's working directory with every
//! non-alphanumeric character replaced by `-`.

use std::path::{Path, PathBuf};

use crate::self_install::home_dir;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("cannot determine home directory")]
    HomeNotFound,
    #[error("no session storage at {0}")]
    NoStorage(PathBuf),
    #[error("no session {0} found on disk")]
    SessionNotFound(String),
    #[error("not a valid session id")]
    InvalidSessionId,
    #[error("cannot read transcript {0}: {1}")]
    Unreadable(PathBuf, std::io::Error),
    #[error("no sessions recorded for {0}")]
    NoSessionsForProject(PathBuf),
    #[error("cannot determine the current directory: {0}")]
    Cwd(std::io::Error),
}

impl crate::output::CliError for StoreError {
    fn code(&self) -> &'static str {
        match self {
            StoreError::HomeNotFound => "HOME_NOT_FOUND",
            StoreError::NoStorage(_) => "NO_STORAGE",
            StoreError::SessionNotFound(_) => "SESSION_NOT_FOUND",
            StoreError::InvalidSessionId => "INVALID_SESSION_ID",
            StoreError::Unreadable(_, _) => "TRANSCRIPT_UNREADABLE",
            StoreError::NoSessionsForProject(_) => "NO_SESSIONS",
            StoreError::Cwd(_) => "CWD_UNAVAILABLE",
        }
    }
}

/// A located transcript file plus the identity we resolved it from.
#[derive(Debug)]
pub struct Transcript {
    pub session_id: String,
    pub path: PathBuf,
    /// The project directory the transcript was recorded under, as
    /// the original path when we can recover it from the slug.
    pub project_dir: PathBuf,
}

/// Root of Claude Code's per-project transcript storage.
///
/// `CLAUDE_CONFIG_DIR` wins when set, matching Claude Code itself;
/// otherwise `~/.claude`.
pub fn projects_root() -> Result<PathBuf, StoreError> {
    let base = match std::env::var("CLAUDE_CONFIG_DIR") {
        Ok(dir) if !dir.trim().is_empty() => PathBuf::from(dir),
        _ => home_dir().ok_or(StoreError::HomeNotFound)?.join(".claude"),
    };
    Ok(base.join("projects"))
}

/// Claude Code's directory name for a project path: every character
/// outside `[A-Za-z0-9]` becomes `-`.
pub fn slug_for(path: &Path) -> String {
    path.to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// Resolve a transcript: `session` when the caller named one,
/// otherwise the project's most recently modified transcript.
///
/// The caller decides what `session` is — see [`current_session_id`]
/// — so this stays a pure function of the store on disk.
pub fn resolve(
    root: &Path,
    project: Option<PathBuf>,
    session: Option<String>,
) -> Result<Transcript, StoreError> {
    if !root.is_dir() {
        return Err(StoreError::NoStorage(root.to_path_buf()));
    }

    let project_dir = match project {
        Some(p) => p,
        None => std::env::current_dir().map_err(StoreError::Cwd)?,
    };

    match session {
        Some(id) => find_by_id(root, &project_dir, &id),
        None => newest_in_project(root, &project_dir),
    }
}

/// The session id of the Claude Code session running this command,
/// if we are running inside one. Claude Code exports it to every
/// command it spawns, so an agent inspecting its own transcript
/// needs no arguments.
pub fn current_session_id() -> Option<String> {
    std::env::var("CLAUDE_CODE_SESSION_ID")
        .ok()
        .filter(|id| !id.trim().is_empty())
}

/// A session id is a bare token, never a path fragment.
///
/// This is a security boundary, not tidiness. The id is interpolated
/// into a filename, and `Path::join` neither normalises `..` nor
/// resists an absolute argument (which replaces the whole prefix),
/// so an unchecked id turns this command into an arbitrary-file
/// reader for anything ending in `.jsonl`. The command is invoked
/// from hooks and subagents, where the id can come from
/// model-influenced text, so the check belongs here rather than at
/// the call sites.
fn valid_session_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Look for `<id>.jsonl` under the project's own directory first,
/// then anywhere in the store — a session id is globally unique, so
/// finding it under another project is better than reporting it
/// missing.
fn find_by_id(
    root: &Path,
    project_dir: &Path,
    id: &str,
) -> Result<Transcript, StoreError> {
    if !valid_session_id(id) {
        // Deliberately does not echo the input back: the error is
        // otherwise a probe oracle for what exists on disk.
        return Err(StoreError::InvalidSessionId);
    }
    let file = format!("{id}.jsonl");
    let local = root.join(slug_for(project_dir)).join(&file);
    if local.is_file() {
        return Ok(Transcript {
            session_id: id.to_string(),
            path: local,
            project_dir: project_dir.to_path_buf(),
        });
    }

    for dir in project_dirs(root) {
        let candidate = dir.join(&file);
        if candidate.is_file() {
            return Ok(Transcript {
                session_id: id.to_string(),
                path: candidate,
                project_dir: dir,
            });
        }
    }

    Err(StoreError::SessionNotFound(id.to_string()))
}

/// The project's transcript with the newest modification time.
fn newest_in_project(
    root: &Path,
    project_dir: &Path,
) -> Result<Transcript, StoreError> {
    let dir = root.join(slug_for(project_dir));
    let newest = transcripts_in(&dir)
        .into_iter()
        .max_by_key(|(_, mtime)| *mtime)
        .map(|(path, _)| path)
        .ok_or_else(|| {
            StoreError::NoSessionsForProject(project_dir.to_path_buf())
        })?;

    Ok(Transcript {
        session_id: session_id_of(&newest),
        path: newest,
        project_dir: project_dir.to_path_buf(),
    })
}

/// Every `*.jsonl` in `dir`, paired with its modification time.
/// An unreadable directory yields nothing rather than failing: the
/// caller reports "no sessions", which is what the user sees anyway.
fn transcripts_in(dir: &Path) -> Vec<(PathBuf, std::time::SystemTime)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "jsonl"))
        .filter_map(|e| {
            let mtime = e.metadata().ok()?.modified().ok()?;
            Some((e.path(), mtime))
        })
        .collect()
}

fn project_dirs(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect()
}

fn session_id_of(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::CliError;

    /// A store with one project directory holding the named
    /// transcripts, created oldest-first so mtimes are ordered.
    fn store(project: &Path, files: &[&str]) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join(slug_for(project));
        std::fs::create_dir_all(&dir).unwrap();
        for (i, f) in files.iter().enumerate() {
            let path = dir.join(format!("{f}.jsonl"));
            std::fs::write(&path, "{}\n").unwrap();
            // Stamp increasing mtimes: writes within the same
            // filesystem tick would otherwise tie.
            let when = std::time::SystemTime::UNIX_EPOCH
                + std::time::Duration::from_secs(1_700_000_000 + i as u64);
            filetime::set_file_mtime(&path, filetime::FileTime::from(when))
                .unwrap();
        }
        root
    }

    #[test]
    fn slug_replaces_every_non_alphanumeric() {
        assert_eq!(
            slug_for(Path::new("/home/vagrant/kozmotic")),
            "-home-vagrant-kozmotic"
        );
        assert_eq!(slug_for(Path::new("/tmp/my_app.v2")), "-tmp-my-app-v2");
    }

    #[test]
    fn missing_store_is_an_error() {
        let root = tempfile::tempdir().unwrap();
        let missing = root.path().join("absent");
        let err =
            resolve(&missing, Some(PathBuf::from("/p")), None).unwrap_err();
        assert_eq!(err.code(), "NO_STORAGE");
    }

    #[test]
    fn resolves_the_newest_transcript_for_the_project() {
        let project = PathBuf::from("/p/one");
        let root = store(&project, &["old", "new"]);
        let t = resolve(root.path(), Some(project.clone()), None).unwrap();
        assert_eq!(t.session_id, "new");
        assert_eq!(t.project_dir, project);
    }

    #[test]
    fn resolves_an_explicit_session_id() {
        let project = PathBuf::from("/p/one");
        let root = store(&project, &["old", "new"]);
        let t = resolve(root.path(), Some(project), Some("old".to_string()))
            .unwrap();
        assert_eq!(t.session_id, "old");
    }

    #[test]
    fn finds_a_session_recorded_under_another_project() {
        let elsewhere = PathBuf::from("/p/other");
        let root = store(&elsewhere, &["stray"]);
        let t = resolve(
            root.path(),
            Some(PathBuf::from("/p/here")),
            Some("stray".to_string()),
        )
        .unwrap();
        assert_eq!(t.session_id, "stray");
        assert!(t.path.is_file());
    }

    #[test]
    fn path_shaped_session_ids_are_rejected() {
        let project = PathBuf::from("/p/one");
        let root = store(&project, &["only"]);
        // A file that exists, reachable only by escaping the store.
        let outside = root.path().parent().unwrap().join("outside.jsonl");
        let _ = std::fs::write(&outside, "{}\n");
        for id in [
            "../../outside",
            "..",
            "/etc/passwd",
            "a/b",
            "a\\b",
            "",
            "with space",
        ] {
            let err = resolve(
                root.path(),
                Some(project.clone()),
                Some(id.to_string()),
            )
            .unwrap_err();
            assert_eq!(err.code(), "INVALID_SESSION_ID", "id {id:?}");
        }
    }

    #[test]
    fn ordinary_session_ids_are_accepted() {
        for id in ["only", "5aa654de-734a-4cf4-8d43-36f51c716a83", "a_b-1"] {
            assert!(valid_session_id(id), "id {id:?}");
        }
        assert!(!valid_session_id(&"a".repeat(129)));
    }

    #[test]
    fn unknown_session_id_is_an_error() {
        let project = PathBuf::from("/p/one");
        let root = store(&project, &["only"]);
        let err = resolve(root.path(), Some(project), Some("nope".to_string()))
            .unwrap_err();
        assert_eq!(err.code(), "SESSION_NOT_FOUND");
    }

    #[test]
    fn project_without_sessions_is_an_error() {
        let root = store(&PathBuf::from("/p/one"), &[]);
        let err = resolve(root.path(), Some(PathBuf::from("/p/two")), None)
            .unwrap_err();
        assert_eq!(err.code(), "NO_SESSIONS");
    }

    #[test]
    fn transcripts_in_an_unreadable_directory_are_empty() {
        assert!(transcripts_in(Path::new("/nonexistent/xyz")).is_empty());
        assert!(project_dirs(Path::new("/nonexistent/xyz")).is_empty());
    }

    #[test]
    fn session_id_falls_back_to_empty_for_a_rootless_path() {
        assert_eq!(session_id_of(Path::new("/")), "");
    }
}
