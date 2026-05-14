use std::path::PathBuf;

fn log_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("cleanup_crew")
        .join("crash.log")
}

/// Installs a panic hook that appends crash info to the log file before
/// re-running the default hook (so the program still terminates normally).
pub fn install() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = write_entry(info);
        prev(info);
    }));
}

fn write_entry(info: &std::panic::PanicHookInfo) -> std::io::Result<()> {
    use std::io::Write;

    let path = log_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;

    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let location = info
        .location()
        .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
        .unwrap_or_else(|| "unknown".to_string());

    let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        s.clone()
    } else {
        "(non-string panic payload)".to_string()
    };

    writeln!(file, "--- CRASH [unix={secs}] ---")?;
    writeln!(file, "Location : {location}")?;
    writeln!(file, "Message  : {message}")?;
    writeln!(file)?;

    Ok(())
}
