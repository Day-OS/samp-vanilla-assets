use std::process::Command;

pub fn resolve_stream_url(source_url: &str) -> Result<String, String> {
    let output = Command::new("yt-dlp")
        .args(["-g", "-f", "worst", source_url])
        .output()
        .map_err(|err| format!("failed to start yt-dlp: {}", err))?;

    if !output.status.success() {
        return Err(format!(
            "yt-dlp failed (exit {:?}): {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_url = stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .ok_or_else(|| "yt-dlp returned no stream URL".to_string())?;

    Ok(first_url.to_string())
}
