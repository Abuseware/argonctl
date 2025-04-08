use clap::Parser;

#[derive(Parser, Debug)]
#[group(required = true)]
pub struct Config {
    /// Low temperature treshold
    #[arg(long)]
    pub temp_low: Option<f32>,
    /// High temperature treshold
    #[arg(long)]
    pub temp_high: Option<f32>,
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
    let ctl = argonctl::RpcControllerProxyBlocking::new(&conn)?;

    if cfg.exit {
        if ctl.exit()? {
            println!("Daemon shutting down!");
            return Ok(());
        }
    }

    if let Some(low) = cfg.temp_low {
        let new_low = ctl.set_low(low)?;
        println!("New low: {new_low}°C");
    }

    if let Some(high) = cfg.temp_high {
        let new_high = ctl.set_high(high)?;
        println!("New high: {new_high}°C");
    }

    if let Some(log) = cfg.log_scale {
        if ctl.set_log_scale(log)? == true {
            println!("Logarithmic scale set to: {log}");
        }
    }

    Ok(())
}