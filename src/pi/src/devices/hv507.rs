use log::*;
use serde::Deserialize;

use rppal::gpio::{Gpio, Level, OutputPin, Pin};
use rppal::pwm::{self, Pwm};

use crate::{Error, Result};

const N_PINS: usize = 128;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub frequency: f64,
    pub duty_cycle: f64,
    pub pins: Pins,
    pub default_polarity: DefaultLevel,
}

#[derive(Debug, Deserialize)]
pub struct Pins {
    pub blank: u8,
    pub latch_enable: u8,
    pub clock: u8,
    pub data: u8,
    pub polarity_pwm_channel: u8,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DefaultLevel {
    Low,
    High,
}

impl Settings {
    pub fn make(&self) -> Result<Hv507> {
        trace!("Initializing pi gpio...");
        let gpio = Gpio::new()?;

        // by default, these pins will be set to low on drop
        let mk_output = |pin| {
            trace!("initializing pin {}...", pin);
            gpio.get(pin).map(Pin::into_output)
        };

        let polarity = Pwm::new(match self.pins.polarity_pwm_channel {
            0 => pwm::Channel::Pwm0,
            1 => pwm::Channel::Pwm1,
            n => return Err(Error::InvalidPwmChannel(n)),
        })?;

        polarity.set_polarity(match self.default_polarity {
            DefaultLevel::Low => pwm::Polarity::Normal,
            DefaultLevel::High => pwm::Polarity::Inverse,
        })?;

        let mut hv = Hv507 {
            blank: mk_output(self.pins.blank)?,
            latch_enable: mk_output(self.pins.latch_enable)?,
            clock: mk_output(self.pins.clock)?,
            data: mk_output(self.pins.data)?,
            pins: [Level::Low; N_PINS],
            polarity,
        };

        hv.init(self)?;

        trace!("init complete!");
        Ok(hv)
    }
}

pub struct Hv507 {
    blank: OutputPin,
    latch_enable: OutputPin,
    clock: OutputPin,
    data: OutputPin,
    polarity: Pwm,

    pins: [Level; N_PINS],
}

impl Hv507 {
    pub fn n_pins(&self) -> usize {
        self.pins.len()
    }

    fn init(&mut self, settings: &Settings) -> Result<()> {
        // setup the HV507 for serial data write
        // see row "LOAD S/R" in table 3-2 in
        // http://ww1.microchip.com/downloads/en/DeviceDoc/20005845A.pdf

        self.blank.set_high();
        self.latch_enable.set_low();
        self.clock.set_low();
        self.data.set_low();

        // now call the public function to set the HV507 polarity pin
        self.set_polarity(settings.frequency, settings.duty_cycle)?;

        Ok(())
    }

    pub fn set_polarity(&mut self, frequency: f64, duty_cycle: f64) -> Result<()> {
        self.polarity.set_frequency(frequency, duty_cycle)?;
        self.polarity.enable()?;
        Ok(())
    }

    pub fn clear_pins(&mut self) {
        for pin in self.pins.iter_mut() {
            *pin = Level::Low;
        }
    }

    pub fn set_pin(&mut self, pin: usize, value: bool) {
        use Level::*;
        self.pins[pin] = if value { High } else { Low };
    }

    pub fn set_pin_hi(&mut self, pin: usize) {
        self.set_pin(pin, true)
    }

    pub fn set_pin_lo(&mut self, pin: usize) {
        self.set_pin(pin, false)
    }

    pub fn shift_and_latch(&mut self) {
        for pin in self.pins.iter() {
            // write and cycle the clock
            self.data.write(*pin);
            self.clock.set_high();
            self.clock.set_low();
        }

        // commit the latch
        self.latch_enable.set_high();
        self.latch_enable.set_low();
    }
}

impl Drop for Hv507 {
    fn drop(&mut self) {
        debug!("Cleaning up HV507");
        self.clear_pins();
        self.shift_and_latch();
    }
}
