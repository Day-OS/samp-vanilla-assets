use std::fs::File;
use std::io::{Read, copy, sink};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use ffmpeg_sidecar::child::FfmpegChild;
use ffmpeg_sidecar::command::FfmpegCommand;
use log::info;
use tiny_http::{Header, Response, Server};

use crate::config;

#[derive(Clone)]
struct AudioSource {
    url: String,
    loops: bool,
}

static LIVE_SOURCES: Mutex<Vec<(String, AudioSource)>> = Mutex::new(Vec::new());
static ACTIVE_RELAYS: Mutex<Vec<(String, BroadcastRelay)>> = Mutex::new(Vec::new());

#[derive(Clone)]
struct BroadcastRelay {
    child: Arc<Mutex<FfmpegChild>>,
    clients: Arc<Mutex<Vec<Sender<Vec<u8>>>>>,
}

struct ChannelReader {
    receiver: Receiver<Vec<u8>>,
    chunk: Vec<u8>,
    offset: usize,
}

impl ChannelReader {
    fn new(receiver: Receiver<Vec<u8>>) -> Self {
        Self {
            receiver,
            chunk: Vec::new(),
            offset: 0,
        }
    }
}

impl Read for ChannelReader {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        while self.offset >= self.chunk.len() {
            match self.receiver.recv() {
                Ok(chunk) => {
                    self.chunk = chunk;
                    self.offset = 0;
                }
                Err(_) => return Ok(0),
            }
        }

        let available = self.chunk.len() - self.offset;
        let count = available.min(out.len());
        out[..count].copy_from_slice(&self.chunk[self.offset..self.offset + count]);
        self.offset += count;
        Ok(count)
    }
}

/// Track a live source
pub fn register_live_source(file_path: String, source_url: String, loops: bool) {
    stop_live_source(&file_path);

    let source = AudioSource {
        url: source_url,
        loops,
    };

    match spawn_broadcast_relay(&file_path, &source) {
        Ok(relay) => {
            if let Ok(mut relays) = ACTIVE_RELAYS.lock() {
                relays.push((file_path.clone(), relay));
            }
        }
        Err(err) => {
            info!(
                "register_live_source({}) -> failed to start audio relay: {}",
                file_path, err
            );
            return;
        }
    }

    if let Ok(mut sources) = LIVE_SOURCES.lock() {
        sources.push((file_path, source));
    }
}

pub fn source_has_audio(source_url: &str) -> bool {
    let mut child = match FfmpegCommand::new()
        .args(["-v", "error"])
        .input(source_url)
        .args(["-map", "0:a:0", "-t", "0.1"])
        .format("null")
        .output("-")
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            info!(
                "source_has_audio({}) -> failed to start ffmpeg: {}",
                source_url, err
            );
            return false;
        }
    };

    let status = match child.wait() {
        Ok(status) => status,
        Err(err) => {
            info!(
                "source_has_audio({}) -> ffmpeg wait failed: {}",
                source_url, err
            );
            return false;
        }
    };

    status.success()
}

/// Removes a live source so future requests for it 404, and kills its
/// in-flight relay (if a client is currently streaming it) so the audio
/// actually stops instead of playing out until the client disconnects.
pub fn stop_live_source(path: &str) {
    if let Ok(mut sources) = LIVE_SOURCES.lock() {
        sources.retain(|(registered, _)| !relay_path_matches(path, registered));
    }

    let relays_to_stop = if let Ok(mut relays) = ACTIVE_RELAYS.lock() {
        info!(
            "stop_live_source({}) -> active relays right now: {:?}",
            path,
            relays.iter().map(|(p, _)| p).collect::<Vec<_>>()
        );
        let mut removed = Vec::new();
        let mut index = 0;
        while index < relays.len() {
            if relay_path_matches(path, &relays[index].0) {
                removed.push(relays.remove(index).1);
            } else {
                index += 1;
            }
        }
        removed
    } else {
        Vec::new()
    };

    if relays_to_stop.is_empty() {
        info!(
            "stop_live_source({}) -> no active relay found (nobody currently connected?)",
            path
        );
    }

    for relay in relays_to_stop {
        if let Ok(mut clients) = relay.clients.lock() {
            clients.clear();
        }

        match relay.child.lock() {
            Ok(mut child) => {
                let kill_result = child.kill();
                info!(
                    "stop_live_source({}) -> killed active relay, result={:?}",
                    path, kill_result
                );
            }
            Err(err) => info!(
                "stop_live_source({}) -> relay mutex poisoned: {:?}",
                path, err
            ),
        }
    }
}

fn relay_path_matches(requested: &str, registered: &str) -> bool {
    requested == registered || requested.trim_end_matches('/').ends_with(registered)
}

fn live_source_for(path: &str) -> Option<AudioSource> {
    LIVE_SOURCES
        .lock()
        .ok()?
        .iter()
        .find(|(registered, _)| registered == path)
        .map(|(_, source)| source.clone())
}

pub fn start() {
    let audio_config = &config::get().audio;

    let server = match Server::http(audio_config.server_bind.as_str()) {
        Ok(server) => server,
        Err(err) => {
            info!(
                "audio_server -> failed to bind {}: {}",
                audio_config.server_bind, err
            );
            return;
        }
    };

    info!(
        "audio_server -> serving {} on http://{}",
        audio_config.output_dir, audio_config.server_bind
    );

    thread::spawn(move || {
        for request in server.incoming_requests() {
            let requested = request.url().trim_start_matches('/').to_string();

            if let Some(source) = live_source_for(&requested) {
                respond_with_live_relay(request, &requested, &source);
                continue;
            }

            respond_with_static_file(request, &requested);
        }
    });
}

fn respond_with_static_file(request: tiny_http::Request, requested: &str) {
    let path =
        resolve_path(requested).and_then(|path| File::open(&path).ok().map(|file| (path, file)));

    match path {
        Some((path, file)) => {
            let mut response = Response::from_file(file);
            if let Some(content_type) = content_type_for(&path) {
                if let Ok(header) =
                    Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes())
                {
                    response = response.with_header(header);
                }
            }
            let _ = request.respond(response);
        }
        None => {
            let _ = request.respond(Response::empty(404));
        }
    }
}

fn spawn_mp3_relay(source: &AudioSource) -> std::io::Result<FfmpegChild> {
    let mut command = FfmpegCommand::new();
    if source.loops {
        command.args(["-stream_loop", "-1"]);
    }

    let mut child = command
        .input(&source.url)
        .no_video()
        .codec_audio("libmp3lame")
        .args(["-b:a", "128k"])
        .format("mp3")
        .pipe_stdout()
        .spawn()?;

    if let Some(mut stderr) = child.take_stderr() {
        thread::spawn(move || {
            let _ = copy(&mut stderr, &mut sink());
        });
    }

    Ok(child)
}

fn spawn_broadcast_relay(path: &str, source: &AudioSource) -> std::io::Result<BroadcastRelay> {
    let mut child = match spawn_mp3_relay(source) {
        Ok(child) => child,
        Err(err) => return Err(err),
    };

    let Some(stdout) = child.take_stdout() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "ffmpeg stdout pipe unavailable",
        ));
    };

    let child = Arc::new(Mutex::new(child));
    let clients: Arc<Mutex<Vec<Sender<Vec<u8>>>>> = Arc::new(Mutex::new(Vec::new()));
    let relay_clients = clients.clone();
    let relay_child = child.clone();
    let relay_path = path.to_string();

    thread::spawn(move || {
        let mut stdout = stdout;
        let mut buffer = [0_u8; 16 * 1024];

        loop {
            match stdout.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    let chunk = buffer[..count].to_vec();
                    if let Ok(mut clients) = relay_clients.lock() {
                        clients.retain(|client| client.send(chunk.clone()).is_ok());
                    }
                }
                Err(_) => break,
            }
        }

        if let Ok(mut child) = relay_child.lock() {
            let _ = child.kill();
        }

        info!(
            "spawn_broadcast_relay({}) -> relay producer stopped",
            relay_path
        );
    });

    info!("spawn_broadcast_relay({}) -> relay producer started", path);

    Ok(BroadcastRelay { child, clients })
}

fn relay_for(path: &str) -> Option<BroadcastRelay> {
    ACTIVE_RELAYS
        .lock()
        .ok()?
        .iter()
        .find(|(registered, _)| registered == path)
        .map(|(_, relay)| relay.clone())
}

fn respond_with_live_relay(request: tiny_http::Request, path: &str, _source: &AudioSource) {
    let Some(relay) = relay_for(path) else {
        let _ = request.respond(Response::empty(502));
        return;
    };

    let (sender, receiver) = mpsc::channel();
    if let Ok(mut clients) = relay.clients.lock() {
        clients.push(sender);
    } else {
        let _ = request.respond(Response::empty(502));
        return;
    }

    info!(
        "respond_with_live_relay({}) -> client attached to broadcast relay",
        path
    );

    let header = Header::from_bytes(&b"Content-Type"[..], &b"audio/mpeg"[..]).ok();
    let mut response = Response::new(
        200.into(),
        Vec::new(),
        ChannelReader::new(receiver),
        None,
        None,
    );
    if let Some(header) = header {
        response = response.with_header(header);
    }

    let respond_result = request.respond(response);
    info!(
        "respond_with_live_relay({}) -> request.respond returned: {:?}",
        path, respond_result
    );
}

fn resolve_path(requested: &str) -> Option<PathBuf> {
    if requested.is_empty() || requested.contains("..") {
        return None;
    }

    Some(Path::new(&config::get().audio.output_dir).join(requested))
}

fn content_type_for(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("mp3") => Some("audio/mpeg"),
        Some("ogg") => Some("audio/ogg"),
        _ => None,
    }
}
