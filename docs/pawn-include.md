# `samp_vanilla_assets.inc`

Pawn API for the plugin. Copy `include/samp_vanilla_assets.inc` into your
compiler's include path (e.g. `qawno/include` or `pawno/include`) and add:

```pawn
#include <open.mp>
#include <streamer>
#include <samp_vanilla_assets>
```

`open.mp`/`a_samp` and `streamer` must come first — the include depends on
both (screens are backed by streamer dynamic objects/areas and by ordinary
objects/textdraws/dialogs).

A full working example is in [`demo/demo.pwn`](../demo/demo.pwn).

## Natives

### 3D screens

```pawn
native Create3DMediaScreen(const url[], Float:x, Float:y, Float:z, Float:rotationX = 0.0, Float:rotationY = 0.0, Float:rotationZ = 0.0, tileCols = 1, tileRows = 1, playerid = -1, world_id = -1, interior_id = -1, Float:audioRange = 5.0, Float:hiddenX = 0.0, Float:hiddenY = 0.0, Float:hiddenZ = 0.0, E_SCREEN_MODEL:modelId = SCREEN_MODEL_STANDARD);
native Destroy3DMediaScreen(screenIndex);
```

Creates a 3D screen at `x, y, z`. The plugin detects image/gif/video/YouTube
live from `url` on its own. `playerid` only matters for video/live (it's who
the extracted audio targets — pass `-1` for none). `tileCols`/`tileRows` size
the mosaic and apply to any media kind. `modelId` picks the 3D model (see
`E_SCREEN_MODEL`). `hiddenX/Y/Z` is where the screen's helper objects park
while out of range — keep it away from `x, y, z` (the demo offsets it on Z).

Returns a `screenIndex` handle. Keep it if you want to `Destroy3DMediaScreen`
it later — it stays valid for the screen's whole lifetime. `Destroy3DMediaScreen`
returns `0` if the index is out of range or was already destroyed.

```pawn
native Create3DMediaScreenPreview(Float:x, Float:y, Float:z, Float:rotationX = 0.0, Float:rotationY = 0.0, Float:rotationZ = 0.0, tileCols = 1, tileRows = 1, worldid = -1, interior_id = -1, playerid = -1);
native Destroy3DMediaScreenPreview(previewObjectId);
```

Spawns a draggable/rotatable placeholder object (no media attached yet) meant
to be handed to the streamer's native object editor (`EditDynamicObject`).
Use this for "drag the screen where you want it, then confirm" UX — see
`OnPlayerEditDynamicObject` in the demo for the full flow. Always destroy the
preview (`Destroy3DMediaScreenPreview`) once the player confirms or cancels,
whether or not you go on to call `Create3DMediaScreen`.

### Dialog screens

```pawn
native CreateDialogScreen(playerid, const url[], cols = 32, rows = 32);
native DestroyDialogScreen(screenIndex);
```

Renders media inside a player's dialog box. Destroy it once the dialog closes
(`OnDialogResponse`) or the resources leak for that player.

### TextDraw screens

```pawn
native CreateTextDrawScreen(const url[], playerid, Float:x, Float:y, cols = 64, rows = 64, Float:letterSizeX = 0.25, Float:letterSizeY = 0.35, Float:boxScale = 9.0, budget = 256);
native DestroyTextDrawScreen(screenIndex);
```

Renders media as a HUD overlay via `PlayerTextDraw`s at screen position
`x, y`.

### Per-player blacklists

```pawn
native SVA_BlacklistScreen3DAdd(playerid);
native SVA_BlacklistScreen3DRemove(playerid);
native SVA_BlacklistScreenDialogAdd(playerid);
native SVA_BlacklistScreenDialogRemove(playerid);
native SVA_BlacklistScreenTextDrawAdd(playerid);
native SVA_BlacklistScreenTextDrawRemove(playerid);
native SVA_BlacklistAudioAdd(playerid);
native SVA_BlacklistAudioRemove(playerid);
```

Lets a player opt out of one kind of content without affecting anyone else.
A blacklisted player simply never has that kind of screen (or audio) created
for them — `Create3DMediaScreen`/`CreateDialogScreen`/`CreateTextDrawScreen`
silently return `INVALID_SCREEN_INDEX` for them, and no `PlayAudioStreamForPlayer`
call is ever made for a player on the audio blacklist, regardless of which
screen type triggered it.

`screen_3d` is the one shared-with-everyone case (a real object in the
world): blacklisting a player there hides the object *only for them*, via the
streamer plugin's own per-player visibility toggle — everyone else still
sees it normally, including screens created before the player was
blacklisted.

Each `*Add`/`*Remove` native returns `1` if the player's membership actually
changed, `0` if they were already in that state (e.g. calling `*Add` twice in
a row for the same player).

## Constants

- `E_SCREEN_MODEL:SCREEN_MODEL_STANDARD` / `E_SCREEN_MODEL:SCREEN_MODEL_SHADOW` — 3D model choice for `Create3DMediaScreen`.
- `INVALID_SCREEN_INDEX` — compare a returned `screenIndex` against this before destroying it.

## Callbacks — no setup required

`OnPlayerEnterDynamicArea` and `OnPlayerLeaveDynamicArea` are wired
automatically by this include. 3D screens use streamer dynamic areas to know
when a player is in range (audio triggering, etc.), and those areas only
notify Pawn through these two ordinary callbacks — so the include defines
them itself and forwards into the plugin.

You can still define `OnPlayerEnterDynamicArea`/`OnPlayerLeaveDynamicArea`
yourself, anywhere *after* `#include <samp_vanilla_assets>`, exactly like any
other callback — no special name, no manual native call:

```pawn
#include <samp_vanilla_assets>

public OnPlayerEnterDynamicArea(playerid, STREAMER_TAG_AREA:areaid)
{
    // your own logic — the include already ran its own listener before this
    return 1;
}
```

This works through the same identifier-rename technique (`ALS`) used by
`streamer.inc`/`foreach.inc`: your `public` is silently renamed to an
internal hook symbol and chained in, so there's no "duplicate public"
conflict even though both the include and your script use the same callback
name. The include always returns `1` regardless of what your hook returns.

## Memory

The plugin allocates large strings on the AMX heap for the pixel-grid data.
The default AMX dynamic memory is small (a 2005-era default), so raise it in
your script:

```pawn
#pragma dynamic 1048576
```

Tune the size up if you hit heap-related runtime errors with bigger
`tileCols`/`tileRows` grids.
