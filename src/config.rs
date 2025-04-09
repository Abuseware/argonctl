use clap::Parser;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(transparent)]
    Io(#[from]std::io::Error),
    #[error(transparent)]
    Serializer(#[from]toml::ser::Error),
    #[error(transparent)]
    Deserializer(#[from]toml::de::Error),
}

#[derive(Parser, Serialize, Deserialize, Debug)]
struct ConfigOverlay {
    /// Low temperature treshold
    #[arg(long)]
    temp_low: Option<f32>,
    /// High temperature treshold
    #[arg(long)]
    temp_high: Option<f32>,
    /// Use logarithmic scaling instead of linear
    #[arg(long, default_missing_value = "true", num_args=0..=1)]
    log_scale: Option<bool>,
    /// Forking to background with dropped privileges
    #[arg(short, long, default_value_t = false)]
    #[serde(skip)]
    daemon: bool,
    /// User for dropped privileges
    #[arg(short, long)]
    uid: Option<Box<str>>,
    /// Log file
    #[arg(short, long)]
    log: Option<Box<str>>,
    /// Configuration file
    #[arg(short, long)]
    #[serde(skip)]
    config: Option<Box<str>>,
}

#[derive(Debug)]
pub struct Config {
    temp_low: f32,
    temp_high: f32,
    log_scale: bool,
    uid: Box<str>,
    log: Box<str>,
    overlay_cli: ConfigOverlay,
    overlay_cfg: Option<ConfigOverlay>,
}

impl Default for Config {
    fn default() -> Self {
        let args = ConfigOverlay::parse();
        Self {
            temp_low: 35.0,
            temp_high: 65.0,
            log_scale: false,
            uid: "nobody".into(),
            log: "/var/log/argond.log".into(),
            overlay_cli: args,
            overlay_cfg: None
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let mut params = Self::default();

        if let Some(path) = params.overlay_cli.config.clone() {
            params.overlay_cfg = if let Ok(data) = std::fs::read_to_string(path.as_ref()) {
                let cfg = toml::from_str(&data)?;
                Some(cfg)
            } else {
                None
            };
        }
        params.set_temp_low(params.temp_low);
        params.set_temp_high(params.temp_high);
        Ok(params)
    }

    fn flatten(&self) -> ConfigOverlay {
        ConfigOverlay {
            temp_low: Some(self.temp_low()),
            temp_high: Some(self.temp_high()),
            log_scale: Some(self.log_scale()),
            daemon: self.daemon(),
            uid: Some(self.uid()),
            log: Some(self.log()),
            config: self.overlay_cli.config.clone(),
        }
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let Some(path) = self.overlay_cli.config.clone() else {
            return Ok(());
        };

        log::debug!("Saving config to: {path}");

        let cfg = toml::ser::to_string_pretty(&self.flatten())?;
        std::fs::write(path.as_ref(), cfg.as_bytes())?;
        Ok(())
    }

    pub fn temp_low(&self) -> f32 {
        if let Some(cfg) = self.overlay_cli.temp_low {
            cfg
        } else if let Some(cfg) = self.overlay_cfg.as_ref().and_then(|v| v.temp_low) {
            cfg
        } else {
            self.temp_low
        }
    }

    pub fn set_temp_low(&mut self, low: f32) {
        self.overlay_cli.temp_low = Some(low.clamp(0.0, self.temp_high()));
    }

    pub fn temp_high(&self) -> f32 {
        if let Some(cfg) = self.overlay_cli.temp_high {
            cfg
        } else if let Some(cfg) = self.overlay_cfg.as_ref().and_then(|v| v.temp_high) {
            cfg
        } else {
            self.temp_high
        }
    }

    pub fn set_temp_high(&mut self, high: f32) {
        self.overlay_cli.temp_high = Some(high.max(self.temp_low() + 1.0));
    }

    pub fn temp_range(&self) -> f32 {
        self.temp_high() - self.temp_low()
    }

    pub fn log_scale(&self) -> bool {
        if let Some(cfg) = self.overlay_cli.log_scale {
            cfg
        } else if let Some(cfg) = self.overlay_cfg.as_ref().and_then(|v| v.log_scale) {
            cfg
        } else {
            self.log_scale
        }
    }

    pub fn set_log_scale(&mut self, log: bool) {
        self.overlay_cli.log_scale = Some(log)
    }

    pub fn daemon(&self) -> bool {
        self.overlay_cli.daemon
    }

    pub fn uid(&self) -> Box<str> {
        if let Some(cfg) = self.overlay_cli.uid.clone() {
            cfg
        } else if let Some(cfg) = self.overlay_cfg.as_ref().and_then(|v| v.uid.clone()) {
            cfg
        } else {
            self.uid.clone()
        }
    }

    pub fn log(&self) -> Box<str> {
        if let Some(cfg) = self.overlay_cli.log.clone() {
            cfg
        } else if let Some(cfg) = self.overlay_cfg.as_ref().and_then(|v| v.log.clone()) {
            cfg
        } else {
            self.log.clone()
        }
    }
}