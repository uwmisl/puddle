#[derive(Debug)]
pub enum Error {
    Gpio(rppal::gpio::Error),
    I2c(rppal::i2c::Error),
    Pwm(rppal::pwm::Error),
    Spi(rppal::spi::Error),
    InvalidPwmChannel(u8),
    Configuration(config::ConfigError),
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

macro_rules! impl_error {
    ($inner:ty, $variant:ident) => {
        impl From<$inner> for Error {
            fn from(inner: $inner) -> Self {
                Error::$variant(inner)
            }
        }
    };
}

impl_error!(rppal::gpio::Error, Gpio);
impl_error!(rppal::i2c::Error, I2c);
impl_error!(rppal::pwm::Error, Pwm);
impl_error!(rppal::spi::Error, Spi);
impl_error!(config::ConfigError, Configuration);

use std::fmt;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Gpio(inner) => write!(f, "{}", inner),
            Error::I2c(inner) => write!(f, "{}", inner),
            Error::Pwm(inner) => write!(f, "{}", inner),
            Error::Spi(inner) => write!(f, "{}", inner),
            Error::InvalidPwmChannel(chan) => write!(f, "Invalid PWM channel: {}", chan),
            Error::Configuration(inner) => write!(f, "{}", inner),
        }
    }
}
