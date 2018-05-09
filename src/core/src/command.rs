use grid::gridview::{GridSubView, RootGridView};
use std::fmt;
use std::sync::mpsc::Sender;

use rand::Rng;

use grid::{Droplet, DropletId, DropletInfo, Grid, Location};

use process::{ProcessId, PuddleResult};

pub trait Command: fmt::Debug + Send {
    fn input_droplets(&self) -> Vec<DropletId> {
        vec![]
    }
    fn output_droplets(&self) -> Vec<DropletId> {
        vec![]
    }
    fn dynamic_info(&self, &RootGridView) -> DynamicCommandInfo;
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
    location: Location,
    dimensions: Location,
    volume: f64,
    trusted: bool,
}

#[derive(Debug)]
pub struct DynamicCommandInfo<'a> {
    pub shape: Grid,
    pub input_locations: Vec<Location>,
    pub actions: Vec<Action<'a>>,
}

pub struct Action<'a> {
    pub func: Box<'a + Fn(&mut GridSubView)>,
    pub description: &'static str,
}

impl<'a> fmt::Debug for Action<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Action {{ {} }}", self.description)
    }
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

fn mk_act<'a, F: 'a + Fn(&mut GridSubView)>(desc: &'static str, func: F) -> Action<'a> {
    Action {
        description: desc,
        func: Box::new(func),
    }
}

impl Command for Input {
    fn input_droplets(&self) -> Vec<DropletId> {
        self.inputs.clone()
    }

    fn output_droplets(&self) -> Vec<DropletId> {
        self.outputs.clone()
    }

    fn dynamic_info(&self, _gridview: &RootGridView) -> DynamicCommandInfo {
        let grid = Grid::rectangle(self.dimensions.y as usize, self.dimensions.x as usize);

        let actions = vec![
            mk_act("input droplet", move |gv| {
                gv.insert(Droplet::new(
                    self.outputs[0],
                    self.volume,
                    self.location,
                    self.dimensions,
                ))
            }),
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
    fn dynamic_info(&self, _gridview: &RootGridView) -> DynamicCommandInfo {
        let actions = vec![
            mk_act("flush", move |gv| {
                if gv.is_exec() {
                    let info = gv.droplet_info(Some(self.pid));
                    self.tx.send(info).unwrap();
                }
            }),
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
    fn input_droplets(&self) -> Vec<DropletId> {
        self.inputs.clone()
    }

    fn output_droplets(&self) -> Vec<DropletId> {
        self.outputs.clone()
    }

    fn dynamic_info(&self, gridview: &RootGridView) -> DynamicCommandInfo {
        let old_id = self.inputs[0];
        let new_id = self.outputs[0];
        let dim = gridview.droplets[&old_id].dimensions;
        let actions = vec![
            mk_act("change droplet id for move", move |gv| {
                let mut d = gv.remove(old_id);
                // NOTE this is pretty much the only place it's ok to change an id
                d.id = new_id;
                gv.insert(d);
            }),
        ];
        DynamicCommandInfo {
            shape: Grid::rectangle(dim.y as usize, dim.x as usize),
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

    fn dynamic_info(&self, gridview: &RootGridView) -> DynamicCommandInfo {
        let droplets = &gridview.droplets;

        let in0 = self.inputs[0];
        let in1 = self.inputs[1];
        let out = self.outputs[0];

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

        // this first set of actions moves d1 into d0 and performs the combine
        // we cannot tick in between; it would cause a collision
        let acts = vec![
            mk_act("do mix", move |gv| {
                gv.move_west(in1);

                let d0 = gv.remove(in0);
                let d1 = gv.remove(in1);
                let vol = d0.volume + d1.volume;
                // TODO right now this only mixes horizontally
                // it should somehow communicate with the Mix command to control the mixed droplets dimensions
                let dim = Location {
                    y: d0.dimensions.y.max(d1.dimensions.y),
                    x: d0.dimensions.x + d1.dimensions.x,
                };
                assert_eq!(d0.location.y, d1.location.y);
                assert_eq!(d0.location.x + d0.dimensions.x, d1.location.x);
                gv.insert(Droplet::new(out, vol, d0.location, dim));
            }),
            mk_act("move", move |gv| {
                gv.move_south(out);
            }),
            mk_act("move", move |gv| {
                gv.move_east(out);
            }),
            mk_act("move", move |gv| {
                gv.move_north(out);
            }),
            mk_act("move", move |gv| {
                gv.move_west(out);
            }),
        ];

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
    fn input_droplets(&self) -> Vec<DropletId> {
        self.inputs.clone()
    }

    fn output_droplets(&self) -> Vec<DropletId> {
        self.outputs.clone()
    }

    fn dynamic_info(&self, gridview: &RootGridView) -> DynamicCommandInfo {
        let d0 = gridview.droplets.get(&self.inputs[0]).unwrap();
        // we only split in the x right now, so we don't need y padding
        let x_dim = (d0.dimensions.x as usize) + SPLIT_PADDING;
        let y_dim = d0.dimensions.y as usize;
        let grid = Grid::rectangle(y_dim, x_dim);

        let input_locations = vec![Location { y: 0, x: 2 }];

        let inp = self.inputs[0];
        let out0 = self.outputs[0];
        let out1 = self.outputs[1];

        let acts = vec![
            mk_act("split", move |gv| {
                let d = gv.remove(inp);
                let vol = d.volume / 2.0;
                // create the error and clamp it to reasonable values
                let err = gv.backing_gridview.split_error_stdev.map_or(0.0, |dist| {
                    gv.backing_gridview
                        .rng
                        .sample(dist)
                        .min(d.volume)
                        .max(-d.volume)
                });

                // TODO: this should be related to volume in some fashion
                // currently, take the ceiling of the division of the split by two
                let dim = Location {
                    y: d.dimensions.y,
                    x: (d.dimensions.x + 1) / 2,
                };

                let vol0 = vol - err;
                let vol1 = vol + err;

                let loc0 = Location { y: 0, x: 1 };
                let loc1 = Location {
                    y: 0,
                    x: x_dim as i32 - (dim.x + 1),
                };

                gv.insert(Droplet::new(out0, vol0, loc0, dim));
                gv.insert(Droplet::new(out1, vol1, loc1, dim));
            }),
            // TODO: right now we only move once
            // this should depend on SPLIT_PADDING
            mk_act("split move", move |gv| {
                gv.move_west(out0);
                gv.move_east(out1);
            }),
        ];

        DynamicCommandInfo {
            shape: grid,
            input_locations: input_locations,
            actions: acts,
        }
    }
}
