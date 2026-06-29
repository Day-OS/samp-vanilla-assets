use samp::amx::{AmxIdent, get as get_amx};
use samp::initialize_plugin;
use samp::prelude::*;

use log::info;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

mod content_sources;
mod screen_2d;
mod screen_3d;
mod screen_dialog;

mod amx_natives;
mod animation;
mod audio_server;
mod commands;
mod constants;
mod engine;
mod network_budget;
mod screen;

use crate::constants::{NETWORK_BUDGET_CAPACITY, NETWORK_BUDGET_RATE_PER_SEC};
use crate::network_budget::NetworkBudget;
use crate::screen::Screen;
use crate::screen_3d::Screen3D;
use crate::screen_3d::screen_buffer::ensure_screen_model_registered;
use crate::screen_dialog::DialogScreen;

static AUDIO_SERVER_STARTED: AtomicBool = AtomicBool::new(false);
static AUDIO_CLIP_COUNTER: AtomicUsize = AtomicUsize::new(0);

enum AnyScreen {
    ThreeD(Screen3D),
    Dialog(DialogScreen),
}

impl AnyScreen {
    fn amx_ident(&self) -> AmxIdent {
        match self {
            AnyScreen::ThreeD(screen) => Screen::amx_ident(screen),
            AnyScreen::Dialog(screen) => Screen::amx_ident(screen),
        }
    }

    fn tick(&mut self, amx: &Amx, budget: &mut NetworkBudget) -> AmxResult<()> {
        match self {
            AnyScreen::ThreeD(screen) => Screen::tick(screen, amx, budget),
            AnyScreen::Dialog(screen) => Screen::tick(screen, amx, budget),
        }
    }

    fn destroy(&self, amx: &Amx) {
        match self {
            AnyScreen::ThreeD(screen) => Screen::destroy(screen, amx),
            AnyScreen::Dialog(screen) => Screen::destroy(screen, amx),
        }
    }

    fn handle_area_enter(&mut self, amx: &Amx, player_id: i32, area_id: i32) -> AmxResult<bool> {
        match self {
            AnyScreen::ThreeD(screen) => Screen::handle_area_enter(screen, amx, player_id, area_id),
            AnyScreen::Dialog(screen) => Screen::handle_area_enter(screen, amx, player_id, area_id),
        }
    }

    fn handle_area_leave(&mut self, amx: &Amx, player_id: i32, area_id: i32) -> AmxResult<bool> {
        match self {
            AnyScreen::ThreeD(screen) => Screen::handle_area_leave(screen, amx, player_id, area_id),
            AnyScreen::Dialog(screen) => Screen::handle_area_leave(screen, amx, player_id, area_id),
        }
    }
}

struct Plugin {
    // `None` marks a destroyed screen. A plain `remove`/`swap_remove` would
    // shift later indices, invalidating every `screen_index` a Pawn script
    // is still holding onto - leaving a hole in place keeps every index
    // stable for the plugin's whole lifetime.
    screens: Vec<Option<AnyScreen>>,
    placement_previews: Vec<Vec<i32>>,
    network_budget: NetworkBudget,
    tick_priority_offset: usize,
}

impl SampPlugin for Plugin {
    fn on_load(&mut self) {
        info!("Plugin is loaded.");
    }

    fn on_amx_load(&mut self, amx: &Amx) {
        ensure_screen_model_registered(amx);

        if !AUDIO_SERVER_STARTED.swap(true, Ordering::SeqCst) {
            audio_server::start();
        }
    }

    fn process_tick(&mut self) {
        let Plugin {
            screens,
            network_budget,
            tick_priority_offset,
            ..
        } = self;

        let screen_count = screens.len();
        if screen_count > 0 {
            let start = *tick_priority_offset % screen_count;
            for offset in 0..screen_count {
                let index = (start + offset) % screen_count;
                if let Some(screen) = screens[index].as_mut() {
                    if let Some(amx) = get_amx(screen.amx_ident()) {
                        match screen.tick(&amx, network_budget) {
                            Ok(_) => {}
                            Err(e) => {
                                log::error!("Error ticking screen {}: {:?}", index, e);
                            }
                        };
                    }
                }
            }
            *tick_priority_offset = start + 1;
        }
    }
}

initialize_plugin!(
    natives: [
        Plugin::create_3d_media_screen,
        Plugin::destroy_3d_media_screen,
        Plugin::create_3d_media_screen_preview,
        Plugin::destroy_3d_media_screen_preview,
        Plugin::create_dialog_screen,
        Plugin::destroy_dialog_screen,
        Plugin::sva_area_listener_on_player_enter,
        Plugin::sva_area_listener_on_player_leave,
    ],
    {
        samp::plugin::enable_process_tick();

        let plugin = Plugin {
            screens: Vec::new(),
            placement_previews: Vec::new(),
            network_budget: NetworkBudget::new(NETWORK_BUDGET_RATE_PER_SEC, NETWORK_BUDGET_CAPACITY),
            tick_priority_offset: 0,
        };

        return plugin; // return the plugin into runtime
    }
);
