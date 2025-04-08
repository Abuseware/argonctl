use smol::lock::Mutex;
use clap::Parser;

#[derive(Parser, Debug, Clone)]
pub struct Config {
    /// Low temperature treshold
    #[arg(long, default_value_t = 35.0)]
    pub temp_low: f32,
    /// High temperature treshold
    #[arg(long, default_value_t = 65.0)]
    pub temp_high: f32,
    /// Use logarithmic scaling instead of linear
    #[arg(long)]
    pub log_scale: bool,
    /// Forking to background with dropped privileges
    #[arg(short, long, default_value_t = false)]
    pub daemon: bool,
    /// User for dropped privileges
    #[arg(short, long, default_value = "nobody")]
    pub uid: Box<str>,
    /// Log file
    #[arg(short, long, default_value = "/var/log/argond.log")]
    pub log: Box<str>,
}

pub static ARGS: std::sync::LazyLock<Mutex<Config>> = std::sync::LazyLock::new(|| {
    let mut cfg = Config::parse();
    cfg.temp_low = cfg.temp_low.clamp(0.0, cfg.temp_high.max(0.0));
    cfg.temp_high = cfg.temp_high.max(cfg.temp_low);
    log::debug!("Args: {:#?}", cfg);
    Mutex::new(cfg)

});

pub static KILLSWITCH: std::sync::OnceLock<smol::channel::Sender<()>> = std::sync::OnceLock::new();

pub struct RpcController;

#[zbus::interface(name = "xyz.abuseware.argond1", proxy(default_path = "/xyz/abuseware/Argond", default_service = "xyz.abuseware.argond"))]
impl RpcController {
    async fn set_low(&self, celcius: f32) -> f32 {
        let mut args = ARGS.lock().await;
        let celcius_clamp = celcius.clamp(0.0, args.temp_high);
        args.temp_low = celcius_clamp;
        log::debug!("Setting low temp to {celcius_clamp}, requested {celcius}");
        celcius_clamp
    }

    async fn set_high(&self, celcius: f32) -> f32 {
        let mut args = ARGS.lock().await;
        let celcius_clamp = celcius.max(args.temp_low);
        args.temp_high = celcius_clamp;
        log::debug!("Setting high temp to {celcius_clamp}, requested {celcius}");
        celcius_clamp
    }

    async fn set_log_scale(&self, log: bool) -> bool {
        ARGS.lock().await.log_scale = log;
        log::debug!("Setting log scale: {log}");
        true
    }

    async fn exit(&self) -> bool {
        if let Some(ks) = KILLSWITCH.get() {
           return ks.send(()).await.is_ok();
        }
        false
    }
}