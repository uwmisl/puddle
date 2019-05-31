#![allow(clippy::assertions_on_constants)]
// https://datasheets.maximintegrated.com/en/ds/MAX31865.pdf

use log::*;
use rppal::spi::Spi;
use serde::Deserialize;

use crate::Result;

// From Table 1
#[allow(dead_code)]
enum Register {
    Configuration = 0,
    RtdMsbs = 1,
    RtdLsbs = 2,
    HighFaultThresholdMsb = 3,
    HighFaultThresholdLsb = 4,
    LowFaultThresholdMsb = 5,
    LowFaultThresholdLsb = 6,
    FaultStatus = 7,
}

impl Register {
    fn read(self) -> u8 {
        self as u8
    }

    fn write(self) -> u8 {
        // msb is marked when writing
        (self as u8) | 0b1000_0000
    }
}

#[allow(dead_code)]
#[rustfmt::skip]
pub enum Config {
    /// 1 = 50Hz, 0 = 60Hz
    Filter50Hz           = 0b0000_0001,
    /// 1 = clear (auto-clear)
    FaultStatusClear     = 0b0000_0010,
    /// See table 3
    FaultDetectionCycle2 = 0b0000_0100,
    /// See table 3
    FaultDetectionCycle3 = 0b0000_1000,
    /// 1 = 3-wire, 0 = 2-wire or 4-wire
    ThreeWire            = 0b0001_0000,
    /// 1 = one-shot (auto-clear)
    OneShot              = 0b0010_0000,
    /// 1 = auto, 0 = normally off
    ConversionMode       = 0b0100_0000,
    /// 1 = on, 0 = off
    VBias                = 0b1000_0000,
}

pub const CLOCK_SPEED: u32 = 40_000;
// use min and max thresholds, we don't care about faults
pub const LOW_THRESHOLD: u16 = 0;
pub const HIGH_THRESHOLD: u16 = 0x7fff;
pub const DEFAULT_CONFIG: u8 = {
    use self::Config::*;
    VBias as u8 | ConversionMode as u8
};

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub bus: u8,
    pub select: u8,
    pub n_samples: u32,
    pub resist_ref: f32,
    pub resist_zero: f32,
}

impl Settings {
    pub fn make(&self) -> Result<Max31865> {
        assert!(LOW_THRESHOLD < HIGH_THRESHOLD);
        // make sure the thresholds are 15-bit
        assert!(LOW_THRESHOLD < (1 << 15));
        assert!(HIGH_THRESHOLD < (1 << 15));

        use rppal::spi::*;

        let bus = match self.bus {
            0 => Bus::Spi0,
            1 => Bus::Spi1,
            2 => Bus::Spi2,
            _ => panic!("Bad bus: {}", self.bus),
        };

        let select = match self.select {
            0 => SlaveSelect::Ss0,
            1 => SlaveSelect::Ss1,
            2 => SlaveSelect::Ss2,
            _ => panic!("Bad select: {}", self.select),
        };

        let spi = Spi::new(bus, select, CLOCK_SPEED, Mode::Mode1)?;

        let mut max = Max31865 {
            spi,
            n_samples: self.n_samples,
            resist_ref: self.resist_ref,
            resist_zero: self.resist_zero,
        };

        max.initalize()?;

        Ok(max)
    }
}

pub struct Max31865 {
    spi: Spi,
    n_samples: u32,
    resist_ref: f32,
    resist_zero: f32,
}

impl Max31865 {
    fn initalize(&mut self) -> Result<()> {
        // first write out the config bits
        self.spi
            .write(&[Register::Configuration.write(), DEFAULT_CONFIG])?;

        // now write out the thresholds, knowing that it will auto-increment
        // starting from the HighFaultThresholdMsb register
        let (ht_msbs, ht_lsbs) = pack_word(HIGH_THRESHOLD);
        let (lt_msbs, lt_lsbs) = pack_word(LOW_THRESHOLD);

        self.spi.write(&[
            Register::HighFaultThresholdMsb.write(),
            ht_msbs,
            ht_lsbs,
            lt_msbs,
            lt_lsbs,
        ])?;
        Ok(())
    }

    pub fn read_one_resistance(&mut self) -> Result<f32> {
        // we are going to write 1 byte, then receive 8
        // but we have to use transfer instead of write/read because we need
        // clock line to stay low
        let count = 9;

        let mut tx_buf = vec![0; count];
        let mut rx_buf = vec![0; count];

        // write out the config register, that's where we will start reading
        tx_buf[0] = Register::Configuration.read();

        self.spi.transfer(&mut rx_buf, &tx_buf)?;

        // ignore the first byte, because that's when we were sending the
        // register to read from
        assert!(rx_buf[0] == 0);

        let config = rx_buf[1];
        let (resistance_bits, _) = unpack_word(rx_buf[2], rx_buf[3]);
        let (hi_threshold, _) = unpack_word(rx_buf[4], rx_buf[5]);
        let (lo_threshold, _) = unpack_word(rx_buf[6], rx_buf[7]);
        let status = rx_buf[8];

        debug!("Configuration:  {:08b}", config);
        debug!(
            "Raw Resistance: {:04x} ({})",
            resistance_bits, resistance_bits
        );
        debug!("Hi Threshold:   {:04x}", hi_threshold);
        debug!("Lo Threshold:   {:04x}", lo_threshold);
        debug!("Status:         {:08b}", status);

        // we don't handle status in anyway right now, so just make sure it's nothing
        // assert_eq!(status, 0);

        let resistance = f32::from(resistance_bits) * self.resist_ref / ((1 << 15) as f32);
        debug!("Resistance:     {}", resistance);
        // using the linear formula from the datasheet
        debug!(
            "ADC Resistance: {}",
            f32::from(resistance_bits) / 32.0 - 256.0
        );

        Ok(resistance)
    }

    /// Callendar-Van Dusen equation
    /// $R = R₀(1 + aT + bT² + c(T - 100)T³)$
    ///
    /// R: resistance
    /// T: temperature
    /// R₀: resistance at 0C
    /// a = 3.90830e-3
    /// b = -5.77500e-7
    /// c = -4.18301e-12 for -200C <= T <= 0C, 0 for 0C <= T <= +850C
    ///
    /// We will assume temperatures above 0C, so c = 0, allowing us to use the quadratic equation
    pub fn read_one_temperature(&mut self) -> Result<f32> {
        let resistance = self.read_one_resistance()? as f32;
        let r0 = self.resist_zero;

        let rtd_a = 3.90830e-3;
        let rtd_b = -5.77500e-7;

        // these are variable in the quadratic equation. note the mismatch
        // between these and the rtd constants
        let a = rtd_b;
        let b = rtd_a;
        let c = 1.0 - (resistance / r0);

        let root = (-b + f32::sqrt(b * b - 4.0 * a * c)) / (2.0 * a);

        Ok(root)
    }

    pub fn read_temperature(&mut self) -> Result<f32> {
        let mut sum = 0.0;
        for _ in 0..self.n_samples {
            sum += self.read_one_temperature()?;
        }

        Ok(sum / self.n_samples as f32)
    }
}

fn pack_word(word: u16) -> (u8, u8) {
    // make sure high bit is clear
    assert!((0x8000 & word) == 0);
    let msbs = (word >> 7) as u8;
    let lsbs = (word << 1) as u8;
    (msbs, lsbs)
}

fn unpack_word(msbs: u8, lsbs: u8) -> (u16, bool) {
    let word = (u16::from(msbs) << 8) | u16::from(lsbs);
    let low_bit = (word & 1) == 1;
    (word >> 1, low_bit)
}
