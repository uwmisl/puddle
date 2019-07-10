use std::time::Duration;

use log::*;
use serde::Deserialize;

use puddle_core::grid::gridview::GridView;
use puddle_core::grid::{location::yx, Peripheral};

pub mod devices;
mod error;

pub use error::{Error, Result};

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub hv507: devices::hv507::Settings,
    pub mcp4725: Option<devices::mcp4725::Settings>,
    pub pca9685: Option<devices::pca9685::Settings>,
    pub max31865: Option<devices::max31865::Settings>,
}

const TABLE_KEYS: &[&str] = &["pi.mcp4725", "pi.pca9685", "pi.max31865"];

impl Settings {
    pub fn from_config(conf: &mut config::Config) -> Result<Self> {
        // For keys that _should_ represent tables, check if they are
        // the empty string, indicating that a use tried to override
        // it with an environment variable. If so, set it to None.
        for k in TABLE_KEYS {
            if let Ok(s) = conf.get_str(k) {
                if s.is_empty() {
                    conf.set(k, Option::<i64>::None)?;
                }
            }
        }

        let pi_conf: config::Value = conf.get("pi")?;
        debug!("{:#?}", pi_conf);
        let settings: Settings = pi_conf.try_into()?;
        info!("{:#?}", settings);
        Ok(settings)
    }
}

pub struct RaspberryPi {
    pub hv507: devices::hv507::Hv507,
    pub mcp4725: Option<devices::mcp4725::Mcp4725>,
    pub pca9685: Option<devices::pca9685::Pca9685>,
    pub max31865: Option<devices::max31865::Max31865>,
}

impl RaspberryPi {
    pub fn new(settings: Settings) -> Result<RaspberryPi> {
        trace!("Initializing pi...");
        let pi = RaspberryPi {
            hv507: settings.hv507.make()?,
            mcp4725: settings.mcp4725.map(|s| s.make()).transpose()?,
            pca9685: settings.pca9685.map(|s| s.make()).transpose()?,
            max31865: settings.max31865.map(|s| s.make()).transpose()?,
        };
        trace!("Initialized pi!");

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

    pub fn output_pins(&mut self, gv: &GridView) {
        let mut pins = vec![0; (gv.grid.max_pin() + 1) as usize];

        self.hv507.clear_pins();

        // set pins to high if there's a droplet on that electrode
        for d in gv.droplets.values() {
            for i in 0..d.dimensions.y {
                for j in 0..d.dimensions.x {
                    let loc = yx(d.location.y + i, d.location.x + j);
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

#[cfg(test)]
mod tests {
    use super::*;

    use config::{Config, Environment, File, FileFormat};

    static YAML: &str = include_str!("../../../tests/arches/purpledrop.yaml");

    #[test]
    fn make_purpledrop() {
        let mut conf = Config::new();
        conf.merge(File::from_str(YAML, FileFormat::Yaml)).unwrap();
        Settings::from_config(&mut conf).unwrap();
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_env_override() {
        std::env::set_var("PI__HV507__DUTY_CYCLE", "123456.7");
        std::env::set_var("PI__MCP4725", "");

        let mut conf = Config::new();
        conf.merge(File::from_str(YAML, FileFormat::Yaml)).unwrap();
        conf.merge(Environment::new().separator("__")).unwrap();
        let settings = Settings::from_config(&mut conf).unwrap();

        assert_eq!(settings.hv507.duty_cycle, 123_456.7);
        if let Some(m) = settings.mcp4725 {
            panic!("Should be None: {:#?}", m);
        }
    }
}
