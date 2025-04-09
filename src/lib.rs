use std::sync::Arc;
use smol::lock::Mutex;
use crate::config::Config;

pub mod config;

pub struct DbusController {
    config: Arc<Mutex<Config>>,
    kill_signal: smol::channel::Sender<()>
}

impl DbusController {
    pub fn new(config: Arc<Mutex<Config>>, kill_signal: smol::channel::Sender<()>) -> Self {
        Self {
            config,
            kill_signal
        }
    }
}

#[zbus::interface(name = "xyz.abuseware.argond1", proxy(default_path = "/xyz/abuseware/Argond", default_service = "xyz.abuseware.argond"))]
impl DbusController {
    #[zbus(property)]
    async fn low(&self) -> f64 {
        self.config.lock().await.temp_low() as f64
    }

    #[zbus(property)]
    async fn high(&self) -> f64 {
        self.config.lock().await.temp_high() as f64
    }

    #[zbus(property)]
    async fn log_scale(&self) -> bool {
        self.config.lock().await.log_scale()
    }

    async fn set_low(&self, celsius: f32) -> f32 {
        let mut args = self.config.lock().await;
        args.set_temp_low(celsius);
        let r = args.temp_low();
        log::debug!("Setting low temp to {r}, requested {celsius}");
        r
    }

    async fn set_high(&self, celsius: f32) -> f32 {
        let mut args = self.config.lock().await;
        args.set_temp_high(celsius);
        let r = args.temp_high();
        log::debug!("Setting high temp to {r}, requested {celsius}");
        r
    }

    async fn set_log_scale(&self, log: bool) -> bool {
        self.config.lock().await.set_log_scale(log);
        log::debug!("Setting log scale: {log}");
        true
    }

    async fn exit(&self) -> bool {
            self.kill_signal.send(()).await.is_ok()
    }

    async fn ping(&self) -> bool {
        true
    }
}