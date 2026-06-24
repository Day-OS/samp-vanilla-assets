use std::process::ExitStatus;

use ffmpeg_sidecar::child::FfmpegChild;
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::iter::FfmpegIterator;

pub fn spawn(cmd: &mut FfmpegCommand) -> Result<FfmpegChild, String> {
    cmd.spawn()
        .map_err(|err| format!("failed to start ffmpeg: {}", err))
}

pub fn events(child: &mut FfmpegChild) -> Result<FfmpegIterator, String> {
    child
        .iter()
        .map_err(|err| format!("failed to read ffmpeg output: {}", err))
}

pub fn wait(child: &mut FfmpegChild) -> Result<ExitStatus, String> {
    child
        .wait()
        .map_err(|err| format!("ffmpeg wait failed: {}", err))
}
