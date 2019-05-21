// pub mod max31865;
// pub mod mcp4725;
// pub mod pca9685;

mod hv507;

use std::time::Duration;

use rppal::gpio::Gpio;

use crate::grid::gridview::GridView;
use crate::grid::{parse::PiConfig, Location, Peripheral};

#[cfg(feature = "vision")]
use vision::Detector;

// use self::max31865::{Max31865, MAX31865_DEFAULT_CONFIG};
// use self::mcp4725::{Mcp4725, MCP4725_DEFAULT_ADDRESS};
// use self::pca9685::{Pca9685, PCA9685_DEFAULT_ADDRESS};
use self::hv507::Hv507;

// const PHYS_0: usize/

// #[derive(Debug)]
// pub struct PiError {
//     msg: String,
//     code: i32,
// }

// impl PiError {
//     fn from_code(code: i32) -> PiError {
//         assert!(code < 0);
//         let msg_buf = unsafe { CStr::from_ptr(pigpio_error(code)) };
//         let msg = msg_buf.to_str().unwrap().into();
//         PiError { msg, code }
//     }
// }

// impl fmt::Display for PiError {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "Pi error code {}: '{}'", self.code, self.msg)
//     }
// }

// impl ::std::error::Error for PiError {
//     fn description(&self) -> &str {
//         &self.msg
//     }
// }

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct RaspberryPi {
    // pi_num: i32,
    // pub mcp4725: Mcp4725,
    // pub pca9685: Pca9685,
    // pub max31865: Max31865,
    // pwm:
    // pwm: Pwm;
    config: PiConfig,
    gpio: Gpio,
    pub hv507: Hv507,
}

impl RaspberryPi {
    pub fn new(config: PiConfig) -> Result<RaspberryPi> {
        trace!("Initializing pi gpio...");
        let gpio = Gpio::new()?;

        trace!("Initializing pi hv507...");
        let mut hv507 = Hv507::new(&gpio)?;
        hv507.init(&config.polarity).unwrap();

        trace!("Initializing pi...");
        let pi = RaspberryPi {
            config,
            gpio,
            hv507,
        };
        trace!("Initialized pi!");

        // let i2c_bus = 1;
        // let i2c_flags = 0;

        // // FIXME THESE ARE FAKE
        // warn!("The SPI params are probably not right yet, and haven't been tested");
        // let spi_channel = 0;
        // let spi_baud = 40_000;
        // let spi_mode = 1;

        // let mcp4725 = {
        //     let i2c = I2cHandle::new(pi_num, i2c_bus, MCP4725_DEFAULT_ADDRESS, i2c_flags)?;
        //     Mcp4725::new(i2c)
        // };

        // let pca9685 = {
        //     let i2c = I2cHandle::new(pi_num, i2c_bus, PCA9685_DEFAULT_ADDRESS, i2c_flags)?;
        //     Pca9685::new(i2c)?
        // };

        // let max31865 = {
        //     let spi = SpiHandle::new(pi_num, spi_channel, spi_baud, spi_mode)?;
        //     // use min and max thresholds, we don't care about faults
        //     let low_threshold = 0;
        //     let high_threshold = 0x7fff;
        //     Max31865::new(spi, MAX31865_DEFAULT_CONFIG, low_threshold, high_threshold)?
        // };

        // let mut pi = RaspberryPi {
        //     pi_num,
        //     // mcp4725,
        //     // pca9685,
        //     // max31865,
        // };
        Ok(pi)
    }

    pub fn heat(
        &mut self,
        _heater: &Peripheral,
        _target_temperature: f64,
        _duration: Duration,
    ) -> Result<()> {
        unimplemented!()
        // // FIXME: for now, this simply blocks

        // let pwm_channel = if let Peripheral::Heater { pwm_channel, .. } = heater {
        //     *pwm_channel
        // } else {
        //     panic!("Peripheral wasn't a heater!: {:#?}", heater)
        // };

        // let mut pid = PidController::default();
        // pid.p_gain = 1.0;
        // pid.i_gain = 1.0;
        // pid.d_gain = 1.0;

        // use std::env;

        // pid.p_gain = env::var("PID_P").unwrap_or("1.0".into()).parse().unwrap();
        // pid.i_gain = env::var("PID_I").unwrap_or("1.0".into()).parse().unwrap();
        // pid.d_gain = env::var("PID_D").unwrap_or("1.0".into()).parse().unwrap();

        // pid.i_min = 0.0;
        // pid.i_max = pca9685::DUTY_CYCLE_MAX as f64;

        // pid.out_min = 0.0;
        // pid.out_max = pca9685::DUTY_CYCLE_MAX as f64;

        // pid.target = target_temperature;

        // let epsilon = 2.0; // degrees C
        // let extra_delay = Duration::from_millis(20);

        // let mut timer = Timer::new();
        // let mut in_range_start: Option<Instant> = None;

        // for iteration in 0.. {
        //     // stop if we've been in the desired temperature range for long enough
        //     if in_range_start
        //         .map(|t| t.elapsed() > duration)
        //         .unwrap_or(false)
        //     {
        //         break;
        //     }

        //     // FIXME HACK the mcp thing is bad here
        //     self.mcp4725.write(0).unwrap();
        //     thread::sleep(Duration::from_millis(30));
        //     let measured = self.max31865.read_one_temperature()? as f64;
        //     self.mcp4725.write(1600).unwrap();

        //     let dt = timer.lap();
        //     let mut duty_cycle = pid.update(measured, &dt);

        //     debug!(
        //         "Heating to {}*C... iteration: {}, measured: {}*C, duty_cycle: {}",
        //         target_temperature, iteration, measured, duty_cycle
        //     );

        //     if measured - target_temperature > epsilon {
        //         self.pca9685.set_duty_cycle(pwm_channel, 0)?;
        //         warn!(
        //             "We overshot the target temperature. Wanted {}, got {}",
        //             target_temperature, measured
        //         );
        //         duty_cycle = 0.0;
        //     }

        //     if (target_temperature - measured).abs() < epsilon && in_range_start.is_none() {
        //         in_range_start = Some(Instant::now())
        //     }

        //     assert!(0.0 <= duty_cycle);
        //     assert!(duty_cycle <= pca9685::DUTY_CYCLE_MAX as f64);
        //     self.pca9685
        //         .set_duty_cycle(pwm_channel, duty_cycle as u16)?;

        //     thread::sleep(extra_delay);
        // }

        // self.pca9685.set_duty_cycle(pwm_channel, 0)?;

        // Ok(())
    }

    pub fn get_temperature(&mut self, _temp_sensor: Peripheral) -> Result<f32> {
        unimplemented!()
        // if let Peripheral::Heater { spi_channel, .. } = temp_sensor {
        //     // right now we can only work on the one channel
        //     assert_eq!(spi_channel, 0);
        //     self.max31865.read_temperature()
        // } else {
        //     panic!("Not a temperature sensor!: {:#?}")
        // }
    }

    // // FIXME HACK
    pub fn bad_manual_output_pins(&mut self, pins: &[u8]) {
        // actually write the pins and cycle the clock
        for (i, pin) in pins.iter().enumerate() {
            if *pin == 0 {
                self.hv507.set_pin_lo(i)
            } else {
                self.hv507.set_pin_hi(i)
            }
        }

        self.hv507.shift_and_latch();
    }

    pub fn output_pins(&mut self, gv: &GridView) {
        let mut pins = vec![0; (gv.grid.max_pin() + 1) as usize];

        self.hv507.clear_pins();

        // set pins to high if there's a droplet on that electrode
        for d in gv.droplets.values() {
            for i in 0..d.dimensions.y {
                for j in 0..d.dimensions.x {
                    let loc = Location {
                        y: d.location.y + i,
                        x: d.location.x + j,
                    };
                    let electrode = gv
                        .grid
                        .get_cell(loc)
                        .unwrap_or_else(|| panic!("Couldn't find electrode for {}", loc));
                    pins[electrode.pin as usize] = 1;
                    self.hv507.set_pin_hi(electrode.pin as usize);
                    trace!(
                        "Setting pin {} at ({}, {})",
                        electrode.pin,
                        d.location.y + i,
                        d.location.x + j,
                    );
                }
            }
        }

        self.hv507.shift_and_latch();
    }

    pub fn input(&mut self, _input_port: &Peripheral, _volume: f64) -> Result<()> {
        unimplemented!()
        //     let pwm_channel = if let Peripheral::Input { pwm_channel, .. } = input_port {
        //         *pwm_channel
        //     } else {
        //         panic!("Peripheral wasn't an input port!: {:#?}", input_port)
        //     };

        //     let pump_duty_cycle = pca9685::DUTY_CYCLE_MAX / 2;

        //     let pump_duration = {
        //         // n.b. max flow rate is .45 ml/min +/- 15% at 20 C, or 7.5 ul/s
        //         let ul_per_second = 7.0;
        //         let ul_per_volume = 1.0;
        //         let seconds = volume * ul_per_volume / ul_per_second;
        //         seconds_duration(seconds)
        //     };

        //     self.pca9685.set_duty_cycle(pwm_channel, pump_duty_cycle)?;
        //     thread::sleep(pump_duration);
        //     self.pca9685.set_duty_cycle(pwm_channel, 0)?;
        //     Ok(())
    }

    pub fn output(&mut self, _output_port: &Peripheral, _volume: f64) -> Result<()> {
        unimplemented!()
        //     let pwm_channel = if let Peripheral::Output { pwm_channel, .. } = output_port {
        //         *pwm_channel
        //     } else {
        //         panic!("Peripheral wasn't an output port!: {:#?}", output_port)
        //     };

        //     let pump_duty_cycle = pca9685::DUTY_CYCLE_MAX / 2;

        //     let pump_duration = {
        //         // n.b. max flow rate is .45 ml/min +/- 15% at 20 C, or 7.5 ul/s
        //         let ul_per_second = 4.0;
        //         let ul_per_volume = 4.0;
        //         let seconds = volume * ul_per_volume / ul_per_second;
        //         seconds_duration(seconds)
        //     };

        //     self.pca9685.set_duty_cycle(pwm_channel, pump_duty_cycle)?;
        //     thread::sleep(pump_duration);
        //     self.pca9685.set_duty_cycle(pwm_channel, 0)?;
        //     Ok(())
    }
}
