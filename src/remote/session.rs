use super::*;

/// A persisted remote login. Only the session token is stored; the password is
/// never written to disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteSession {
    pub base_url: String,
    pub email: String,
    pub token: String,
}

/// Default location of the remembered remote session.
pub fn session_path() -> PathBuf {
    crate::store::data_path().with_file_name("remote-session.json")
}

/// Persist a session at an explicit path (tests, alternate profiles).
pub fn save_session_at(path: &Path, session: &RemoteSession) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(session)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    write_private_file(path, &text)?;
    Ok(())
}

fn write_private_file(path: &Path, text: &str) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(text.as_bytes())?;
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, text)?;
    }
    Ok(())
}

/// Load a session for `base_url`, ignoring missing/malformed/other-server files.
pub fn load_session_at(path: &Path, base_url: &str) -> Option<RemoteSession> {
    let text = std::fs::read_to_string(path).ok()?;
    let session: RemoteSession = serde_json::from_str(&text).ok()?;
    let expected = base_url.trim_end_matches('/');
    (session.base_url.trim_end_matches('/') == expected).then_some(session)
}

/// Remove a saved session. Missing files are fine.
pub fn clear_session_at(path: &Path) -> io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// Persist the remembered session in the default profile location.
pub fn save_session(base_url: &str, email: &str, token: &str) -> io::Result<()> {
    save_session_at(
        &session_path(),
        &RemoteSession {
            base_url: base_url.to_string(),
            email: email.to_string(),
            token: token.to_string(),
        },
    )
}

/// Load the remembered session for `base_url`.
pub fn load_session(base_url: &str) -> Option<RemoteSession> {
    load_session_at(&session_path(), base_url)
}

/// Forget the remembered session.
pub fn clear_session() -> io::Result<()> {
    clear_session_at(&session_path())
}
