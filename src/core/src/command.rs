use grid::gridview::GridView;
use std::fmt::Debug;
use std::sync::mpsc::Sender;

use grid::{DropletId, DropletInfo, Grid, Location};
use exec::Action;

use process::{ProcessId, PuddleResult};

// Send and 'static here are necessary to move trait objects around
// TODO is that necessary
pub trait Command: Debug + Send + 'static {
    fn input_droplets(&self) -> &[DropletId] {
        &[]
    }
    fn output_droplets(&self) -> &[DropletId] {
        &[]
    }
    fn dynamic_info(&self, &GridView) -> DynamicCommandInfo;
    fn is_blocking(&self) -> bool {
        false
    }
    fn trust_placement(&self) -> bool {
        false
    }
}

//
//  Input
//

#[derive(Debug)]
pub struct Input {
    inputs: Vec<DropletId>,
    outputs: Vec<DropletId>,
    location: Vec<Location>,
    dimensions: Vec<Location>,
    volume: f64,
    trusted: bool,
}

#[derive(Debug)]
pub struct DynamicCommandInfo {
    pub shape: Grid,
    pub input_locations: Vec<Location>,
    pub actions: Vec<Action>,
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
            location: vec![loc.unwrap_or(Location { y: 0, x: 0 })],
            dimensions: vec![dim.unwrap_or(Location { y: 1, x: 1 })],
            volume: vol,
            trusted: loc.is_some(),
        })
    }
}

impl Command for Input {
    fn input_droplets(&self) -> &[DropletId] {
        self.inputs.as_slice()
    }

    fn output_droplets(&self) -> &[DropletId] {
        self.outputs.as_slice()
    }

    fn dynamic_info(&self, _gridview: &GridView) -> DynamicCommandInfo {
        let dim = self.dimensions[0];
        let grid = Grid::rectangle(dim.y as usize, dim.x as usize);

        let actions = vec![
            Action::AddDroplet {
                id: self.outputs[0],
                location: self.location[0],
                dimensions: dim,
                volume: self.volume,
            },
            Action::Tick,
        ];

        DynamicCommandInfo {
            shape: grid,
            input_locations: vec![],
            actions: actions,
        }
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
        let actions = vec![
            Action::Ping {
                pid: self.pid,
                tx: self.tx.clone(),
            },
        ];
        DynamicCommandInfo {
            shape: Grid::rectangle(0, 0),
            input_locations: vec![],
            actions: actions,
        }
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
    fn input_droplets(&self) -> &[DropletId] {
        self.inputs.as_slice()
    }

    fn output_droplets(&self) -> &[DropletId] {
        self.outputs.as_slice()
    }

    fn dynamic_info(&self, _gridview: &GridView) -> DynamicCommandInfo {
        let old_id = self.inputs[0];
        let new_id = self.outputs[0];
        let actions = vec![Action::UpdateDroplet { old_id, new_id }];
        DynamicCommandInfo {
            shape: Grid::rectangle(0, 0),
            input_locations: vec![self.destination[0]],
            actions: actions,
        }
    }

    fn trust_placement(&self) -> bool {
        true
    }
}

//
//  Mix
//

lazy_static! {

    static ref MIX_LOOP: Vec<Location> =
        vec![(0,0), (1,0), (1,1), (1,2), (0,2), (0,1), (0,0)]
        .iter()
        .map(|&(y,x)| Location {y, x})
        .collect();
}

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
    fn input_droplets(&self) -> &[DropletId] {
        self.inputs.as_slice()
    }

    fn output_droplets(&self) -> &[DropletId] {
        self.outputs.as_slice()
    }

    fn dynamic_info(&self, gridview: &GridView) -> DynamicCommandInfo {
        let droplets = &gridview.droplets;

        // defines grid shape
        let d0 = droplets.get(&self.inputs[0]).unwrap();
        let d1 = droplets.get(&self.inputs[1]).unwrap();
        let y_dim = (d0.dimensions.y.max(d1.dimensions.y) as usize) + MIX_PADDING;
        let x_dim = (d0.dimensions.x as usize) + (d1.dimensions.x as usize) + MIX_PADDING;
        let grid = Grid::rectangle(y_dim, x_dim);

        let start_d1 = d0.dimensions.x + 1;

        let input_locations = vec![Location { y: 0, x: 0 }, Location { y: 0, x: start_d1 }];

        let out = self.outputs[0];

        // this first set of actions moves d1 into d0 and performs the combine
        // we cannot tick in between; it would cause a collision
        let mut acts = vec![];

        // move the right droplet one to the left
        acts.push(Action::MoveDroplet {
            id: self.inputs[1],
            location: Location {
                y: 0,
                x: start_d1 - 1,
            },
        });

        acts.push(Action::Mix {
            in0: self.inputs[0],
            in1: self.inputs[1],
            out: out,
        });
        acts.push(Action::Tick);

        for loc in MIX_LOOP.iter() {
            acts.push(Action::MoveDroplet {
                id: out,
                location: *loc,
            });
            acts.push(Action::Tick);
        }

        DynamicCommandInfo {
            shape: grid,
            input_locations: input_locations,
            actions: acts,
        }
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
    fn input_droplets(&self) -> &[DropletId] {
        self.inputs.as_slice()
    }

    fn output_droplets(&self) -> &[DropletId] {
        self.outputs.as_slice()
    }

    fn dynamic_info(&self, gridview: &GridView) -> DynamicCommandInfo {
        let droplets = &gridview.droplets;
        let d0 = droplets.get(&self.inputs[0]).unwrap();
        // we only split in the x right now, so we don't need y padding
        let x_dim = (d0.dimensions.x as usize) + SPLIT_PADDING;
        let y_dim = d0.dimensions.y as usize;
        let grid = Grid::rectangle(y_dim, x_dim);

        let input_locations = vec![Location { y: 0, x: 2 }];

        let inp = self.inputs[0];
        let mut acts = vec![
            Action::Split {
                inp: inp,
                out0: self.outputs[0],
                out1: self.outputs[1],
            },
        ];

        let mid = (x_dim / 2) as i32;

        // NOTE
        // we cannot tick here because a collision will happen!
        // only tick after they've moved apart
        // TODO incorporate this into split somehow?
        for dim in 1..(SPLIT_PADDING as i32 / 2 + 1) {
            acts.push(Action::MoveDroplet {
                id: self.outputs[0],
                // this droplets starts at offset 2, moving left
                location: Location { y: 0, x: 2 - dim },
            });
            acts.push(Action::MoveDroplet {
                id: self.outputs[1],
                // this droplets starts at offset mid, moving right
                location: Location { y: 0, x: mid + dim },
            });
            acts.push(Action::Tick);
        }

        DynamicCommandInfo {
            shape: grid,
            input_locations: input_locations,
            actions: acts,
        }
    }
}
