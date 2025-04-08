use smol::prelude::*;
use i2cdev::core::*;
use i2cdev::linux::LinuxI2CError;
use argonctl::{RpcController, ARGS, KILLSWITCH};

static FAN: std::sync::LazyLock<std::sync::Mutex<i2cdev::linux::LinuxI2CDevice>> = std::sync::LazyLock::new(|| {
    let fan = match i2cdev::linux::LinuxI2CDevice::new("/dev/i2c-1", 0x1a) {
        Ok(fan) => fan,
        Err(LinuxI2CError::Errno(no)) => {
            let e = nix::errno::Errno::from_raw(no);
            panic!("{e}");
        }
        Err(LinuxI2CError::Io(io)) => {
            panic!("{io}");
        }
    };

    std::sync::Mutex::new(fan)
});

fn main() {
    env_logger::init();

    FAN.try_lock().ok();
    //let executor = smol::LocalExecutor::new();
    {
        let args = ARGS.lock_blocking();
        if args.daemon {
            let log = std::fs::File::create(args.log.as_ref()).unwrap();
            log.set_len(0).unwrap();
            //let err = std::fs::File::create("argond.err").unwrap();
            if let Err(e) = daemonize::Daemonize::new()
                .user(args.uid.as_ref())
                .working_directory("/")
                .stderr(log)
                .start() {
                panic!("Cannot daemonize: {e}")
            }
        }
    }

    let (killswitch_tx, killswitch_rx) = smol::channel::bounded::<()>(10);

    KILLSWITCH.set(killswitch_tx.clone()).unwrap();

    ctrlc::set_handler(move || {
        log::info!("Exiting!");
        if killswitch_tx.send_blocking(()).is_err() {
            std::process::exit(1);
        };
    }).unwrap();

    //smol::future::block_on(executor.run(fan_task()));
    smol::block_on(async {
        let _rpc = match rpc().await {
            Ok(rpc) => Some(rpc),
            Err(e) => {
                log::warn!("Cannot create RPC: {e}");
                None
            }
        };
        fan_task(killswitch_rx).await;
    });

}

async fn rpc() -> Result<zbus::Connection, zbus::Error> {
    zbus::connection::Builder::system()?
        .name("xyz.abuseware.argond")?
        .serve_at("/xyz/abuseware/Argond", RpcController)?
        .build()
        .await
}

async fn fan_task(killswitch: smol::channel::Receiver<()>) {
    let mut iv = smol::Timer::interval(std::time::Duration::from_millis(500));
    loop {
        if let Ok(_) = killswitch.try_recv() {
            log::info!("Shutting down...");
            set_fan_speed(0);
            break;
        }
        let temp = read_temp().await;
        let percent = if ARGS.lock().await.log_scale { calc_speed_log(temp).await } else { calc_speed_linear(temp).await };
        let speed = percent.round() as u8;
        if speed != fan_speed() {
            log::debug!("Setting speed to {speed}%, temp {temp}°C");
            if !set_fan_speed(speed) {
                log::error!("Cannot lock i2c");
            }
        }
        iv.next().await;
    }
}

async fn calc_speed_log(temperature: f32) -> f32 {
    let args = ARGS.lock().await;
    let temp_range = args.temp_high - args.temp_low;
    let temp_rel = (temperature - args.temp_low).clamp(0.0, temp_range) + 1.0;
    let log_temp_rel = temp_rel.log10();
    let log_temp_rel_min = 1.0f32.log10();
    let log_temp_rel_max = (temp_range + 1.0).log10();
    ((log_temp_rel - log_temp_rel_min) / (log_temp_rel_max - log_temp_rel_min)) * 100.0
}

async fn calc_speed_linear(temperature: f32) -> f32 {
    let args = ARGS.lock().await;
    let temp_range = args.temp_high - args.temp_low;
    let temp_clamped = temperature.clamp(args.temp_low, args.temp_high);
    ((temp_clamped - args.temp_low) / temp_range) * 100.0
}

#[inline]
fn fan_speed() -> u8 {
    FAN.lock().ok().and_then(|mut f| f.smbus_read_byte_data(0x80).ok()).unwrap_or_default()
}

#[inline]
fn set_fan_speed(speed: u8) -> bool {
    FAN.lock().ok().and_then(|mut f| f.smbus_write_byte_data(0x80, speed).ok()).is_some()
}

async fn read_temp() -> f32 {
    let data = smol::fs::read_to_string("/sys/class/thermal/thermal_zone0/temp").await.unwrap();
    data.trim().parse::<f32>().unwrap_or(f32::NAN) / 1000.0
}
