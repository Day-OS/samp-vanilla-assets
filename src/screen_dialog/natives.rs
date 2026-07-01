use samp::amx::Amx;
use samp::error::AmxResult;
use samp::native;
use samp::prelude::*;

use crate::commands::start_media_playback_for;
use crate::{AnyScreen, Plugin};

use super::{DialogDecodeConfig, DialogScreen};

impl Plugin {
    #[native(name = "CreateDialogScreen")]
    pub fn create_dialog_screen(
        &mut self,
        amx: &Amx,
        player_id: i32,
        url: AmxString,
        cols: i32,
        rows: i32,
    ) -> AmxResult<i32> {
        if crate::blacklist::is_screen_dialog_blacklisted(player_id) {
            log::info!(
                "CreateDialogScreen -> player {} is blacklisted from screen_dialog, skipping",
                player_id
            );
            return Ok(-1);
        }

        let url = url.to_string();
        let cols = cols.max(1) as usize;
        let rows = rows.max(1) as usize;
        let screen_index =
            DialogScreen::new(amx, &mut self.screens, player_id, url.clone(), cols, rows);

        match self.screens[screen_index].as_mut() {
            Some(AnyScreen::Dialog(screen)) => {
                start_media_playback_for(
                    screen,
                    url,
                    DialogDecodeConfig { cols, rows },
                    "DialogScreen",
                )?;
            }
            _ => {
                log::info!(
                    "CreateDialogScreen -> screen {} vanished before media playback could start",
                    screen_index
                );
            }
        }

        Ok(screen_index as i32)
    }

    #[native(name = "DestroyDialogScreen")]
    pub fn destroy_dialog_screen(&mut self, amx: &Amx, screen_index: i32) -> AmxResult<i32> {
        if screen_index < 0 {
            return Ok(0);
        }

        match self.screens.get_mut(screen_index as usize) {
            Some(slot) => match slot {
                Some(AnyScreen::Dialog(_)) => {
                    if let Some(screen) = slot.take() {
                        screen.destroy(amx);
                        Ok(1)
                    } else {
                        Ok(0)
                    }
                }
                _ => Ok(0),
            },
            None => Ok(0),
        }
    }
}
