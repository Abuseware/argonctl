use clap::Parser;

#[derive(Parser, Debug)]
pub struct Config {
    /// Low temperature treshold
    #[arg(long)]
    pub temp_low: Option<u8>,
    /// High temperature treshold
    #[arg(long)]
    pub temp_high: Option<u8>,
    /// Use linear scaling instead of logarithmic
    #[arg(long)]
    pub log_scale: Option<bool>,
    /// Exit daemon
    #[arg(long)]
    pub exit: bool
}

fn main() -> Result<(), zbus::Error> {
    let cfg = Config::parse();
    let conn = zbus::blocking::Connection::system()?;
    let ctl = argonctl::DbusControllerProxyBlocking::new(&conn)?;

    if !ctl.ping().unwrap_or_default() {
        eprintln!("Daemon is not running");
        return Ok(())
    }

    if cfg.exit {
        if ctl.exit()? {
            println!("Daemon shutting down!");
            return Ok(());
        }
    }

    let current_low = ctl.low()?;
    let current_high = ctl.high()?;
    let current_log = ctl.log_scale()?;

    println!("[Current config]");
    println!("Low:\t{current_low}°C");
    println!("High:\t{current_high}°C");
    println!("Scale:\t{}", if current_log {"logarithmic"} else {"linear"});
    println!("----------------");

    if let Some(low) = cfg.temp_low {
        let new_low = ctl.set_low(low as f32)?;
        println!("New low: {new_low}°C");
    }

    if let Some(high) = cfg.temp_high {
        let new_high = ctl.set_high(high as f32)?;
        println!("New high: {new_high}°C");
    }

    if let Some(log) = cfg.log_scale {
        if ctl.set_log_scale(log)? == true {
            println!("Logarithmic scale set to: {log}");
        }
    }

    Ok(())
}