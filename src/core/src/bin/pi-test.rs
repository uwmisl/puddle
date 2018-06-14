extern crate clap;
extern crate puddle_core;
extern crate env_logger;
#[macro_use] extern crate log;
extern crate sysfs_pwm;
extern crate rppal;

use std::error::Error;
use clap::{SubCommand, App, Arg};
use sysfs_pwm::Pwm;
use rppal::gpio::{Gpio, Mode, Level};

use puddle_core::mcp4725::MCP4725;
use puddle_core::pca9685::PCA9685;

fn main() -> Result<(), Box<Error>> {
    // enable logging
    let _ = env_logger::try_init();

    let matches = App::new("pi test")
        .version("0.1")
        .author("Max Willsey <me@mwillsey.com>")
        .about("Test out some of the hardware on the pi")
        .subcommand(
            SubCommand::with_name("dac")
                .arg(
                    Arg::with_name("value")
                        .takes_value(true)
                        .required(true)
                )
        )
        .subcommand(
            SubCommand::with_name("pwm")
                .arg(
                    Arg::with_name("channel")
                        .takes_value(true)
                        .required(true)
                )
                .arg(
                    Arg::with_name("duty")
                        .takes_value(true)
                        .required(true)
                )
                .arg(
                    Arg::with_name("freq")
                        .takes_value(true)
                        .required(true)
                )
        )
        .subcommand(
            SubCommand::with_name("pi-pwm")
                .arg(
                    Arg::with_name("channel")
                        .takes_value(true)
                        .required(true)
                )
                .arg(
                    Arg::with_name("duty")
                        .takes_value(true)
                        .required(true)
                )
                .arg(
                    Arg::with_name("period")
                        .takes_value(true)
                        .required(true)
                )
        )
        .get_matches();

    match matches.subcommand() {
        ("dac", Some(m)) => {
            let value = m.value_of("value").unwrap().parse().unwrap();
            dac_test(value)
        },
        ("pwm", Some(m)) => {
            let channel = m.value_of("channel").unwrap().parse().unwrap();
            let duty = m.value_of("duty").unwrap().parse().unwrap();
            let freq = m.value_of("freq").unwrap().parse().unwrap();
            pwm_test(channel, duty, freq)
        },
        ("pi-pwm", Some(m)) => {
            let channel = m.value_of("channel").unwrap().parse().unwrap();
            let duty = m.value_of("duty").unwrap().parse().unwrap();
            let period = m.value_of("period").unwrap().parse().unwrap();
            pi_pwm_test(channel, duty, period)
        },
        _ => {
            println!("Please pick a subcommmand.");
            Ok(())
        },
    }
}

fn dac_test(value: u16) -> Result<(), Box<Error>> {
    let addr = 0x60;
    let mut mcp = MCP4725::new(addr);
    mcp.write(value);
    Ok(())
}

fn pwm_test(channel: u8, duty: u16, frequency: f64) -> Result<(), Box<Error>> {
    let addr = 0x42;
    let mut pca = PCA9685::new(addr);
    pca.set_pwm_freq(frequency);
    pca.set_duty_cycle(channel, duty);
    Ok(())
}

fn pi_pwm_test(channel: u8, duty: u32, period: u32) -> Result<(), Box<Error>> {

    let mut gpio = Gpio::new().expect("gpio init failed!");
    gpio.set_mode(18, Mode::Alt5);
    gpio.set_mode(13, Mode::Alt0);
    gpio.set_mode(12, Mode::Alt0);
    gpio.set_clear_on_drop(false);

    // gpio.set_mode(18, Mode::Output);
    // gpio.write(18, Level::High);

    let pi_chip = 0;
    let pwm = Pwm::new(pi_chip, channel as u32).unwrap();
    pwm.export().unwrap();
    info!("PWM exported");
    pwm.set_period_ns(period).unwrap();
    info!("PWM period set to {} ns", period);
    pwm.set_duty_cycle_ns(duty).unwrap();
    info!("PWM duty cycle set to {} ns", duty);
    pwm.enable(true).unwrap();
    info!("PWM enabled");


    Ok(())
}
