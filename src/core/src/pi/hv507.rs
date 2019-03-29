use std::thread;
use std::time::Duration;

use rppal::gpio::{Gpio, OutputPin, Level, IoPin, Mode};
use rppal::pwm::{Pwm, Channel, Polarity};

use crate::util::seconds_duration;

use super::Result;
use crate::grid::parse::PolarityConfig;

const N_PINS: usize = 128;

const ALL_ZEROS: &[Level] = &[Level::Low; N_PINS];


// physical pin 11
pub const BLANK_PIN: u8 = 17;
// physical pin 13
pub const LATCH_ENABLE_PIN: u8 = 27;
// physical pin 15
pub const CLOCK_PIN: u8 = 22;
// physical pin 16
pub const DATA_PIN: u8 = 23;

pub struct Hv507 {
    blank: OutputPin,
    latch_enable: OutputPin,
    clock: OutputPin,
    data: OutputPin,

    polarity_gpio: IoPin,
    polarity: Pwm,

    pins: [Level; N_PINS],
}

impl Hv507 {
    pub fn new(gpio: &Gpio) -> Result<Hv507> {
        // by default, these pins will be set to low on drop

        macro_rules! mk_output {
            ($pin:expr) => {{
                let pin = $pin;
                trace!("initializing pin {}...", pin);
                gpio.get(pin)?.into_output() 
            }}
        }

        // let mut blank = mk_output!(BLANK_PIN);
        // thread::spawn(move || {
        //     let dur = seconds_duration(1.0 / 500.0);
        //     loop {
        //         thread::sleep(dur);
        //         blank.toggle();
        //     }
        // });

        let hv = Hv507 {
            blank: mk_output!(BLANK_PIN),
            latch_enable: mk_output!(LATCH_ENABLE_PIN),
            clock: mk_output!(CLOCK_PIN),
            data: mk_output!(DATA_PIN),
            polarity_gpio: gpio.get(12)?.into_io(Mode::Alt0),
            polarity: {
                trace!("initializing pwm0...");
                // physical pin 32, bcm 12 (pwm0)
                Pwm::new(Channel::Pwm0)?
                // trace!("initializing pwm1...");
                // // physical pin 33, bcm 13 (pwm1)
                // Pwm::new(Channel::Pwm1)?
            },
            pins: [Level::Low; N_PINS],
        };

        trace!("init complete!");
        Ok(hv)
    }

    pub fn init(&mut self, config: &PolarityConfig) -> Result<()> {
        // setup the HV507 for serial data write
        // see row "LOAD S/R" in table 3-2 in
        // http://ww1.microchip.com/downloads/en/DeviceDoc/20005845A.pdf

        self.blank.set_high();
        self.latch_enable.set_low();
        self.clock.set_low();
        self.data.set_low();

        // make sure the active state is set to high
        self.polarity.set_polarity(Polarity::Normal);

        // now call the public function to set the HV507 polarity pin
        self.set_polarity(config);

        Ok(())
    }

    pub fn set_polarity(&mut self, config: &PolarityConfig) -> Result<()> {
        self.polarity.set_frequency(config.frequency, config.duty_cycle)?;
        self.polarity.enable()?;
        Ok(())
    }

    pub fn clear_pins(&mut self) {
        for pin in self.pins.iter_mut() {
            *pin = Level::Low;
        }
    }

    pub fn set_pin_hi(&mut self, pin: usize) {
        self.pins[pin] = Level::High;
    }

    pub fn set_pin_lo(&mut self, pin: usize) {
        self.pins[pin] = Level::Low;
    }

    pub fn shift_and_latch(&mut self) {
        // self.do_one_shift_and_latch(ALL_ZEROS);
        // let duration = Duration::from_millis(10);
        // thread::sleep(duration);
        //FIXME dumb clone
        let pins = self.pins.clone();
        self.do_one_shift_and_latch(&pins);
    }

    fn do_one_shift_and_latch(&mut self, pins: &[Level]) {
        for pin in pins.iter() {
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
