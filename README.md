# Samp Vanilla Assets

**Load external media dynamically without cache.**

Rust plugin for open.mp/SA-MP that renders media (image, GIF, video, and YouTube live) on 3D objects using object material text.

<div align="center">

| 3D elements | Gui images |
|:---:|:---:|
| <img src="docs/fro.png" width="320"> | <img src="docs/2dtext.jpg" width="320"> |

</div>

More examples in [`/docs`](docs).

> ⚠️ **Client compatibility:** requires **SA-MP 0.3.DL**. Object material text — the feature used to render media on 3D objects — only exists on 0.3.DL clients. Players on older SA-MP versions won't see the rendered media.

---

## Requirements

- Rust toolchain (rustup + cargo)
- ffmpeg available in PATH
- yt-dlp available in PATH (for YouTube)
- open.mp/SA-MP server with legacy plugin support
- Players connecting with the SA-MP 0.3.DL client

## Build

From the `samp-vanilla-assets` directory:

```bash
cargo +stable-i686-pc-windows-msvc build --release
```

The build generates a DLL under `target/release` (usually named after the crate, but it may vary if the project is renamed).

## Dependencies

This plugin depends on the [SA-MP Streamer Plugin](https://github.com/samp-incognito/samp-streamer-plugin).

Install it following its instructions, then add it to your server's plugin list **before** loading this plugin.

## Server installation

1. Copy the generated DLL to the server `plugins` folder.
2. Make sure `models/screen.dff` and `models/screen.txd` are present in the server `models` folder.
3. In `config.json`, add the plugin name under `pawn.legacy_plugins` (example: `"samp_vanilla_assets"`).
4. Copy `include/samp_vanilla_assets.inc` into your compiler's include path (e.g. `qawno/include`) and `#include <samp_vanilla_assets>` in your script — see [`docs/pawn-include.md`](docs/pawn-include.md) for the full Pawn API and a working example in [`demo/demo.pwn`](demo/demo.pwn).
5. Restart the server.

## Configuration

Network, audio and screen-model settings live in `SVA_Config.toml`, next to
`omp-server.exe`/`config.json`. It's created automatically (with the defaults
below) the first time the plugin loads if it doesn't exist yet — edit it and
restart the server, no rebuild needed.

```toml
[network]
budget_rate_per_sec = 2800.0
budget_capacity = 60.0

[audio]
server_bind = "0.0.0.0:7878"
output_dir = "samp-led/audio_cache"
base_url = "http://127.0.0.1:7878"

[screen_model]
standard_model_id = -1003
shadow_model_id = -1004
standard_base_model_id = 19805
shadow_base_model_id = 19806
standard_dff = "screen.dff"
standard_txd = "screen.txd"
shadow_dff = "screen-shadow.dff"
shadow_txd = "screen-shadow.txd"
```

| Setting | Purpose |
|---|---|
| `network.budget_rate_per_sec` / `budget_capacity` | Shared object-update token bucket — tune while watching the server log for `client exceeded 'ackslimit'` warnings. |
| `audio.server_bind` | Bind address for the internal HTTP server that relays extracted audio clips. |
| `audio.output_dir` | Where extracted audio clips are cached on disk. |
| `audio.base_url` | Base URL players' clients use to fetch audio — change it if the server sits behind a different public host/port than `server_bind`. |
| `screen_model.*` | Custom model IDs and DFF/TXD file names for the screen/screen-shadow objects — must match whatever's in the server's `models/` folder. |

Everything else (grid size, tile physical dimensions, FPS, etc.) is rendering
logic tied to the shipped `screen.dff`/`screen.txd` assets and stays in
`src/constants.rs` — changing it without matching models will just distort
the mosaic.

## Quick troubleshooting

| Symptom | Fix |
|---|---|
| Live stream start error | Confirm `yt-dlp` is in PATH |
| No audio for video/live | Confirm `ffmpeg` is in PATH and port `7878` is free |
| Missing custom screen model | Confirm `screen.dff` and `screen.txd` are inside `models` |
| Native not found | Confirm plugin is loaded in `config.json` and the gamemode was recompiled |
