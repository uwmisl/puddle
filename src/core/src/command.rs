use grid::gridview::{GridSubView, GridView};
use std::fmt;
use std::sync::mpsc::Sender;
use std::time::Duration;

use grid::{Droplet, DropletId, DropletInfo, Grid, Location, Peripheral, Snapshot};

use process::{ProcessId, PuddleResult};

pub trait Command: fmt::Debug + Send {
    fn input_droplets(&self) -> Vec<DropletId> {
        vec![]
    }
    fn output_droplets(&self) -> Vec<DropletId> {
        vec![]
    }
    fn bypass(&self, _gridview: &GridView) -> bool {
        false
    }
    fn dynamic_info(&self, &GridView) -> DynamicCommandInfo;
    fn run(&self, &mut GridSubView);
    fn is_blocking(&self) -> bool {
        false
    }
    fn trust_placement(&self) -> bool {
        false
    }
    fn finalize(&mut self, &Snapshot) {}
}

//
//  Input
//

#[derive(Debug)]
pub struct Input {
    inputs: Vec<DropletId>,
    outputs: Vec<DropletId>,
    location: Location,
    dimensions: Location,
    volume: f64,
    trusted: bool,
}

#[derive(Debug)]
pub struct DynamicCommandInfo {
    pub shape: Grid,
    pub input_locations: Vec<Location>,
}

// TODO: dimensions probably shouldn't be optional?
impl Input {
    pub fn new(
        loc: Option<Location>,
        vol: f64,
        dim: Option<Location>,
        out_id: DropletId,
    ) -> PuddleResult<Input> {
        Ok(Input {
            inputs: vec![],
            outputs: vec![out_id],
            location: loc.unwrap_or(Location { y: 0, x: 0 }),
            dimensions: dim.unwrap_or(Location { y: 1, x: 1 }),
            volume: vol,
            trusted: loc.is_some(),
        })
    }
}

impl Command for Input {
    fn input_droplets(&self) -> Vec<DropletId> {
        self.inputs.clone()
    }

    fn output_droplets(&self) -> Vec<DropletId> {
        self.outputs.clone()
    }

    fn dynamic_info(&self, _gridview: &GridView) -> DynamicCommandInfo {
        let grid = Grid::rectangle(self.dimensions.y as usize, self.dimensions.x as usize);

        DynamicCommandInfo {
            shape: grid,
            input_locations: vec![],
        }
    }

    fn run(&self, gridview: &mut GridSubView) {
        gridview.insert(Droplet::new(
            self.outputs[0],
            self.volume,
            self.location,
            self.dimensions,
        ));
        gridview.tick();
    }

    fn trust_placement(&self) -> bool {
        self.trusted
    }
}

//
// Flush
//

#[derive(Debug)]
pub struct Flush {
    pid: ProcessId,
    tx: Sender<Vec<DropletInfo>>,
}

impl Flush {
    pub fn new(pid: ProcessId, tx: Sender<Vec<DropletInfo>>) -> Flush {
        Flush { pid, tx }
    }
}

impl Command for Flush {
    fn dynamic_info(&self, _gridview: &GridView) -> DynamicCommandInfo {
        DynamicCommandInfo {
            shape: Grid::rectangle(0, 0),
            input_locations: vec![],
        }
    }

    fn run(&self, gridview: &mut GridSubView) {
        gridview.tick();
    }

    fn finalize(&mut self, gv: &Snapshot) {
        let info = gv.droplet_info(Some(self.pid));
        self.tx.send(info).unwrap();
    }
}

//
//  Move
//

#[derive(Debug)]
pub struct Move {
    inputs: Vec<DropletId>,
    outputs: Vec<DropletId>,
    destination: [Location; 1],
}

impl Move {
    pub fn new(in_id: DropletId, loc: Location, out_id: DropletId) -> PuddleResult<Move> {
        Ok(Move {
            inputs: vec![in_id],
            outputs: vec![out_id],
            destination: [loc],
        })
    }
}

impl Command for Move {
    fn input_droplets(&self) -> Vec<DropletId> {
        self.inputs.clone()
    }

    fn output_droplets(&self) -> Vec<DropletId> {
        self.outputs.clone()
    }

    fn dynamic_info(&self, gridview: &GridView) -> DynamicCommandInfo {
        let old_id = self.inputs[0];
        let dim = gridview.snapshot().droplets[&old_id].dimensions;
        DynamicCommandInfo {
            shape: Grid::rectangle(dim.y as usize, dim.x as usize),
            input_locations: vec![self.destination[0]],
        }
    }

    fn run(&self, gridview: &mut GridSubView) {
        let old_id = self.inputs[0];
        let new_id = self.outputs[0];
        let mut d = gridview.remove(&old_id);
        // NOTE this is pretty much the only place it's ok to change an id
        d.id = new_id;
        gridview.insert(d);
        gridview.tick()
    }

    fn trust_placement(&self) -> bool {
        true
    }
}

//
//  Mix
//

#[derive(Debug)]
pub struct Mix {
    inputs: Vec<DropletId>,
    outputs: Vec<DropletId>,
}

impl Mix {
    pub fn new(id1: DropletId, id2: DropletId, out_id: DropletId) -> PuddleResult<Mix> {
        Ok(Mix {
            inputs: vec![id1, id2],
            outputs: vec![out_id],
        })
    }
}

const MIX_PADDING: usize = 1;

impl Command for Mix {
    fn input_droplets(&self) -> Vec<DropletId> {
        self.inputs.clone()
    }

    fn output_droplets(&self) -> Vec<DropletId> {
        self.outputs.clone()
    }

    fn bypass(&self, gridview: &GridView) -> bool {
        let droplets = &gridview.snapshot().droplets;
        if droplets.contains_key(&self.outputs[0]) {
            assert!(!droplets.contains_key(&self.inputs[0]));
            assert!(!droplets.contains_key(&self.inputs[1]));
            true
        } else {
            false
        }
    }

    fn dynamic_info(&self, gridview: &GridView) -> DynamicCommandInfo {
        let droplets = &gridview.snapshot().droplets;

        // define the grid shape now based on the droplets in the *predicted* gridview
        let (grid, input_locations) = {
            let d0 = droplets.get(&self.inputs[0]).unwrap();
            let d1 = droplets.get(&self.inputs[1]).unwrap();
            let y_dim = (d0.dimensions.y.max(d1.dimensions.y) as usize) + MIX_PADDING;
            let x_dim = (d0.dimensions.x as usize) + (d1.dimensions.x as usize) + MIX_PADDING;

            let start_d1 = d0.dimensions.x + 1;

            (
                Grid::rectangle(y_dim, x_dim),
                vec![Location { y: 0, x: 0 }, Location { y: 0, x: start_d1 }],
            )
        };

        DynamicCommandInfo {
            shape: grid,
            input_locations: input_locations,
        }
    }

    fn run(&self, gridview: &mut GridSubView) {
        let in0 = self.inputs[0];
        let in1 = self.inputs[1];
        let out = self.outputs[0];

        // this first set of actions moves d1 into d0 and performs the combine
        // we cannot tick in between; it would cause a collision
        gridview.move_west(in1);

        let d0 = gridview.remove(&in0);
        let d1 = gridview.remove(&in1);
        let vol = d0.volume + d1.volume;
        // TODO right now this only mixes horizontally
        // it should somehow communicate with the Mix command to control the mixed droplets dimensions
        let dim = Location {
            y: d0.dimensions.y.max(d1.dimensions.y),
            x: d0.dimensions.x + d1.dimensions.x,
        };
        assert_eq!(d0.location.y, d1.location.y);
        assert_eq!(d0.location.x + d0.dimensions.x, d1.location.x);
        gridview.insert(Droplet::new(out, vol, d0.location, dim));

        gridview.tick();
        gridview.move_south(out);
        gridview.tick();
        gridview.move_east(out);
        gridview.tick();
        gridview.move_north(out);
        gridview.tick();
        gridview.move_west(out);
        gridview.tick();
    }
}

//
//  Split
//

#[derive(Debug)]
pub struct Split {
    inputs: Vec<DropletId>,
    outputs: Vec<DropletId>,
}

impl Split {
    pub fn new(id: DropletId, out_id1: DropletId, out_id2: DropletId) -> PuddleResult<Split> {
        Ok(Split {
            inputs: vec![id],
            outputs: vec![out_id1, out_id2],
        })
    }
}

const SPLIT_PADDING: usize = 4;

impl Command for Split {
    fn input_droplets(&self) -> Vec<DropletId> {
        self.inputs.clone()
    }

    fn output_droplets(&self) -> Vec<DropletId> {
        self.outputs.clone()
    }

    fn bypass(&self, gridview: &GridView) -> bool {
        let droplets = &gridview.snapshot().droplets;
        // if it has one, it better have both
        if droplets.contains_key(&self.outputs[0]) || droplets.contains_key(&self.outputs[1]) {
            assert!(droplets.contains_key(&self.outputs[0]));
            assert!(droplets.contains_key(&self.outputs[1]));
            assert!(!droplets.contains_key(&self.inputs[0]));
            true
        } else {
            false
        }
    }

    fn dynamic_info(&self, gridview: &GridView) -> DynamicCommandInfo {
        let droplets = &gridview.snapshot().droplets;
        let d0 = droplets.get(&self.inputs[0]).unwrap();
        // we only split in the x right now, so we don't need y padding
        let x_dim = (d0.dimensions.x as usize) + SPLIT_PADDING;
        let y_dim = d0.dimensions.y as usize;
        let grid = Grid::rectangle(y_dim, x_dim);

        let input_locations = vec![Location { y: 0, x: 2 }];

        DynamicCommandInfo {
            shape: grid,
            input_locations: input_locations,
        }
    }

    fn run(&self, gridview: &mut GridSubView) {
        let x_dim = {
            // limit the scope of d0 borrow
            let d0 = gridview.get(&self.inputs[0]);
            (d0.dimensions.x as usize) + SPLIT_PADDING
        };

        let inp = self.inputs[0];
        let out0 = self.outputs[0];
        let out1 = self.outputs[1];

        let d = gridview.remove(&inp);
        let vol = d.volume / 2.0;

        // TODO: this should be related to volume in some fashion
        // currently, take the ceiling of the division of the split by two
        let dim = Location {
            y: d.dimensions.y,
            x: (d.dimensions.x + 1) / 2,
        };

        let loc0 = Location { y: 0, x: 1 };
        let loc1 = Location {
            y: 0,
            x: x_dim as i32 - (dim.x + 1),
        };

        gridview.insert(Droplet::new(out0, vol, loc0, dim));
        gridview.insert(Droplet::new(out1, vol, loc1, dim));

        gridview.tick();
        gridview.move_west(out0);
        gridview.move_east(out1);
        gridview.tick();
    }
}

#[derive(Debug)]
pub struct Heat {
    inputs: Vec<DropletId>,
    outputs: Vec<DropletId>,
    temperature: f32,
}

impl Heat {
    pub fn new(id: DropletId, out_id: DropletId, temperature: f32) -> PuddleResult<Heat> {
        Ok(Heat {
            inputs: vec![id],
            outputs: vec![out_id],
            temperature,
        })
    }
}

impl Command for Heat {
    fn input_droplets(&self) -> Vec<DropletId> {
        self.inputs.clone()
    }

    fn output_droplets(&self) -> Vec<DropletId> {
        self.outputs.clone()
    }

    fn dynamic_info(&self, gridview: &GridView) -> DynamicCommandInfo {
        let droplets = &gridview.snapshot().droplets;
        let d = droplets.get(&self.inputs[0]).unwrap();
        // we only split in the x right now, so we don't need y padding
        let x_dim = d.dimensions.x as usize;
        let y_dim = d.dimensions.y as usize;

        // right now we can only heat droplets that are 1x1
        assert_eq!(y_dim, 1);
        assert_eq!(x_dim, 1);
        let mut grid = Grid::rectangle(y_dim, x_dim);

        // the parameters of heater here don't matter, as it's just used to
        // match up with the "real" heater in the actual grid
        let loc = Location { y: 0, x: 0 };
        grid.get_cell_mut(&loc).unwrap().peripheral = Some(Peripheral::Heater {
            pwm_channel: 0,
            spi_channel: 0,
        });

        let input_locations = vec![loc];

        DynamicCommandInfo {
            shape: grid,
            input_locations: input_locations,
        }
    }

    fn run(&self, gridview: &mut GridSubView) {
        #[cfg(feature = "pi")]
        {
            let loc = Location { y: 0, x: 0 };
            let temperature = 50.0;
            let duration = Duration::from_secs(1);
            let heater = gridview
                .get_electrode(&loc)
                .unwrap()
                .peripheral
                .unwrap()
                .clone();
            assert_matches!(heater, Peripheral::Heater{..});
            gridview.with_pi(|pi| pi.heat(&heater, temperature, duration));
        }
        let old_id = self.inputs[0];
        let new_id = self.outputs[0];

        let mut d = gridview.remove(&old_id);
        // NOTE this is a rare place it's ok to change an id, like move
        d.id = new_id;
        gridview.insert(d);
        gridview.tick()
    }
}
