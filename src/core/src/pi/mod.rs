pub mod max31865;
pub mod mcp4725;
pub mod pca9685;

use std::ffi::CStr;
use std::fmt;
use std::os::raw::{c_char, c_int, c_uint};
use std::ptr;
use std::thread;
use std::time::{Duration, Instant};

use grid::{Grid, Location, Peripheral, Snapshot};
use util::{pid::PidController, Timer};

use self::max31865::{MAX31865_DEFAULT_CONFIG, Max31865};
use self::mcp4725::{MCP4725_DEFAULT_ADDRESS, Mcp4725};
use self::pca9685::{PCA9685_DEFAULT_ADDRESS, Pca9685};

#[allow(non_camel_case_types)]
type int = c_int;
#[allow(non_camel_case_types)]
type unsigned = c_uint;

#[link(name = "pigpiod_if2", kind = "dylib")]
extern "C" {
    fn pigpio_start(addr: *const c_char, port: *const c_char) -> int;
    // fn pigpio_stop(pi: int);
    fn set_mode(pi: int, gpio: unsigned, mode: unsigned) -> int;
    fn gpio_write(pi: int, gpio: unsigned, level: unsigned) -> int;
    fn hardware_PWM(pi: int, gpio: unsigned, pwm_freq: unsigned, pwm_duty: u32) -> int;
    fn pigpio_error(code: int) -> *const c_char;

    fn i2c_open(pi: int, i2c_bus: unsigned, i2c_addr: unsigned, i2c_flags: unsigned) -> int;
    fn i2c_close(pi: int, handle: unsigned) -> int;
    fn i2c_write_device(pi: int, handle: unsigned, buf: *const u8, count: unsigned) -> int;
    fn i2c_read_device(pi: int, handle: unsigned, buf: *mut u8, count: unsigned) -> int;

    fn spi_open(pi: int, spi_channel: unsigned, baud: unsigned, spi_flags: unsigned) -> int;
    fn spi_close(pi: int, handle: unsigned) -> int;
    fn spi_xfer(
        pi: int,
        handle: unsigned,
        tx_buf: *const u8,
        rx_buf: *mut u8,
        count: unsigned,
    ) -> int;
    fn spi_write(pi: int, handle: unsigned, buf: *const u8, count: unsigned) -> int;
    fn spi_read(pi: int, handle: unsigned, buf: *mut u8, count: unsigned) -> int;
}

pub enum GpioPin {
    /// HV507 blank
    /// Physical pin 11 - BCM 17
    Blank = 17,

    /// HV507 latch enable
    /// Physical pin 13 - BCM 27
    LatchEnable = 27,

    /// HV507 clock
    /// Physical pin 15 - BCM 22
    Clock = 22,

    /// HV507 data
    /// Physical pin 16 - BCM 23
    Data = 23,

    /// HV507 polarity
    /// Pin 32 - BCM 12 (PWM0)
    Polarity = 12,
    // /// High voltage converter "analog" signal
    // /// Pin 33 - BCM 13 (PWM1)
    // Voltage = 13,
}

// numbers taken from pigpio.h
pub enum GpioMode {
    Input = 0,
    Output = 1,
    Alt0 = 4,
    Alt1 = 5,
    Alt2 = 6,
    Alt3 = 7,
    Alt4 = 3,
    Alt5 = 2,
}

#[derive(Debug)]
pub struct PiError {
    msg: String,
    code: i32,
}

impl PiError {
    fn from_code(code: i32) -> PiError {
        assert!(code < 0);
        let msg_buf = unsafe { CStr::from_ptr(pigpio_error(code)) };
        let msg = msg_buf.to_str().unwrap().into();
        PiError { msg, code }
    }
}

impl fmt::Display for PiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Pi error code {}: '{}'", self.code, self.msg)
    }
}

impl ::std::error::Error for PiError {
    fn description(&self) -> &str {
        &self.msg
    }
}

pub type Result<T> = ::std::result::Result<T, PiError>;

macro_rules! res {
    ($code:expr, $ok:expr) => {
        if $code >= 0 {
            Ok($ok)
        } else {
            Err(PiError::from_code($code))
        }
    };
    ($code:expr) => {
        res!($code, ())
    };
}

pub struct RaspberryPi {
    pi_num: i32,
    pub mcp4725: Mcp4725,
    pub pca9685: Pca9685,
    pub max31865: Max31865,
}

impl RaspberryPi {
    pub fn new() -> Result<RaspberryPi> {
        let pi_num = {
            let r = unsafe { pigpio_start(ptr::null(), ptr::null()) };
            res!(r, r)?
        };

        // at this point, the pi_num is validated by the res! macro
        assert!(pi_num >= 0);

        let i2c_bus = 1;
        let i2c_flags = 0;

        // FIXME THESE ARE FAKE
        warn!("The SPI params are probably not right yet, and haven't been tested");
        let spi_channel = 0;
        let spi_baud = 40_000;
        let spi_mode = 1;

        let mcp4725 = {
            let i2c = I2cHandle::new(pi_num, i2c_bus, MCP4725_DEFAULT_ADDRESS, i2c_flags)?;
            Mcp4725::new(i2c)
        };

        let pca9685 = {
            let i2c = I2cHandle::new(pi_num, i2c_bus, PCA9685_DEFAULT_ADDRESS, i2c_flags)?;
            Pca9685::new(i2c)?
        };

        let max31865 = {
            let spi = SpiHandle::new(pi_num, spi_channel, spi_baud, spi_mode)?;
            // use min and max thresholds, we don't care about faults
            let low_threshold = 0;
            let high_threshold = 0x7fff;
            Max31865::new(spi, MAX31865_DEFAULT_CONFIG, low_threshold, high_threshold)?
        };

        let mut pi = RaspberryPi {
            pi_num,
            mcp4725,
            pca9685,
            max31865,
        };

        pi.init_hv507();

        Ok(pi)
    }

    pub fn gpio_write(&mut self, gpio: GpioPin, level: u8) -> Result<()> {
        self.gpio_write_num(gpio as u32, level)
    }

    pub fn gpio_write_num(&mut self, gpio: u32, level: u8) -> Result<()> {
        let code = unsafe { gpio_write(self.pi_num, gpio, level as u32) };
        res!(code)
    }

    pub fn gpio_set_mode(&mut self, gpio: GpioPin, mode: GpioMode) -> Result<()> {
        let code = unsafe { set_mode(self.pi_num, gpio as u32, mode as u32) };
        res!(code)
    }

    pub fn set_pwm(&mut self, gpio: u32, pwm_freq: u32, pwm_duty: u32) -> Result<()> {
        let code = unsafe { hardware_PWM(self.pi_num, gpio, pwm_freq, pwm_duty) };
        res!(code)
    }

    fn init_hv507(&mut self) {
        // setup the HV507 for serial data write
        // see row "LOAD S/R" in table 3-2 in
        // http://ww1.microchip.com/downloads/en/DeviceDoc/20005845A.pdf

        use self::GpioMode::*;
        use self::GpioPin::*;

        let frequency = 490;
        let duty_cycle = 500_000; // out of 1_000_000
        self.set_pwm(Polarity as u32, frequency, duty_cycle)
            .unwrap();

        self.gpio_set_mode(Blank, Output).unwrap();
        self.gpio_write(Blank, 1).unwrap();

        self.gpio_set_mode(LatchEnable, Output).unwrap();
        self.gpio_write(LatchEnable, 0).unwrap();

        self.gpio_set_mode(Clock, Output).unwrap();
        self.gpio_write(Clock, 0).unwrap();

        self.gpio_set_mode(Data, Output).unwrap();
        self.gpio_write(Data, 0).unwrap();
    }

    pub fn heat(
        &mut self,
        heater: &Peripheral,
        target_temperature: f64,
        duration: Duration,
    ) -> Result<()> {
        // FIXME: for now, this simply blocks

        if let Peripheral::Heater { pwm_channel, .. } = *heater {
            let mut pid = PidController::default();
            pid.p_gain = 1.0;
            pid.i_gain = 1.0;
            pid.d_gain = 1.0;

            pid.i_min = 0.0;
            pid.i_max = pca9685::DUTY_CYCLE_MAX as f64;

            pid.out_min = 0.0;
            pid.out_max = pca9685::DUTY_CYCLE_MAX as f64;

            pid.target = target_temperature;

            let epsilon = 2.0; // degrees C
            let extra_delay = Duration::from_millis(20);

            let mut timer = Timer::new();
            let mut in_range_start: Option<Instant> = None;

            for iteration in 0.. {
                // stop if we've been in the desired temperature range for long enough
                if in_range_start
                    .map(|t| t.elapsed() > duration)
                    .unwrap_or(false)
                {
                    break;
                }

                let measured = self.max31865.read_temperature()? as f64;
                let dt = timer.lap();

                debug!(
                    "Heating to {}*C... iteration: {}, measured: {}*C",
                    target_temperature, iteration, measured
                );

                if measured - target_temperature > epsilon {
                    self.pca9685.set_duty_cycle(pwm_channel, 0)?;
                    panic!(
                        "We overshot the target temperature. Wanted {}, got {}",
                        target_temperature, measured
                    );
                }

                if target_temperature - measured > epsilon {
                    in_range_start = Some(Instant::now())
                }

                let duty_cycle = pid.update(measured, &dt);
                assert!(0.0 <= duty_cycle);
                assert!(duty_cycle <= pca9685::DUTY_CYCLE_MAX as f64);
                self.pca9685.set_duty_cycle(pwm_channel, duty_cycle as u16)?;

                thread::sleep(extra_delay);
            }

            self.pca9685.set_duty_cycle(pwm_channel, 0)?;
        } else {
            panic!("Not a temperature sensor!: {:#?}")
        };

        Ok(())
    }

    pub fn get_temperature(&mut self, temp_sensor: Peripheral) -> Result<f32> {
        if let Peripheral::Heater { spi_channel, .. } = temp_sensor {
            // right now we can only work on the one channel
            assert_eq!(spi_channel, 0);
            self.max31865.read_temperature()
        } else {
            panic!("Not a temperature sensor!: {:#?}")
        }
    }

    pub fn output_pins(&mut self, grid: &Grid, snapshot: &Snapshot) {
        let mut pins = vec![0; (grid.max_pin() + 1) as usize];

        // reset pins to low by default
        for p in pins.iter_mut() {
            *p = 0;
        }

        // set pins to high if there's a droplet on that electrode
        for d in snapshot.droplets.values() {
            for i in 0..d.dimensions.y {
                for j in 0..d.dimensions.x {
                    let loc = Location {
                        y: d.location.y + i,
                        x: d.location.x + j,
                    };
                    let electrode = grid.get_cell(&loc).unwrap();
                    pins[electrode.pin as usize] = 1;
                    trace!(
                        "Setting pin {} at ({}, {})",
                        electrode.pin,
                        d.location.y + i,
                        d.location.x + j,
                    );
                }
            }
        }

        use self::GpioPin::*;
        // actually write the pins and cycle the clock
        for pin in pins.iter() {
            self.gpio_write(Data, *pin).unwrap();
            self.gpio_write(Clock, 1).unwrap();
            self.gpio_write(Clock, 0).unwrap();
        }

        // commit the latch
        self.gpio_write(LatchEnable, 1).unwrap();
        self.gpio_write(LatchEnable, 0).unwrap();
    }
}

#[derive(Debug)]
pub struct I2cHandle {
    pi_num: i32,
    handle: u32,
}

impl I2cHandle {
    fn new(pi_num: i32, bus: u8, address: u16, flags: u8) -> Result<I2cHandle> {
        let handle_result = unsafe { i2c_open(pi_num, bus as u32, address as u32, flags as u32) };
        let handle = res!(handle_result, handle_result)? as u32;
        Ok(I2cHandle { pi_num, handle })
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<()> {
        res!(unsafe { i2c_write_device(self.pi_num, self.handle, buf.as_ptr(), buf.len() as u32) })
    }

    pub fn read(&mut self, count: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0; count];
        self.read_into(&mut buf)?;
        Ok(buf)
    }

    pub fn read_into(&mut self, buf: &mut [u8]) -> Result<()> {
        let read_result = unsafe {
            i2c_read_device(self.pi_num, self.handle, buf.as_mut_ptr(), buf.len() as u32)
        };
        let n_read = res!(read_result, read_result)?;
        assert!(n_read as usize == buf.len());
        Ok(())
    }
}

impl Drop for I2cHandle {
    fn drop(&mut self) {
        let result = res!(unsafe { i2c_close(self.pi_num, self.handle) });
        match result {
            Ok(()) => debug!("Successfully dropped {:#?}", self),
            Err(err) => error!("Error while dropping {:#?}: {:#?}", self, err),
        }
    }
}

#[derive(Debug)]
pub struct SpiHandle {
    pi_num: i32,
    handle: u32,
}

impl SpiHandle {
    fn new(pi_num: i32, channel: u8, baud: u32, mode: u8) -> Result<SpiHandle> {
        // mode is 0-3, and is the only flag we care about for now
        // for the rest, we use the pigpio defaults of 0
        assert!(mode <= 3);
        let flags = mode as u32;

        let handle_result = unsafe { spi_open(pi_num, channel as u32, baud, flags) };
        let handle = res!(handle_result, handle_result)? as u32;
        Ok(SpiHandle { pi_num, handle })
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<()> {
        trace!("SPI write on handle {}: {:#x?}", self.handle, buf);
        res!(unsafe { spi_write(self.pi_num, self.handle, buf.as_ptr(), buf.len() as u32) })
    }

    pub fn read(&mut self, count: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0; count];
        self.read_into(&mut buf)?;
        Ok(buf)
    }

    pub fn transfer(&mut self, tx_buf: &[u8], rx_buf: &mut [u8]) -> Result<()> {
        // to prevent unexpected behavior, we just assert that the buffers are the same length
        assert_eq!(tx_buf.len(), rx_buf.len());
        trace!(
            "SPI transfer tx with handle {}: {:#x?}",
            self.handle,
            tx_buf
        );
        let count = tx_buf.len() as u32;
        let xfer_result = unsafe {
            spi_xfer(
                self.pi_num,
                self.handle,
                tx_buf.as_ptr(),
                rx_buf.as_mut_ptr(),
                count,
            )
        };
        trace!(
            "SPI transfer rx with handle {}: {:#x?}",
            self.handle,
            rx_buf
        );
        let n_read = res!(xfer_result, xfer_result)?;
        assert_eq!(n_read as usize, rx_buf.len());
        Ok(())
    }

    pub fn read_into(&mut self, buf: &mut [u8]) -> Result<()> {
        let read_result =
            unsafe { spi_read(self.pi_num, self.handle, buf.as_mut_ptr(), buf.len() as u32) };
        trace!("SPI read from handle {}: {:#x?}", self.handle, buf);
        let n_read = res!(read_result, read_result)?;
        assert!(n_read as usize == buf.len());
        Ok(())
    }
}

impl Drop for SpiHandle {
    fn drop(&mut self) {
        let result = res!(unsafe { spi_close(self.pi_num, self.handle) });
        match result {
            Ok(()) => trace!("Successfully dropped {:#?}", self),
            Err(err) => error!("Error while dropping {:#?}: {:#?}", self, err),
        }
    }
}
