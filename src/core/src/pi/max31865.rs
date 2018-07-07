// https://datasheets.maximintegrated.com/en/ds/MAX31865.pdf

use super::{Result, SpiHandle};

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
        (self as u8) & 0b1000_0000
    }
}

#[allow(dead_code)]
pub enum Config {
    /// 1 = 50Hz, 0 = 60Hz
    Filter50Hz = 0b0000_0001,
    /// 1 = clear (auto-clear)
    FaultStatusClear = 0b0000_0010,
    /// See table 3
    FaultDetectionCycle2 = 0b0000_0100,
    /// See table 3
    FaultDetectionCycle3 = 0b0000_1000,
    /// 1 = 3-wire, 0 = 2-wire or 4-wire
    ThreeWire = 0b0001_0000,
    /// 1 = one-shot (auto-clear)
    OneShot = 0b0010_0000,
    /// 1 = auto, 0 = normally off
    ConversionMode = 0b0100_0000,
    /// 1 = on, 0 = off
    VBias = 0b1000_0000,
}

pub struct Max31865 {
    spi: SpiHandle,
    config: u8,
    low_threshold: u16,
    high_threshold: u16,
}

pub const MAX31865_DEFAULT_CONFIG: u8 = 0;

impl Max31865 {
    pub fn new(
        spi: SpiHandle,
        config: u8,
        low_threshold: u16,
        high_threshold: u16,
    ) -> Result<Max31865> {
        assert!(low_threshold < high_threshold);
        // make sure the thresholds are 15-bit
        assert!(low_threshold < (1 << 15));
        assert!(high_threshold < (1 << 15));
        let mut max = Max31865 {
            spi,
            config,
            low_threshold,
            high_threshold,
        };
        max.initalize()?;
        Ok(max)
    }

    fn initalize(&mut self) -> Result<()> {
        // first write out the config bits
        self.spi
            .write(&[Register::Configuration.write(), self.config])?;

        // now write out the thresholds, knowing that it will auto-increment
        // starting from the HighFaultThresholdMsb register
        let ht_msbs = (self.high_threshold >> 8) as u8;
        let ht_lsbs = self.high_threshold as u8;
        let lt_msbs = (self.low_threshold >> 8) as u8;
        let lt_lsbs = self.low_threshold as u8;

        self.spi.write(&[
            Register::HighFaultThresholdMsb.write(),
            self.config,
            ht_msbs,
            ht_lsbs,
            lt_msbs,
            lt_lsbs,
        ])
    }

    pub fn read_resistance(&mut self) -> Result<u16> {
        let count = 8;

        let mut tx_buf = vec![0; count];
        let mut rx_buf = vec![0; count];

        // write out the config register, that's where we will start reading
        tx_buf[0] = Register::Configuration.read();

        self.spi.transfer(&tx_buf, &mut rx_buf)?;

        let _config = rx_buf[0];
        let resistance = word(rx_buf[1], rx_buf[2]);
        let _hi_threshold = word(rx_buf[3], rx_buf[4]);
        let _lo_threshold = word(rx_buf[5], rx_buf[6]);
        let _status = rx_buf[7];

        Ok(resistance)
    }

    /// Callendar-Van Dusen equation
    /// $R = R₀(1 + aT + bT² + c(T - 100)T³)$
    ///
    /// R: resistance
    /// T: temperature
    /// R₀: resistance as 0C
    /// a = 3.90830e-3
    /// b = -5.77500e-7
    /// c = -4.18301e-12 for -200C <= T <= 0C, 0 for 0C <= T <= +850C
    ///
    /// We will assume temperatures above 0C, so c = 0, allowing us to use the quadratic equation
    pub fn read_temperature(&mut self) -> Result<f32> {
        let resistance = self.read_resistance()? as f32;
        let r0 = 100.0;

        let rtd_a = 3.90830e-3;
        let rtd_b = -5.77500e-7;

        // these are variable in the quadratic equation. note the mismatch
        // between these and the rtd constants
        let a = rtd_b;
        let b = rtd_a;
        let c = (resistance / r0) - 1.0;

        let root = (-b + f32::sqrt(b * b - 4.0 * a * c)) / (2.0 * a);

        Ok(root)
    }
}

fn word(hi: u8, lo: u8) -> u16 {
    ((hi as u16) << 8) | (lo as u16)
}
