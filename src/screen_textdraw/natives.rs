use samp::amx::Amx;
use samp::error::AmxResult;
use samp::native;
use samp::prelude::*;

use crate::Plugin;
use crate::commands::start_media_playback_for;
use crate::screen_textdraw::{TextDrawDecodeConfig, TextDrawScreen};
use crate::AnyScreen;

impl Plugin {
    #[native(name = "CreateTextDrawScreen")]
    pub fn create_textdraw_screen(
        &mut self,
        amx: &Amx,
        url: AmxString,
        playerid: i32,
        x: f32,
        y: f32,
        cols: i32,
        rows: i32,
        letter_size_x: f32,
        letter_size_y: f32,
        box_scale: f32,
        budget: i32,
    ) -> AmxResult<i32> {
        if crate::blacklist::is_screen_textdraw_blacklisted(playerid) {
            log::info!(
                "CreateTextDrawScreen -> player {} is blacklisted from screen_textdraw, skipping",
                playerid
            );
            return Ok(-1);
        }

        let url = url.to_string();
        let cols = (cols.max(1)) as usize;
        let rows = (rows.max(1)) as usize;
        let budget = (budget.max(1)) as usize;

        let screen_index = TextDrawScreen::new(
            amx,
            &mut self.screens,
            playerid,
            cols,
            rows,
            x,
            y,
            letter_size_x,
            letter_size_y,
            box_scale,
            budget,
        )?;

        let config = TextDrawDecodeConfig { cols, rows, budget };
        match self.screens[screen_index].as_mut() {
            Some(AnyScreen::TextDraw(screen)) => {
                start_media_playback_for(screen, url, config, "TextDrawScreen")?;
            }
            _ => {
                log::info!("CreateTextDrawScreen -> screen {} vanished before playback could start", screen_index);
            }
        }

        Ok(screen_index as i32)
    }

    #[native(name = "DestroyTextDrawScreen")]
    pub fn destroy_textdraw_screen(&mut self, amx: &Amx, screen_index: i32) -> AmxResult<i32> {
        if screen_index < 0 {
            return Ok(0);
        }

        match self.screens.get_mut(screen_index as usize) {
            Some(slot) => match slot.take() {
                Some(screen) => {
                    screen.destroy(amx);
                    Ok(1)
                }
                None => Ok(0),
            },
            None => Ok(0),
        }
    }
}
