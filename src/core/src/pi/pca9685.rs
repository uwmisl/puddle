use super::{I2cHandle, Result};
use std::thread::sleep;
use std::time::Duration;

// https://cdn-shop.adafruit.com/datasheets/PCA9685.pdf
#[allow(dead_code)]
enum Register {
    /// Mode register 1
    Mode1 = 0,
    /// Mode register 2
    Mode2 = 1,
    /// I2C-bus subaddress 1
    SubAdr1 = 2,
    /// I2C-bus subaddress 2
    SubAdr2 = 3,
    /// I2C-bus subaddress 3
    SubAdr3 = 4,
    /// LED All Call I2C-bus address
    AllCallAdr = 5,
    /// LED0_ON_L, the first LED register
    LedBase,
    /// prescaler for PWM output frequency
    PreScale = 254,
}

pub const PCA9685_DEFAULT_ADDRESS: u16 = 0x42;

const NUM_LEDS: u8 = 16;

/// There are 4 registers per LED:
/// LED*_ON_L, LED*_ON_H, LED*_OFF_L, LED*_OFF_H
const REGISTERS_PER_LED: u8 = 4;

pub const DUTY_CYCLE_MAX: u16 = 4095;

#[cfg_attr(rustfmt, rustfmt_skip)]
enum Mode1 {
    Restart       = 0b1000_0000,
    ExtClk        = 0b0100_0000,
    AutoIncrement = 0b0010_0000,
    Sleep         = 0b0001_0000,
    Sub1          = 0b0000_1000,
    Sub2          = 0b0000_0100,
    Sub3          = 0b0000_0010,
    AllCall       = 0b0000_0001,
}

impl From<Mode1> for u8 {
    fn from(x: Mode1) -> u8 {
        x as u8
    }
}

pub struct Pca9685 {
    i2c: I2cHandle,
}

impl Pca9685 {
    pub fn new(i2c: I2cHandle) -> Result<Pca9685> {
        let mut pca = Pca9685 { i2c };
        pca.init().map(|_| pca)
    }

    pub fn init(&mut self) -> Result<()> {
        self.i2c
            .write(&[Register::Mode1 as u8, Mode1::AutoIncrement as u8])
        // self.i2c.write(&[MODE2, OUTDRV]).unwrap();
    }

    fn write_reg(&mut self, reg: Register, data: impl Into<u8>) -> Result<()> {
        self.i2c.write(&[reg as u8, data.into()])
    }

    fn read_reg(&mut self, reg: Register) -> Result<u8> {
        self.i2c.write(&[reg as u8])?;
        let buf = self.i2c.read(1)?;
        Ok(buf[0])
    }

    pub fn reset(&mut self) -> Result<()> {
        self.write_reg(Register::Mode1, Mode1::Restart)?;
        sleep(Duration::from_millis(10));
        Ok(())
    }

    pub fn set_pwm_freq(&mut self, frequency: f64) -> Result<()> {
        debug!("Attempting to set pwm frequency to {}", frequency);

        // https://github.com/adafruit/Adafruit-PWM-Servo-Driver-Library/issues/11
        // freq *= 0.9;  // Correct for overshoot in the frequency setting

        // https://cdn-shop.adafruit.com/datasheets/PCA9685.pdf
        // Section 7.3.5. PWM frequency PRE_SCALE
        let oscillator_clock = 25e6; // 25 MHz
        let prescale = f64::round(oscillator_clock / (4096.0 * frequency)) - 1.0;
        assert!(0.0 < prescale);
        assert!(prescale <= ::std::u8::MAX.into());
        let prescale_u8 = prescale as u8;

        debug!("Prescale pwm: {}", prescale_u8);

        use self::Mode1::*;
        use self::Register::{Mode1, PreScale};

        let old_mode = self.read_reg(Mode1)?;
        let new_mode = (old_mode & !(Restart as u8)) | (Sleep as u8);
        self.write_reg(Mode1, new_mode)?;
        self.write_reg(PreScale, prescale_u8)?;
        self.write_reg(Mode1, old_mode)?;
        // self.write_reg(Mode1, old_mode | 0xa0)?;  //  This sets the MODE1 register to turn on auto increment.
        Ok(())
    }

    pub fn set_duty_cycle(&mut self, channel: u8, on_fraction: u16) -> Result<()> {
        assert!(channel < NUM_LEDS);
        assert!(on_fraction <= DUTY_CYCLE_MAX);

        // if on_fraction if min or max, use the special values of 0, DUTY_CYCLE_MAX + 1
        let special = DUTY_CYCLE_MAX + 1;
        let (on, off) = if on_fraction == 0 {
            (0, special)
        } else if on_fraction == DUTY_CYCLE_MAX {
            (special, 0)
        } else {
            (0, on_fraction)
        };

        let on_l = on as u8;
        let on_h = (on >> 8) as u8;
        let off_l = off as u8;
        let off_h = (off >> 8) as u8;

        self.i2c.write(&[
            Register::LedBase as u8 + (REGISTERS_PER_LED * channel),
            on_l,
            on_h,
            off_l,
            off_h,
        ])
    }
}

impl Drop for Pca9685 {
    fn drop(&mut self) {
        for chan in 0..NUM_LEDS {
            let res = self.set_duty_cycle(chan, 0);
            if res.is_err() {
                error!("Failed to shutdown pwm channel {}: {:#?}", chan, res);
            }
        }
    }
}
