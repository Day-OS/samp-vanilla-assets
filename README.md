# samp-vanilla-assets

Rust plugin for open.mp/SA-MP that renders media (image, GIF, video, and YouTube live) on 3D objects using object material text.
## Requirements

- Rust toolchain (rustup + cargo)
- ffmpeg available in PATH
- yt-dlp available in PATH (for YouTube live)
- open.mp/SA-MP server with legacy plugin support

## Build

From the samp-vanilla-assets directory:

```bash
cargo +stable-i686-pc-windows-msvc build --release
```

The build generates a DLL under target/release (usually named after the crate, but it may vary if the project is renamed).

## Dependencies

This plugin depends on the SA-MP Streamer Plugin.

Download it from:
https://github.com/samp-incognito/samp-streamer-plugin

Install the plugin following its instructions, then add it to your server's plugin list before loading this plugin.

## Server installation

1. Copy the generated DLL to the server plugins folder.
2. Make sure models/screen.dff and models/screen.txd are present in the server models folder.
3. In config.json, add the plugin name under pawn.legacy_plugins (example: "samp_led").
4. Restart the server.


## Audio

- Internal audio HTTP server bind: 0.0.0.0:7878
- Default audio base URL: http://127.0.0.1:7878
- Audio output directory is defined in code (AUDIO_OUTPUT_DIR)

If you want to change bind/port/path, edit constants in src/constants.rs and rebuild.

## Quick troubleshooting

- Live stream start error: confirm yt-dlp is in PATH.
- No audio for video/live: confirm ffmpeg is in PATH and port 7878 is free.
- Missing custom screen model: confirm screen.dff and screen.txd are inside models.
- Native not found: confirm plugin is loaded in config.json and the gamemode was recompiled.
