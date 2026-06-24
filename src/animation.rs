use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use log::info;

/// Decoded media's playback state - which frame is "now" and for how long.
/// Doesn't know or care what kind of screen is consuming it; any screen with
/// an `Option<ScreenAnimation<Frame>>` can drive itself off this.
pub struct ScreenAnimation<Frame> {
    frames: Vec<(Frame, Duration)>,
    cumulative_start: Vec<Duration>,
    total_duration: Duration,
    started_at: Instant,
    last_scheduled_frame_index: Option<usize>,
    loops: bool,
    source_done: bool,
    playback_started: bool,
    receiver: Option<Receiver<(Frame, Duration)>>,
    frames_received_since_log: u64,
    frames_reused_since_log: u64,
    last_received_at: Option<Instant>,
    last_live_log_at: Instant,
}

impl<Frame: Clone> ScreenAnimation<Frame> {
    pub fn from_frame_stream(receiver: Receiver<(Frame, Duration)>, loops: bool) -> Self {
        ScreenAnimation {
            frames: Vec::new(),
            cumulative_start: Vec::new(),
            total_duration: Duration::ZERO,
            started_at: Instant::now(),
            last_scheduled_frame_index: None,
            loops,
            source_done: false,
            playback_started: false,
            receiver: Some(receiver),
            frames_received_since_log: 0,
            frames_reused_since_log: 0,
            last_received_at: None,
            last_live_log_at: Instant::now(),
        }
    }

    /// Only true once elapsed time has actually moved onto a new decoded
    /// frame - avoids re-scheduling the same still-current frame every tick.
    pub fn should_build_next_frame(&mut self) -> bool {
        self.ingest_frame_stream();
        if !self.ready_to_play() {
            return false;
        }
        self.ensure_playback_started();

        let elapsed = self.started_at.elapsed();
        let current_frame_index = self.frame_index_for_elapsed(elapsed);
        let frame_advanced = Some(current_frame_index) != self.last_scheduled_frame_index;

        if frame_advanced {
            self.last_scheduled_frame_index = Some(current_frame_index);
        }

        frame_advanced
    }

    /// Picks the current decoded frame and clones it out for the caller to paint.
    pub fn build_frame(&mut self) -> Option<(Frame, Duration)> {
        self.ingest_frame_stream();
        if !self.ready_to_play() {
            return None;
        }
        self.ensure_playback_started();

        let elapsed = self.started_at.elapsed();
        let frame_index = self.frame_index_for_elapsed(elapsed);
        let (frame, delay) = &self.frames[frame_index];

        let duration = if frame_index + 1 < self.cumulative_start.len() {
            self.cumulative_start[frame_index + 1] - self.cumulative_start[frame_index]
        } else {
            *delay
        };

        Some((frame.clone(), duration))
    }

    fn ingest_frame_stream(&mut self) {
        let Some(receiver) = self.receiver.as_ref() else {
            return;
        };

        let mut received_now: u64 = 0;
        loop {
            match receiver.try_recv() {
                Ok(frame) => {
                    self.cumulative_start.push(self.total_duration);
                    self.total_duration += frame.1;
                    self.frames.push(frame);
                    received_now += 1;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.source_done = true;
                    self.receiver = None;
                    break;
                }
            }
        }

        if received_now > 0 {
            self.frames_received_since_log += received_now;
            self.last_received_at = Some(Instant::now());
        } else if !self.frames.is_empty() {
            self.frames_reused_since_log += 1;
        }

        if self.last_live_log_at.elapsed() >= Duration::from_secs(1) {
            let idle_ms = self
                .last_received_at
                .map(|t| t.elapsed().as_millis() as u64)
                .unwrap_or(u64::MAX);

            info!(
                "video stream render stats: new={} reused={} idle_ms={}",
                self.frames_received_since_log, self.frames_reused_since_log, idle_ms
            );

            self.frames_received_since_log = 0;
            self.frames_reused_since_log = 0;
            self.last_live_log_at = Instant::now();
        }
    }

    fn ready_to_play(&self) -> bool {
        !self.frames.is_empty() && (!self.loops || self.source_done)
    }

    fn ensure_playback_started(&mut self) {
        if self.playback_started {
            return;
        }

        self.started_at = Instant::now();
        self.last_scheduled_frame_index = None;
        self.playback_started = true;
    }

    fn frame_index_for_elapsed(&self, elapsed: Duration) -> usize {
        frame_index_for_elapsed(
            &self.cumulative_start,
            self.total_duration,
            elapsed,
            self.loops,
        )
    }
}

pub struct Animation<Frame> {
    pub frame_duration: Duration,
    pub screen_animation: Option<ScreenAnimation<Frame>>,
    pub since: Instant,
}

fn frame_index_for_elapsed(
    cumulative_start: &[Duration],
    total_duration: Duration,
    elapsed: Duration,
    loops: bool,
) -> usize {
    if total_duration.is_zero() || cumulative_start.is_empty() {
        return 0;
    }

    let elapsed_in_range = if loops {
        Duration::from_nanos((elapsed.as_nanos() % total_duration.as_nanos()) as u64)
    } else {
        elapsed.min(total_duration.saturating_sub(Duration::from_nanos(1)))
    };

    match cumulative_start.binary_search_by(|start| start.cmp(&elapsed_in_range)) {
        Ok(index) => index,
        Err(0) => 0,
        Err(index) => index - 1,
    }
}
