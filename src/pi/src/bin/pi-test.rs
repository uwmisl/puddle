use std::error::Error;
use std::fs::File;
use std::path::Path;

use log::*;

use puddle_core::{
    grid::droplet::{Blob, DropletId, SimpleBlob},
    grid::gridview::GridView,
    grid::parse::{ParsedGrid, PolarityConfig},
    grid::{Grid, Location},
    util::seconds_duration,
};
use puddle_pi::RaspberryPi;

#[derive(Debug, Clone, Copy)]
struct MyDuration(std::time::Duration);

impl std::str::FromStr for MyDuration {
    type Err = std::num::ParseFloatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let float: f64 = s.parse()?;
        if float < 0.0 {
            panic!("Float should be non-negative");
        }
        Ok(MyDuration(seconds_duration(float)))
    }
}

use structopt::StructOpt;

type RunResult<T> = Result<T, Box<dyn Error>>;
type SleepFn = Fn(MyDuration) -> RunResult<()>;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
enum SubCommand {
    SetPolarity(SetPolarity),
    SetPin(SetPin),
    SetLoc(SetLoc),
    Circle(Circle),
    BackAndForth(BackAndForth),
    ToggleMask(ToggleMask),
    // Dac,
    // Pwm,
    // Temp,
    // Heat,
    // Pins,
}

static SIGNALS: &[i32] = &[signal_hook::SIGINT];

fn main() -> RunResult<()> {
    // enable logging
    let _ = env_logger::try_init();

    // set up the sleep function by registering a signal handler that
    // will catch a ctrl-c, stop the thread from sleeping, and return
    // an error
    let sleep = {
        let (signal_tx, signal_rx) = std::sync::mpsc::sync_channel(10);

        for &sig in SIGNALS {
            let tx = signal_tx.clone();
            let f = move || {
                if let Err(e) = tx.try_send(sig) {
                    eprintln!("Couldn't send a signal! {:?}", e);
                }
            };
            unsafe { signal_hook::register(sig, f) }.unwrap();
        }

        move |dur: MyDuration| match signal_rx.recv_timeout(dur.0) {
            Ok(sig) => {
                eprintln!("Got signal {}, closing...", sig);
                Err(format!("Got signal {}", sig).into())
            }
            Err(_timeout) => Ok(()),
        }
    };

    let sub = SubCommand::from_args();

    let config: ParsedGrid = std::env::var("PI_CONFIG")
        .map_err(|err| {
            eprintln!("Please set environment variable PI_CONFIG");
            err.into()
        })
        .and_then(|path| {
            println!("Using PI_CONFIG={}", path);
            mk_grid(&path)
        })?;
    let grid = config.to_grid();

    let mut pi = RaspberryPi::new(config.pi_config)?;

    use SubCommand::*;
    match sub {
        SetPolarity(x) => x.run(&grid, &mut pi, &sleep),
        SetPin(x) => x.run(&grid, &mut pi, &sleep),
        SetLoc(x) => x.run(&grid, &mut pi, &sleep),
        Circle(x) => x.run(&grid, &mut pi, &sleep),
        BackAndForth(x) => x.run(&grid, &mut pi, &sleep),
        ToggleMask(x) => x.run(&grid, &mut pi, &sleep),
    }
}

fn mk_grid(path_str: &str) -> RunResult<ParsedGrid> {
    let path = Path::new(path_str);
    let reader = File::open(path)?;
    debug!("Read config file successfully");
    let grid = ParsedGrid::from_reader(reader)?;
    Ok(grid)
}

fn mk_id(i: usize) -> DropletId {
    DropletId {
        id: i,
        process_id: 42,
    }
}

fn mk_gridview(grid: Grid, blobs: &[SimpleBlob]) -> GridView {
    let mut gv = GridView::new(grid);

    for (i, blob) in blobs.iter().enumerate() {
        let id = mk_id(i);
        gv.droplets.insert(id, blob.to_droplet(id));
    }

    gv
}

#[derive(Debug, StructOpt)]
struct SetPolarity {
    frequency: f64,
    duty_cycle: f64,
    seconds: MyDuration,
}

impl SetPolarity {
    fn run(&self, _: &Grid, pi: &mut RaspberryPi, sleep: &SleepFn) -> RunResult<()> {
        let polarity_config = PolarityConfig {
            frequency: self.frequency,
            duty_cycle: self.duty_cycle,
        };
        pi.hv507.set_polarity(&polarity_config)?;
        sleep(self.seconds)
    }
}

#[derive(Debug, StructOpt)]
struct SetPin {
    pin: usize,
    #[structopt(default_value = "1")]
    seconds: MyDuration,
}

impl SetPin {
    fn run(&self, _: &Grid, pi: &mut RaspberryPi, sleep: &SleepFn) -> RunResult<()> {
        let n = pi.hv507.n_pins();
        if self.pin >= n {
            let s = format!("Pin out of bounds! Should be between 0 and {}.", n);
            return Err(s.into());
        }
        pi.hv507.set_pin_hi(self.pin);
        pi.hv507.shift_and_latch();
        sleep(self.seconds)
    }
}

#[derive(Debug, StructOpt)]
struct SetLoc {
    location: Location,
    #[structopt(default_value = "(1,1)")]
    dimensions: Location,
    #[structopt(default_value = "1")]
    seconds: MyDuration,
}

impl SetLoc {
    fn run(&self, grid: &Grid, pi: &mut RaspberryPi, sleep: &SleepFn) -> RunResult<()> {
        let gv = mk_gridview(
            grid.clone(),
            &[SimpleBlob {
                location: self.location,
                dimensions: self.dimensions,
                volume: 0.0,
            }],
        );
        pi.output_pins(&gv);
        sleep(self.seconds)
    }
}

#[derive(Debug, StructOpt)]
struct Circle {
    location: Location,
    dimensions: Location,
    circle_size: Location,
    #[structopt(default_value = "1")]
    seconds: MyDuration,
}

impl Circle {
    fn run(&self, grid: &Grid, pi: &mut RaspberryPi, sleep: &SleepFn) -> RunResult<()> {
        let mut gv = mk_gridview(
            grid.clone(),
            &[SimpleBlob {
                location: self.location,
                dimensions: self.dimensions,
                volume: 0.0,
            }],
        );
        let id = mk_id(0);

        //     pi.output_pins(&grid, &snapshot);

        let mut set = |yo, xo| {
            let loc = Location {
                y: self.location.y + yo,
                x: self.location.x + xo,
            };
            gv.droplets.get_mut(&id).unwrap().location = loc;
            pi.output_pins(&gv);
            println!("Droplet at {}", loc);
            sleep(self.seconds)
        };

        for xo in 0..self.circle_size.x {
            set(0, xo)?;
        }
        for yo in 0..self.circle_size.y {
            set(yo, self.circle_size.x - 1)?;
        }
        for xo in 0..self.circle_size.x {
            set(self.circle_size.y - 1, self.circle_size.x - 1 - xo)?;
        }
        for yo in 0..self.circle_size.y {
            set(self.circle_size.y - 1 - yo, 0)?;
        }

        Ok(())
    }
}

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
struct BackAndForth {
    #[structopt(short, long, default_value = "2,2")]
    dimensions: Location,
    #[structopt(short, long, default_value = "3")]
    x_distance: i32,
    #[structopt(long, default_value = "1")]
    spacing: u32,
    #[structopt(long, default_value = "1,0")]
    starting_location: Location,
    #[structopt(short, long, default_value = "1")]
    n_droplets: u32,
    #[structopt(short, long, default_value = "1")]
    seconds: MyDuration,
    #[structopt(long, help = "additional seconds to stagger the movement of droplets")]
    stagger: Option<MyDuration>,
}

impl BackAndForth {
    fn run(&self, grid: &Grid, pi: &mut RaspberryPi, sleep: &SleepFn) -> RunResult<()> {
        let blobs: Vec<_> = (0..self.n_droplets)
            .map(|i| {
                let y_offset = (self.dimensions.y + self.spacing as i32) * i as i32;
                let location = self.starting_location + Location { y: y_offset, x: 0 };
                SimpleBlob {
                    location,
                    dimensions: self.dimensions,
                    volume: 0.0,
                }
            })
            .collect();

        let mut gv = mk_gridview(grid.clone(), &blobs);
        let ids: Vec<_> = (0..self.n_droplets).map(|i| mk_id(i as usize)).collect();

        let xs: Vec<i32> = {
            let start = self.starting_location.x;
            let end = start + self.x_distance;
            assert!(end >= 0);

            if start < end {
                let xs = start..end;
                (xs.clone()).chain(xs.rev()).collect()
            } else {
                let xs = end..start;
                (xs.clone().rev()).chain(xs).collect()
            }
        };

        println!("Moving to x's: {:?}", xs);

        for x in xs {
            for id in &ids {
                let droplet = gv.droplets.get_mut(id).unwrap();
                droplet.location.x = x as i32;
                if let Some(stagger) = self.stagger {
                    pi.output_pins(&gv);
                    sleep(stagger)?;
                }
            }
            let locs: Vec<_> = gv.droplets.values().map(|d| d.location).collect();
            pi.output_pins(&gv);
            println!("Droplets at {:?}", locs);

            sleep(self.seconds)?;
        }

        Ok(())
    }
}

fn parse_hex(src: &str) -> Result<u128, std::num::ParseIntError> {
    u128::from_str_radix(src, 16)
}

#[derive(Debug, StructOpt)]
struct ToggleMask {
    #[structopt(parse(try_from_str = "parse_hex"), help = "in hex, but no 0x prefix")]
    mask: u128,
    #[structopt(long, default_value = "1.0")]
    delay: MyDuration,
    #[structopt(long, short = "n", default_value = "10")]
    iterations: usize,
}

impl ToggleMask {
    fn run(&self, _: &Grid, pi: &mut RaspberryPi, sleep: &SleepFn) -> RunResult<()> {
        for i in 0..self.iterations {
            let flip = i & 1;
            for pin in 0..128 {
                let bit = (self.mask >> (127 - pin)) & 1;
                pi.hv507.set_pin(pin, bit as usize == flip);
            }
            pi.hv507.shift_and_latch();
            sleep(self.delay)?;
        }
        Ok(())
    }
}
