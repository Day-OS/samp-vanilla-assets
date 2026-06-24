use std::io::Read;
use std::path::Path;

use ffmpeg_sidecar::command::FfmpegCommand;

use crate::content_sources::ffmpeg_util;

pub fn extract_audio(video_path: &str, output_mp3_path: &str) -> Result<(), String> {
    if let Some(parent) = Path::new(output_mp3_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {}", parent.display(), err))?;
    }

    let mut child = ffmpeg_util::spawn(
        FfmpegCommand::new()
            .args(["-v", "error"])
            .overwrite()
            .input(video_path)
            .no_video()
            .codec_audio("libmp3lame")
            .args(["-b:a", "128k"])
            .output(output_mp3_path),
    )?;

    let mut stderr_text = String::new();
    if let Some(mut stderr) = child.take_stderr() {
        let _ = stderr.read_to_string(&mut stderr_text);
    }

    let status = ffmpeg_util::wait(&mut child)?;

    if !status.success() {
        let real_error = stderr_text
            .lines()
            .filter(|line| !line.starts_with("[info]"))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!(
            "ffmpeg failed (exit {:?}): {}",
            status.code(),
            real_error.trim()
        ));
    }

    Ok(())
}
