use std::sync::Mutex;

use log::info;
use samp::amx::Amx;
use samp::error::AmxResult;
use samp::native;

use crate::amx_natives;
use crate::constants::STREAMER_TYPE_OBJECT;
use crate::screen_3d::Screen3D;
use crate::{AnyScreen, Plugin};

static SCREEN_3D: Mutex<Vec<i32>> = Mutex::new(Vec::new());
static SCREEN_DIALOG: Mutex<Vec<i32>> = Mutex::new(Vec::new());
static SCREEN_TEXTDRAW: Mutex<Vec<i32>> = Mutex::new(Vec::new());
static AUDIO: Mutex<Vec<i32>> = Mutex::new(Vec::new());

fn add(list: &Mutex<Vec<i32>>, player_id: i32) -> bool {
    let Ok(mut list) = list.lock() else {
        return false;
    };
    if list.contains(&player_id) {
        false
    } else {
        list.push(player_id);
        true
    }
}

fn remove(list: &Mutex<Vec<i32>>, player_id: i32) -> bool {
    let Ok(mut list) = list.lock() else {
        return false;
    };
    let before = list.len();
    list.retain(|id| *id != player_id);
    list.len() != before
}

fn contains(list: &Mutex<Vec<i32>>, player_id: i32) -> bool {
    list.lock()
        .map(|list| list.contains(&player_id))
        .unwrap_or(false)
}

fn snapshot(list: &Mutex<Vec<i32>>) -> Vec<i32> {
    list.lock().map(|list| list.clone()).unwrap_or_default()
}

pub fn is_screen_3d_blacklisted(player_id: i32) -> bool {
    contains(&SCREEN_3D, player_id)
}

pub fn is_screen_dialog_blacklisted(player_id: i32) -> bool {
    contains(&SCREEN_DIALOG, player_id)
}

pub fn is_screen_textdraw_blacklisted(player_id: i32) -> bool {
    contains(&SCREEN_TEXTDRAW, player_id)
}

pub fn is_audio_blacklisted(player_id: i32) -> bool {
    contains(&AUDIO, player_id)
}

/// Applies every currently screen_3d-blacklisted player's visibility to a
/// freshly created screen - called once right after `Create3DMediaScreen`
/// builds it, so a player who was already blacklisted never sees it appear
/// in the first place.
pub fn hide_new_screen_3d_from_blacklisted_players(amx: &Amx, screen: &Screen3D) {
    for player_id in snapshot(&SCREEN_3D) {
        toggle_screen_3d_visibility(amx, screen, player_id, false);
    }
}

/// screen_3d objects are ordinary streamer dynamic objects shared by every
/// player in range - `Streamer_ToggleItem` is the streamer plugin's own
/// per-player visibility override, so toggling it here hides/shows the
/// screen for exactly one player without touching anyone else's view of the
/// same object.
fn toggle_screen_3d_visibility(amx: &Amx, screen: &Screen3D, player_id: i32, visible: bool) {
    for buffer in &screen.buffers {
        for tile in &buffer.tiles {
            if let Err(err) = amx_natives::streamer_toggle_item(
                amx,
                player_id,
                STREAMER_TYPE_OBJECT,
                tile.object_id,
                if visible { 1 } else { 0 },
            ) {
                info!(
                    "blacklist -> failed to toggle object {} visibility for player {}: {:?}",
                    tile.object_id, player_id, err
                );
            }
        }
    }
}

impl Plugin {
    fn resync_all_screen_3d(&self, amx: &Amx, player_id: i32, visible: bool) {
        for screen in self.screens.iter().flatten() {
            if let AnyScreen::ThreeD(screen) = screen {
                toggle_screen_3d_visibility(amx, screen, player_id, visible);
            }
        }
    }

    #[native(name = "SVA_BlacklistScreen3DAdd")]
    pub fn sva_blacklist_screen_3d_add(&mut self, amx: &Amx, player_id: i32) -> AmxResult<i32> {
        let added = add(&SCREEN_3D, player_id);
        if added {
            self.resync_all_screen_3d(amx, player_id, false);
        }
        Ok(added as i32)
    }

    #[native(name = "SVA_BlacklistScreen3DRemove")]
    pub fn sva_blacklist_screen_3d_remove(&mut self, amx: &Amx, player_id: i32) -> AmxResult<i32> {
        let removed = remove(&SCREEN_3D, player_id);
        if removed {
            self.resync_all_screen_3d(amx, player_id, true);
        }
        Ok(removed as i32)
    }

    #[native(name = "SVA_BlacklistScreenDialogAdd")]
    pub fn sva_blacklist_screen_dialog_add(
        &mut self,
        _amx: &Amx,
        player_id: i32,
    ) -> AmxResult<i32> {
        Ok(add(&SCREEN_DIALOG, player_id) as i32)
    }

    #[native(name = "SVA_BlacklistScreenDialogRemove")]
    pub fn sva_blacklist_screen_dialog_remove(
        &mut self,
        _amx: &Amx,
        player_id: i32,
    ) -> AmxResult<i32> {
        Ok(remove(&SCREEN_DIALOG, player_id) as i32)
    }

    #[native(name = "SVA_BlacklistScreenTextDrawAdd")]
    pub fn sva_blacklist_screen_textdraw_add(
        &mut self,
        _amx: &Amx,
        player_id: i32,
    ) -> AmxResult<i32> {
        Ok(add(&SCREEN_TEXTDRAW, player_id) as i32)
    }

    #[native(name = "SVA_BlacklistScreenTextDrawRemove")]
    pub fn sva_blacklist_screen_textdraw_remove(
        &mut self,
        _amx: &Amx,
        player_id: i32,
    ) -> AmxResult<i32> {
        Ok(remove(&SCREEN_TEXTDRAW, player_id) as i32)
    }

    #[native(name = "SVA_BlacklistAudioAdd")]
    pub fn sva_blacklist_audio_add(&mut self, _amx: &Amx, player_id: i32) -> AmxResult<i32> {
        Ok(add(&AUDIO, player_id) as i32)
    }

    #[native(name = "SVA_BlacklistAudioRemove")]
    pub fn sva_blacklist_audio_remove(&mut self, _amx: &Amx, player_id: i32) -> AmxResult<i32> {
        Ok(remove(&AUDIO, player_id) as i32)
    }
}
