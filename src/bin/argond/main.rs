mod device;

use std::sync::Arc;
use smol::prelude::*;
use smol::lock::Mutex;
use argonctl::config::Config;
use argonctl::DbusController;
use crate::device::{ArgonDevice, ArgonDeviceError};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let config = Config::load()?;

    let device = ArgonDevice::new("/dev/i2c-1")?;
    let device =  Arc::new(Mutex::new(device));

    {
        if config.daemon() {
            let log = std::fs::File::create(config.log().as_ref())?;
            //let err = std::fs::File::create("argond.err").unwrap();
            if let Err(e) = daemonize::Daemonize::new()
                .user(config.uid().as_ref())
                .working_directory("/")
                .stderr(log)
                .start() {
                panic!("Cannot daemonize: {e}")
            }
        }
    }

    let config = Arc::new(Mutex::new(config));

    let (killswitch_tx, killswitch_rx) = smol::channel::bounded::<()>(10);

    let ctrlc_ks = killswitch_tx.clone();
    ctrlc::set_handler(move || {
        if ctrlc_ks.send_blocking(()).is_err() {
            std::process::exit(1);
        };
    })?;

    let executor = smol::LocalExecutor::new();

    smol::future::block_on(executor.run(async {

        if let Ok(rpc) = zbus::Connection::system().await {
            if let Ok(ctl) = argonctl::DbusControllerProxy::new(&rpc).await {
                if ctl.ping().await.is_ok_and(|v| v) {
                    log::error!("Daemon already running!");
                    std::process::exit(1);
                }
            }
        }

        let _rpc = match rpc(config.clone(), killswitch_tx.clone()).await {
            Ok(rpc) => Some(rpc),
            Err(e) => {
                log::warn!("Cannot create RPC: {e}");
                None
            }
        };
        smol::spawn(fan_task(config.clone(), device.clone())).detach();

        let _ = killswitch_rx.recv().await;
        log::info!("Shutting down...");
        device.lock().await.set_fan_speed(100).unwrap();
    }));

    if let Err(e) = config.lock_blocking().save() {
        log::warn!("Cannot save config: {e}");
    }

    Ok(())
}

async fn rpc(config: Arc<Mutex<Config>>, kill_signal: smol::channel::Sender<()>) -> Result<zbus::Connection, zbus::Error> {
    let ctl = DbusController::new(config, kill_signal);
    zbus::connection::Builder::system()?
        .name("xyz.abuseware.argond")?
        .serve_at("/xyz/abuseware/Argond", ctl)?
        .build()
        .await
}

async fn fan_task(config: Arc<Mutex<Config>>, device: Arc<Mutex<ArgonDevice>>) -> Result<(), ArgonDeviceError> {
    device.lock().await.set_fan_speed(0)?;
    let mut iv = smol::Timer::interval(std::time::Duration::from_millis(200));
    let mut samples = Vec::with_capacity(50);
    let mut last_avg = read_temp_pi().await;
    let mut idle_timer: Option<std::time::Instant> = None;
    loop {
        let temp_pi = read_temp_pi().await;
        let temp_ssd = read_temp_ssd().await;
        let temp = temp_pi.max(temp_ssd);
        samples.push(temp);

        if samples.len() >= 5 {
            let temp_avg = samples.iter().sum::<f32>() / samples.len() as f32;
            samples.clear();

            let delta = temp_avg - last_avg;

            let delta_trigger = delta >= 0.5 || delta <= -5.0;
            let timer_trigger = idle_timer.as_ref().is_some_and(|t| t.elapsed() >= std::time::Duration::from_secs(5));

            if delta_trigger || timer_trigger {
                log::debug!("Reason: delta: {delta_trigger}, timer: {timer_trigger}");
                idle_timer = None;
                let config = config.lock().await;
                let speed = calc_speed(&config, temp_avg).await.round() as u8;
                let mut device = device.lock().await;
                if speed != device.fan_speed()? {
                    log::debug!("Setting speed to {speed}%, temp sampled {temp_avg:.2}°C; Current pi: {temp_pi:.2}°C{}", if temp_ssd > 0.0 {format!(", ssd: {temp_ssd:.2}°C")} else {"".to_string()});
                    device.set_fan_speed(speed)?;
                }
                last_avg = temp_avg;
            } else if idle_timer.is_none() {
                last_avg = temp_avg;
                idle_timer = Some(std::time::Instant::now())
            }
        }
        iv.next().await;
    }
}

async fn calc_speed(config: &Config, temperature: f32) -> f32 {
    if config.log_scale() { calc_speed_log(config, temperature).await } else { calc_speed_linear(config, temperature).await }
}

async fn calc_speed_log(config: &Config, temperature: f32) -> f32 {
    let temp_rel = (temperature - config.temp_low()).clamp(0.0, config.temp_range()) + 1.0;
    let log_temp_rel = temp_rel.log2();
    let log_temp_rel_min = 1.0f32.log2();
    let log_temp_rel_max = (config.temp_range() + 1.0).log2();
    ((log_temp_rel - log_temp_rel_min) / (log_temp_rel_max - log_temp_rel_min)) * 100.0
}

async fn calc_speed_linear(config: &Config, temperature: f32) -> f32 {
    let temp_clamped = temperature.clamp(config.temp_low(), config.temp_high());
    ((temp_clamped - config.temp_low()) / config.temp_range()) * 100.0
}

async fn read_temp_pi() -> f32 {
    let data = smol::fs::read_to_string("/sys/class/thermal/thermal_zone0/temp").await.unwrap();
    data.trim().parse::<f32>().unwrap_or(f32::NAN) / 1000.0
}

async fn read_temp_ssd() -> f32 {
    if let Ok(data) = smol::fs::read_to_string("/sys/class/nvme/nvme0/hwmon1/temp1_input").await {
        let ssd_temp = data.trim().parse::<f32>().unwrap_or(f32::NAN) / 1000.0;
        ssd_temp
    } else {
        0.0
    }
}
