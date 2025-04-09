use thiserror::Error;

const ADDRESS_ARGON: u16 = 0x1a;
const ADDRESS_ARGON_FAN: u8 = 0x80;


#[derive(Debug, Error)]
pub enum ArgonDeviceError {
    #[error("I2C bus error")]
    I2C(#[from] nix::errno::Errno),
    #[error("I2c IO error")]
    Io(#[from] std::io::Error)
}

impl From<i2cdev::linux::LinuxI2CError> for ArgonDeviceError {
    fn from(value: i2cdev::linux::LinuxI2CError) -> Self {
        match value {
            i2cdev::linux::LinuxI2CError::Errno(no) => {
                let e = nix::errno::Errno::from_raw(no);
                ArgonDeviceError::I2C(e)
            }
            i2cdev::linux::LinuxI2CError::Io(io) => ArgonDeviceError::Io(io)
        }
    }
}

pub struct ArgonDevice {
    i2c: i2cdev::linux::LinuxI2CDevice,
}

impl ArgonDevice {
    pub fn new(bus_path: impl AsRef<std::path::Path>) -> Result<Self, ArgonDeviceError> {
        let dev = i2cdev::linux::LinuxI2CDevice::new(bus_path, ADDRESS_ARGON)
            .map_err(ArgonDeviceError::from)?;

        let s = Self {
            i2c: dev
        };

        Ok(s)
    }

    #[inline]
    pub fn fan_speed(&mut self) -> Result<u8, ArgonDeviceError> {
        use i2cdev::core::I2CDevice;
        self.i2c.smbus_read_byte_data(ADDRESS_ARGON_FAN).map_err(ArgonDeviceError::from)
    }

    #[inline]
    pub fn set_fan_speed(&mut self, value: u8) -> Result<(), ArgonDeviceError> {
        use i2cdev::core::I2CDevice;
        self.i2c.smbus_write_byte_data(ADDRESS_ARGON_FAN, value).map_err(ArgonDeviceError::from)
    }
}