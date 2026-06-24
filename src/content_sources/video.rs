use std::time::{Duration, Instant};

use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::{FfmpegEvent, LogLevel};
use log::info;

use crate::constants::VIDEO_OUTPUT_FPS;
use crate::content_sources::ffmpeg_util;

const STREAM_OUTPUT_FPS: u32 = 4;

pub struct RawFrame {
    pub data: Vec<u8>,
    pub delay: Duration,
}

pub enum FrameOutcome {
    Sent,
    Dropped,
    Disconnected,
}

fn letterbox_filter(width: u32, height: u32) -> String {
    format!(
        "scale={w}:{h}:force_original_aspect_ratio=decrease,format=rgba,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:color=black@0.0",
        w = width,
        h = height
    )
}

pub fn load_frames(url: &str, width: u32, height: u32) -> Result<Vec<RawFrame>, String> {
    let frame_delay = Duration::from_secs_f64(1.0 / VIDEO_OUTPUT_FPS as f64);

    info!(
        "load_frames -> spawning ffmpeg for {} ({}x{})",
        url, width, height
    );
    let mut child = ffmpeg_util::spawn(
        FfmpegCommand::new()
            .input(url)
            .args(["-vf", &letterbox_filter(width, height)])
            .rate(VIDEO_OUTPUT_FPS as f32)
            .format("rawvideo")
            .pix_fmt("rgba")
            .no_audio()
            .args(["-sn"])
            .pipe_stdout(),
    )?;
    info!("load_frames -> ffmpeg spawned, reading events");

    let events = ffmpeg_util::events(&mut child)?;

    let mut frames = Vec::new();
    let mut errors = Vec::new();

    for event in events {
        match event {
            FfmpegEvent::OutputFrame(frame) => {
                frames.push(RawFrame {
                    data: frame.data,
                    delay: frame_delay,
                });
            }
            FfmpegEvent::Log(LogLevel::Error | LogLevel::Fatal, message)
            | FfmpegEvent::Error(message) => {
                errors.push(message);
            }
            _ => {}
        }
    }
    info!(
        "load_frames -> event stream ended, {} frame(s) decoded, waiting for exit",
        frames.len()
    );

    let status = ffmpeg_util::wait(&mut child)?;
    info!("load_frames -> ffmpeg exited with {:?}", status.code());

    if frames.is_empty() {
        return Err(format!(
            "ffmpeg produced no frames (exit {:?}): {}",
            status.code(),
            errors.join("\n").trim()
        ));
    }

    Ok(frames)
}

pub fn stream_frames(
    url: &str,
    width: u32,
    height: u32,
    mut on_frame: impl FnMut(RawFrame) -> FrameOutcome,
) -> Result<(), String> {
    loop {
        match stream_frames_once(url, width, height, &mut on_frame) {
            Ok(StreamState::ReceiverDisconnected) => return Ok(()),
            Ok(StreamState::StreamEnded) => {
                info!("video stream ended, reconnecting");
                std::thread::sleep(Duration::from_millis(400));
            }
            Err(err) => {
                info!("video stream decode failed, retrying: {}", err);
                std::thread::sleep(Duration::from_secs(2));
            }
        }
    }
}

enum StreamState {
    StreamEnded,
    ReceiverDisconnected,
}

fn stream_frames_once(
    url: &str,
    width: u32,
    height: u32,
    on_frame: &mut impl FnMut(RawFrame) -> FrameOutcome,
) -> Result<StreamState, String> {
    let frame_delay = Duration::from_secs_f64(1.0 / STREAM_OUTPUT_FPS as f64);

    let mut child = ffmpeg_util::spawn(
        FfmpegCommand::new()
            .args([
                "-re",
                "-rw_timeout",
                "15000000",
                "-reconnect",
                "1",
                "-reconnect_streamed",
                "1",
                "-reconnect_at_eof",
                "1",
                "-reconnect_on_network_error",
                "1",
                "-reconnect_on_http_error",
                "4xx,5xx",
                "-reconnect_delay_max",
                "2",
                "-reconnect_max_retries",
                "0",
            ])
            .input(url)
            .args(["-vf", &letterbox_filter(width, height)])
            .rate(STREAM_OUTPUT_FPS as f32)
            .format("rawvideo")
            .pix_fmt("rgba")
            .no_audio()
            .args(["-sn"])
            .pipe_stdout(),
    )?;

    let events = ffmpeg_util::events(&mut child)?;

    let mut sent_any_frame = false;
    let mut decoded_count: u64 = 0;
    let mut sent_count: u64 = 0;
    let mut dropped_count: u64 = 0;
    let mut stats_window_start = Instant::now();
    let mut errors = Vec::new();

    for event in events {
        let frame = match event {
            FfmpegEvent::OutputFrame(frame) => frame,
            FfmpegEvent::Log(LogLevel::Error | LogLevel::Fatal, message)
            | FfmpegEvent::Error(message) => {
                errors.push(message);
                continue;
            }
            _ => continue,
        };

        decoded_count += 1;
        sent_any_frame = true;

        match on_frame(RawFrame {
            data: frame.data,
            delay: frame_delay,
        }) {
            FrameOutcome::Sent => sent_count += 1,
            FrameOutcome::Dropped => dropped_count += 1,
            FrameOutcome::Disconnected => {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(StreamState::ReceiverDisconnected);
            }
        }

        if stats_window_start.elapsed() >= Duration::from_secs(1) {
            info!(
                "video stream decode stats: decoded={} sent={} dropped={} queue_pressure={:.1}%",
                decoded_count,
                sent_count,
                dropped_count,
                if decoded_count == 0 {
                    0.0
                } else {
                    (dropped_count as f64 / decoded_count as f64) * 100.0
                }
            );
            decoded_count = 0;
            sent_count = 0;
            dropped_count = 0;
            stats_window_start = Instant::now();
        }
    }

    let _ = child.kill();
    let status = ffmpeg_util::wait(&mut child)?;

    if sent_any_frame {
        return Ok(StreamState::StreamEnded);
    }

    Err(format!(
        "ffmpeg produced no stream frames (exit {:?}): {}",
        status.code(),
        errors.join("\n").trim()
    ))
}
