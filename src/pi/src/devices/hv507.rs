use log::*;
use serde::Deserialize;

use rppal::gpio::{Gpio, IoPin, Level, OutputPin, Pin};
use rppal::pwm::{self, Pwm};

use crate::{Error, Result};

const N_PINS: usize = 128;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub frequency: f64,
    pub duty_cycle: f64,
    pub blank_pin: u8,
    pub latch_enable_pin: u8,
    pub clock_pin: u8,
    pub data_pin: u8,
    pub pwm_pin: u8,
}

fn mk_pwm(gpio: &Gpio, pin: u8) -> Result<(IoPin, Pwm)> {
    use rppal::gpio::Mode::*;
    use rppal::pwm::Channel::*;
    let (chan, alt) = match pin {
        12 => (Pwm0, Alt0),
        13 => (Pwm1, Alt0),
        18 => (Pwm0, Alt5),
        19 => (Pwm1, Alt5),
        _ => return Err(Error::InvalidPwmChannel(pin)),
    };
    info!(
        "Initializing hv507 pwm - pin={}, chan={:?}, alt={:?}",
        pin, chan, alt
    );
    let io_pin = gpio.get(pin)?.into_io(alt);
    let pwm = Pwm::new(chan)?;
    Ok((io_pin, pwm))
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

        let (polarity_gpio, polarity) = mk_pwm(&gpio, self.pwm_pin)?;

        let mut hv = Hv507 {
            blank: mk_output(self.blank_pin)?,
            latch_enable: mk_output(self.latch_enable_pin)?,
            clock: mk_output(self.clock_pin)?,
            data: mk_output(self.data_pin)?,
            pins: [Level::Low; N_PINS],
            polarity_gpio,
            polarity,
            gpio,
        };

        hv.init(self)?;

        trace!("init complete!");
        Ok(hv)
    }
}

#[allow(dead_code)]
pub struct Hv507 {
    gpio: Gpio,
    blank: OutputPin,
    latch_enable: OutputPin,
    clock: OutputPin,
    data: OutputPin,

    polarity_gpio: IoPin,
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

        // make sure the active state is set to high
        self.polarity.set_polarity(pwm::Polarity::Normal)?;

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
