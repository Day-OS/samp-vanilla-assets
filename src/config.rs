use std::fs;
use std::sync::OnceLock;

use log::{info, warn};
use serde::{Deserialize, Serialize};

/// Path is relative to the server's working directory (where omp-server.exe
/// runs), same as `models/` and `config.json`.
pub const CONFIG_PATH: &str = "SVA_Config.toml";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub network: NetworkConfig,
    pub audio: AudioConfig,
    pub screen_model: ScreenModelConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            network: NetworkConfig::default(),
            audio: AudioConfig::default(),
            screen_model: ScreenModelConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct NetworkConfig {
    /// Token-bucket refill rate for the shared object-update budget (see
    /// `network_budget.rs`) - tune this while watching the server log for
    /// "client exceeded 'ackslimit'" warnings.
    pub budget_rate_per_sec: f64,
    /// Burst allowance on top of the refill rate - deliberately small
    /// relative to the rate, see the comment on `NETWORK_BUDGET_CAPACITY` in
    /// `constants.rs` for why.
    pub budget_capacity: f64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        NetworkConfig {
            budget_rate_per_sec: 2800.0,
            budget_capacity: 60.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct AudioConfig {
    /// Bind address for the internal HTTP server that relays extracted
    /// audio clips to players (`0.0.0.0:PORT` to accept external
    /// connections, change the port if it collides with something else on
    /// the box).
    pub server_bind: String,
    /// Where extracted audio clips are cached on disk, relative to the
    /// server's working directory.
    pub output_dir: String,
    /// Base URL players' game clients use to fetch audio clips from the
    /// server above - update this if the server sits behind a reverse
    /// proxy/different public hostname or port than `server_bind`.
    pub base_url: String,
}

impl Default for AudioConfig {
    fn default() -> Self {
        AudioConfig {
            server_bind: "0.0.0.0:7878".to_string(),
            output_dir: "samp-led/audio_cache".to_string(),
            base_url: "http://127.0.0.1:7878".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ScreenModelConfig {
    /// Custom model ID used for the standard screen object - must not
    /// collide with any other custom/base GTA model ID in use.
    pub standard_model_id: i32,
    /// Custom model ID used for the screen's shadow variant.
    pub shadow_model_id: i32,
    /// Base GTA model ID the standard screen model replaces/extends.
    pub standard_base_model_id: i32,
    /// Base GTA model ID the shadow screen model replaces/extends.
    pub shadow_base_model_id: i32,
    /// DFF/TXD file names (under the server's `models/` folder) for the
    /// standard screen model.
    pub standard_dff: String,
    pub standard_txd: String,
    /// DFF/TXD file names for the shadow screen model.
    pub shadow_dff: String,
    pub shadow_txd: String,
}

impl Default for ScreenModelConfig {
    fn default() -> Self {
        ScreenModelConfig {
            standard_model_id: -1003,
            shadow_model_id: -1004,
            standard_base_model_id: 19805,
            shadow_base_model_id: 19806,
            standard_dff: "screen.dff".to_string(),
            standard_txd: "screen.txd".to_string(),
            shadow_dff: "screen-shadow.dff".to_string(),
            shadow_txd: "screen-shadow.txd".to_string(),
        }
    }
}

static CONFIG: OnceLock<Config> = OnceLock::new();

/// Loads `SVA_Config.toml` on first call (creating it with defaults if it
/// doesn't exist yet) and returns the same parsed config on every call after
/// that - the file is only read once per server run.
pub fn get() -> &'static Config {
    CONFIG.get_or_init(load)
}

fn load() -> Config {
    match fs::read_to_string(CONFIG_PATH) {
        Ok(text) => match toml::from_str(&text) {
            Ok(config) => {
                info!("Loaded {}", CONFIG_PATH);
                config
            }
            Err(err) => {
                warn!(
                    "Failed to parse {}: {} - falling back to defaults",
                    CONFIG_PATH, err
                );
                Config::default()
            }
        },
        Err(_) => {
            let config = Config::default();
            match toml::to_string_pretty(&config) {
                Ok(text) => match fs::write(CONFIG_PATH, text) {
                    Ok(()) => info!(
                        "{} not found - created with default values, edit it to customize the plugin",
                        CONFIG_PATH
                    ),
                    Err(err) => warn!("Failed to create {}: {}", CONFIG_PATH, err),
                },
                Err(err) => warn!("Failed to serialize default {}: {}", CONFIG_PATH, err),
            }
            config
        }
    }
}
