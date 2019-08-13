use std::error::Error;
use std::time::Instant;

use config::{Config, Environment, File};
use log::*;

use puddle_core::{
    grid::droplet::{Blob, DropletId, SimpleBlob},
    grid::gridview::GridView,
    grid::location::yx,
    grid::parse::ParsedGrid,
    grid::{Grid, Location},
    util::seconds_duration,
};
use puddle_pi::{RaspberryPi, Settings};

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

static CUSTOM_HELP: &str = r#"./pi-test custom < file.txt

Feed a file that looks like the following into stdin:

oo
---
oooo
---
o  o

The only allow characters in a segment are ' ' and 'o'.
A line beginning with '-' starts the next segement.
"#;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
#[structopt(raw(about = r#"env!("PI_TEST_ABOUT")"#))]
enum SubCommand {
    SetPolarity(SetPolarity),
    SetPin(SetPin),
    SetLoc(SetLoc),
    Circle(Circle),
    BackAndForth(BackAndForth),
    ToggleMask(ToggleMask),
    Split(Split),
    #[structopt(raw(usage = "CUSTOM_HELP"))]
    Custom(Custom),
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

    let conf_path = std::env::var("PI_CONFIG").map_err(|err| {
        eprintln!("Please set environment variable PI_CONFIG");
        err
    })?;
    println!("Using PI_CONFIG={}", conf_path);

    let mut conf = Config::new();
    conf.merge(File::with_name(&conf_path))?;
    conf.merge(Environment::new().separator("__"))?;
    let settings = Settings::from_config(&mut conf)?;
    debug!("Settings made!");

    let mut pi = RaspberryPi::new(settings)?;
    debug!("Pi made!");

    let parsed_grid: ParsedGrid = conf.try_into()?;
    let grid = parsed_grid.into();
    debug!("Grid made!");

    use SubCommand::*;
    match sub {
        SetPolarity(x) => x.run(&grid, &mut pi, &sleep),
        SetPin(x) => x.run(&grid, &mut pi, &sleep),
        SetLoc(x) => x.run(&grid, &mut pi, &sleep),
        Circle(x) => x.run(&grid, &mut pi, &sleep),
        BackAndForth(x) => x.run(&grid, &mut pi, &sleep),
        ToggleMask(x) => x.run(&grid, &mut pi, &sleep),
        Split(x) => x.run(&grid, &mut pi, &sleep),
        Custom(x) => x.run(&grid, &mut pi, &sleep),
    }
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

fn blob(location: Location, dimensions: Location) -> SimpleBlob {
    SimpleBlob {
        location,
        dimensions,
        volume: 0.0,
    }
}

#[derive(Debug, StructOpt)]
struct SetPolarity {
    frequency: f64,
    duty_cycle: f64,
    seconds: MyDuration,
}

impl SetPolarity {
    fn run(&self, _: &Grid, pi: &mut RaspberryPi, sleep: &SleepFn) -> RunResult<()> {
        pi.hv507.set_polarity(self.frequency, self.duty_cycle)?;
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
        let gv = mk_gridview(grid.clone(), &[blob(self.location, self.dimensions)]);
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
        let mut gv = mk_gridview(grid.clone(), &[blob(self.location, self.dimensions)]);
        let id = mk_id(0);

        //     pi.output_pins(&grid, &snapshot);

        let mut set = |yo, xo| {
            let loc = self.location + yx(yo, xo);
            gv.droplets.get_mut(&id).unwrap().location = loc;
            let start = Instant::now();
            pi.output_pins(&gv);
            print!("Droplet at {}...", loc);
            let res = sleep(self.seconds);
            println!("{:?}", start.elapsed());
            res
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
                let location = self.starting_location + yx(y_offset, 0);
                blob(location, self.dimensions)
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

#[derive(Debug, StructOpt)]
struct Split {
    #[structopt(short, long, default_value = "1")]
    delay: MyDuration,
    location: Location,
    dimensions: Location,
    #[structopt(subcommand)]
    kind: SplitKind,
}

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
enum SplitKind {
    Side,
    Diag,
}

impl Split {
    fn run(&self, grid: &Grid, pi: &mut RaspberryPi, sleep: &SleepFn) -> RunResult<()> {
        let mut gv = mk_gridview(grid.clone(), &[]);

        let loc0 = self.location;
        let dim0 = self.dimensions;

        let mut set = |blobs: &[SimpleBlob]| {
            gv.droplets.clear();
            for (i, blob) in blobs.iter().enumerate() {
                let id = mk_id(i);
                gv.droplets.insert(id, blob.to_droplet(id));
            }
            pi.output_pins(&gv);
            let locs: Vec<_> = blobs.iter().map(|b| (b.location.y, b.location.x)).collect();
            println!("Droplets at {:?}", locs);
            sleep(self.delay)
        };

        let pair = (&self.kind, (self.dimensions.y, self.dimensions.x));
        match pair {
            (SplitKind::Side, (1, 1)) => {
                let b = |x| blob(loc0 + yx(0, x), yx(1, 1));
                set(&[b(0)])?;
                set(&[b(0), b(1)])?;
                set(&[b(-1), b(1)])?;
                set(&[b(-2), b(2)])?;
            }
            (SplitKind::Diag, (1, 1)) => {
                let b = |y, x| blob(loc0 + yx(y, x), yx(1, 1));
                set(&[b(0, 0)])?;
                set(&[b(0, 0), b(0, 1)])?;
                set(&[b(-1, 0), b(1, 1)])?;
            }
            (SplitKind::Side, (1, 2)) => {
                let b = |x| blob(loc0 + yx(0, x), yx(1, 1));
                set(&[blob(loc0, dim0)])?;
                set(&[blob(loc0 + yx(0, -1), yx(1, 4))])?;
                set(&[b(-1), b(2)])?;
            }
            (SplitKind::Side, (2, 2)) => {
                let b = |x| blob(loc0 + yx(0, x), yx(2, 1));
                set(&[blob(loc0, dim0)])?;
                set(&[blob(loc0 + yx(0, -1), yx(2, 4))])?;
                set(&[b(-1), b(2)])?;
            }
            _ => panic!("Unsupported args: {:?}", pair),
        }

        Ok(())
    }
}

#[derive(Debug, StructOpt)]
struct Custom {
    #[structopt(short, long, default_value = "1")]
    delay: MyDuration,
    #[structopt(short, long, default_value = "1,0")]
    offset: Location,
}

impl Custom {
    fn run(&self, grid: &Grid, pi: &mut RaspberryPi, sleep: &SleepFn) -> RunResult<()> {
        let mut gv = mk_gridview(grid.clone(), &[]);
        let mut y = 0;
        let mut locations = Vec::new();

        use std::io::{stdin, BufRead};
        let stdin = stdin();

        let mut go = |locations: &mut Vec<Location>| {
            gv.droplets.clear();
            for (i, loc) in locations.iter().enumerate() {
                let id = mk_id(i);
                let droplet = blob(*loc, yx(1, 1)).to_droplet(id);
                gv.droplets.insert(id, droplet);
            }
            pi.output_pins(&gv);
            let locs: Vec<_> = locations.iter().map(|l| (l.y, l.x)).collect();
            println!("Droplets at {:?}", locs);
            locations.clear();
            sleep(self.delay)
        };

        for line in stdin.lock().lines() {
            let line = line?;
            if line.starts_with('-') {
                y = -1;
                go(&mut locations)?;
            } else {
                for (i, c) in line.char_indices() {
                    match c {
                        'o' => locations.push(yx(y, i as i32) + self.offset),
                        ' ' => (),
                        _ => Err(format!("Unsupported character: '{}'", c))?,
                    }
                }
            }
            y += 1;
        }
        go(&mut locations)?;
        Ok(())
    }
}
