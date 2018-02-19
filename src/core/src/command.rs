use std::fmt::Debug;
use std::sync::mpsc::Sender;

use grid::{DropletId, Grid, Location};
use exec::Action;

use process::PuddleResult;

static EMPTY_IDS: &[DropletId] = &[];
static EMPTY_LOCATIONS: &[Location] = &[];

// Send and 'static here are necessary to move trait objects around
// TODO is that necessary
pub trait Command: Debug + Send + 'static {
    fn input_droplets(&self) -> &[DropletId] {
        EMPTY_IDS
    }
    fn input_locations(&self) -> &[Location] {
        EMPTY_LOCATIONS
    }
    fn output_droplets(&self) -> &[DropletId] {
        EMPTY_IDS
    }
    fn output_locations(&self) -> &[Location] {
        EMPTY_LOCATIONS
    }
    fn shape(&self) -> &Grid;
    fn actions(&self) -> Vec<Action>;

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

lazy_static! {
    static ref INPUT_SHAPE: Grid = Grid::rectangle(1,1);
    static ref INPUT_INPUT_LOCS: Vec<Location> = vec![];
}

#[derive(Debug)]
pub struct Input {
    inputs: Vec<DropletId>,
    outputs: Vec<DropletId>,
    location: Vec<Location>,
    trusted: bool,
}

impl Input {
    pub fn new(loc: Option<Location>, out_id: DropletId) -> PuddleResult<Input> {
        Ok(Input {
            inputs: vec![],
            outputs: vec![out_id],
            location: vec![loc.unwrap_or(Location { y: 0, x: 0 })],
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

    fn input_locations(&self) -> &[Location] {
        INPUT_INPUT_LOCS.as_slice()
    }

    fn output_locations(&self) -> &[Location] {
        &self.location
    }

    fn shape(&self) -> &Grid {
        &INPUT_SHAPE
    }

    fn trust_placement(&self) -> bool {
        self.trusted
    }

    fn actions(&self) -> Vec<Action> {
        vec![
            Action::AddDroplet {
                id: self.outputs[0],
                location: self.location[0],
            },
            Action::Tick,
        ]
    }
}

//
// Flush
//

lazy_static! {
    /// Flush shape is just empty because it doesn't need to be placed
    static ref FLUSH_SHAPE: Grid = Grid::rectangle(0,0);
}

#[derive(Debug)]
pub struct Flush {
    tx: Sender<()>,
}

impl Flush {
    pub fn new(tx: Sender<()>) -> Flush {
        Flush { tx }
    }
}

impl Command for Flush {
    fn shape(&self) -> &Grid {
        &FLUSH_SHAPE
    }
    fn actions(&self) -> Vec<Action> {
        vec![
            Action::Ping {
                tx: self.tx.clone(),
            },
        ]
    }
}

//
//  Move
//

lazy_static! {
    static ref MOVE_SHAPE: Grid = Grid::rectangle(0,0);
}

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

    fn input_locations(&self) -> &[Location] {
        &self.destination
    }

    fn output_locations(&self) -> &[Location] {
        &self.destination
    }

    fn shape(&self) -> &Grid {
        &MOVE_SHAPE
    }

    fn trust_placement(&self) -> bool {
        true
    }

    fn actions(&self) -> Vec<Action> {
        let old_id = self.inputs[0];
        let new_id = self.outputs[0];
        vec![Action::UpdateDroplet { old_id, new_id }]
    }
}

//
//  Mix
//

lazy_static! {
    static ref MIX_SHAPE: Grid = Grid::rectangle(3,2);
    static ref MIX_INPUT_LOCS: Vec<Location> =
        vec![
            Location {y: 0, x: 0},
            Location {y: 2, x: 0},
        ];
    static ref MIX_OUTPUT_LOCS: Vec<Location> =
        vec![
            Location {y: 0, x: 0},
        ];
    static ref MIX_LOOP: Vec<Location> =
        vec![(0,0), (0,1), (1,1), (2,1), (2,0), (1,0), (0,0)]
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

impl Command for Mix {
    fn input_droplets(&self) -> &[DropletId] {
        self.inputs.as_slice()
    }

    fn output_droplets(&self) -> &[DropletId] {
        self.outputs.as_slice()
    }

    fn input_locations(&self) -> &[Location] {
        MIX_INPUT_LOCS.as_slice()
    }

    fn output_locations(&self) -> &[Location] {
        MIX_OUTPUT_LOCS.as_slice()
    }

    fn shape(&self) -> &Grid {
        &MIX_SHAPE
    }

    fn actions(&self) -> Vec<Action> {
        let out = self.outputs[0];
        // this first set of actions moves d1 into d0 and performs the combine
        // we cannot tick in between; it would cause a collision
        let mut acts = vec![
            Action::MoveDroplet {
                id: self.inputs[1],
                location: Location { y: 1, x: 0 },
            },
            Action::MoveDroplet {
                id: self.inputs[1],
                location: Location { y: 0, x: 0 },
            },
            Action::Mix {
                in0: self.inputs[0],
                in1: self.inputs[1],
                out: out,
            },
            Action::Tick,
        ];

        for loc in MIX_LOOP.iter() {
            acts.push(Action::MoveDroplet {
                id: out,
                location: *loc,
            });
            acts.push(Action::Tick);
        }
        acts
    }
}

//
//  Split
//

lazy_static! {
    static ref SPLIT_SHAPE: Grid = Grid::rectangle(1,5);
    static ref SPLIT_INPUT_LOCS: Vec<Location> =
        vec![
            Location {y: 0, x: 2},
        ];
    static ref SPLIT_OUTPUT_LOCS: Vec<Location> =
        vec![
            Location {y: 0, x: 0},
            Location {y: 0, x: 4},
        ];
    static ref SPLIT_PATH0: Vec<Location> =
        vec![
            Location {y: 0, x: 1},
            Location {y: 0, x: 0},
        ];
    static ref SPLIT_PATH1: Vec<Location> =
        vec![
            Location {y: 0, x: 3},
            Location {y: 0, x: 4},
        ];
}

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

impl Command for Split {
    fn input_droplets(&self) -> &[DropletId] {
        self.inputs.as_slice()
    }

    fn output_droplets(&self) -> &[DropletId] {
        self.outputs.as_slice()
    }

    fn input_locations(&self) -> &[Location] {
        SPLIT_INPUT_LOCS.as_slice()
    }

    fn output_locations(&self) -> &[Location] {
        SPLIT_OUTPUT_LOCS.as_slice()
    }

    fn shape(&self) -> &Grid {
        &SPLIT_SHAPE
    }

    fn actions(&self) -> Vec<Action> {
        let inp = self.inputs[0];
        let mut acts = vec![
            Action::Split {
                inp: inp,
                out0: self.outputs[0],
                out1: self.outputs[1],
            },
        ];

        // NOTE
        // we cannot tick here because a collision will happen!
        // only tick after they've moved apart
        // TODO incorporate this into split somehow?

        for (l0, l1) in SPLIT_PATH0.iter().zip(SPLIT_PATH1.iter()) {
            acts.push(Action::MoveDroplet {
                id: self.outputs[0],
                location: *l0,
            });
            acts.push(Action::MoveDroplet {
                id: self.outputs[1],
                location: *l1,
            });
            acts.push(Action::Tick);
        }

        acts
    }
}
