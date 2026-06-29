use std::sync::atomic::Ordering;
use std::sync::mpsc::{Sender, SyncSender};
use std::time::Duration;

use log::info;
use samp::prelude::*;

use crate::animation::ScreenAnimation;
use crate::audio_server;
use crate::constants::AUDIO_BASE_URL;
use crate::content_sources::yt_resolver;
use crate::screen::Screen;
use crate::AUDIO_CLIP_COUNTER;

fn is_youtube_url(url: &str) -> bool {
    url.contains("youtube.com") || url.contains("youtu.be")
}

fn attach_audio_to_screen<S: Screen>(
    screen: &mut S,
    source_url: &str,
    loops: bool,
    log_context: &str,
) {
    if !audio_server::source_has_audio(source_url) {
        info!("{log_context} -> source has no audio stream, skipping audio relay");
        return;
    }

    let audio_id = AUDIO_CLIP_COUNTER.fetch_add(1, Ordering::SeqCst);
    let audio_relay_path = format!("clip_{}.mp3", audio_id);
    let audio_url = format!("{}/{}", AUDIO_BASE_URL, audio_relay_path);

    audio_server::register_live_source(audio_relay_path.clone(), source_url.to_string(), loops);
    screen.set_audio_url(audio_url);
    screen.set_audio_relay_path(audio_relay_path);
}

fn start_screen_media<S>(
    screen: &mut S,
    media_url: String,
    config: S::DecodeConfig,
    is_live_source: bool,
    log_context: &'static str,
) where
    S: Screen + 'static,
    S::DecodeConfig: Clone + Send + 'static,
    S::Frame: Send + 'static,
{
    if is_live_source {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        spawn_stream_decode::<S>(media_url, tx, config, log_context);
        screen.set_animation(ScreenAnimation::from_frame_stream(rx, false));
    } else {
        let (tx, rx) = std::sync::mpsc::channel();
        spawn_clip_decode::<S>(media_url, tx, config, log_context);
        screen.set_animation(ScreenAnimation::from_frame_stream(rx, true));
    }
}

pub(crate) fn start_media_playback_for<S>(
    screen: &mut S,
    url: String,
    config: S::DecodeConfig,
    log_context: &'static str,
) -> AmxResult<()>
where
    S: Screen + 'static,
    S::DecodeConfig: Clone + Send + 'static,
    S::Frame: Send + 'static,
{
    let is_live_source = is_youtube_url(&url);
    let media_url = if is_live_source {
        let resolved = match yt_resolver::resolve_stream_url(&url) {
            Ok(resolved) => resolved,
            Err(err) => {
                log::error!(
                    "{log_context} -> failed to resolve youtube url {}: {}",
                    url,
                    err
                );
                return Ok(());
            }
        };

        log::info!(
            "{log_context} -> resolved youtube url {} to {}",
            url,
            resolved
        );
        resolved
    } else {
        url
    };

    attach_audio_to_screen(screen, &media_url, true, log_context);
    start_screen_media::<S>(screen, media_url, config, is_live_source, log_context);
    Ok(())
}

fn spawn_stream_decode<S>(
    media_url: String,
    tx: SyncSender<(S::Frame, Duration)>,
    config: S::DecodeConfig,
    log_context: &'static str,
) where
    S: Screen + 'static,
    S::DecodeConfig: Send + 'static,
    S::Frame: Send + 'static,
{
    std::thread::spawn(move || {
        if let Err(err) = S::stream_clip(&media_url, tx, &config) {
            info!("{log_context}::stream_clip -> video stream ended: {}", err);
        }
    });
}

fn spawn_clip_decode<S>(
    media_url: String,
    tx: Sender<(S::Frame, Duration)>,
    config: S::DecodeConfig,
    log_context: &'static str,
) where
    S: Screen + 'static,
    S::DecodeConfig: Send + 'static,
    S::Frame: Send + 'static,
{
    std::thread::spawn(move || {
        if let Err(err) = S::load_clip(&media_url, tx, &config) {
            info!("{log_context}::load_clip -> clip decode failed: {}", err);
        }
        info!("{log_context}::load_clip -> clip decode thread finished");
    });
}
