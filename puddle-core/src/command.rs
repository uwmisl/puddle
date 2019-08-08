use std::fmt;
use std::time::Duration;

use crate::plan::PlanError;

use crate::grid::{
    gridview::{GridSubView, GridView},
    location::yx,
    Blob, Droplet, DropletId, Grid, Location, Peripheral, SimpleBlob,
};

use crate::process::PuddleResult;

#[derive(Debug)]
pub struct CommandRequest {
    pub name: String,
    pub shape: Grid,
    pub input_locations: Vec<Location>,
    // TODO needed to plan ahead, but we can omit if we don't do that for now
    // pub outputs: Vec<Droplet>,
    pub offset: Option<Location>,
}

pub enum RunStatus {
    Done,
    KeepGoing,
}

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

    fn request(&self, gridview: &GridView) -> CommandRequest;

    // FIXME this is definitely a hack for combining droplets
    // run before the final routing tick that
    // this better not tick!!!
    fn pre_run(&self, _: &mut GridSubView) {}

    fn run(&mut self, _: &mut GridSubView) -> RunStatus;

    fn finalize(&mut self, _: &GridSubView) {}

    fn abort(&mut self, err: PlanError) {
        error!("Aborting command {:?} with {:#?}", self, err);
    }
}

pub type BoxedCommand = Box<dyn Command>;

//
//  Create
//

#[derive(Debug)]
pub struct Create {
    inputs: Vec<DropletId>,
    outputs: Vec<DropletId>,
    location: Option<Location>,
    dimensions: Location,
    volume: f64,
}

// TODO: dimensions probably shouldn't be optional?
impl Create {
    pub fn new(
        loc: Option<Location>,
        vol: f64,
        dim: Option<Location>,
        out_id: DropletId,
    ) -> PuddleResult<Create> {
        Ok(Create {
            inputs: vec![],
            outputs: vec![out_id],
            location: loc,
            dimensions: dim.unwrap_or_else(|| yx(1, 1)),
            volume: vol,
        })
    }
}

impl Command for Create {
    fn input_droplets(&self) -> Vec<DropletId> {
        self.inputs.clone()
    }

    fn output_droplets(&self) -> Vec<DropletId> {
        self.outputs.clone()
    }

    fn request(&self, _gridview: &GridView) -> CommandRequest {
        let grid = Grid::rectangle(self.dimensions.y as usize, self.dimensions.x as usize);

        CommandRequest {
            name: format!("create -> {:?}", self.outputs[0]),
            shape: grid,
            input_locations: vec![],
            offset: self.location,
        }
    }

    fn run(&mut self, gridview: &mut GridSubView) -> RunStatus {
        gridview.insert(Droplet::new(
            self.outputs[0],
            self.volume,
            yx(0, 0),
            self.dimensions,
        ));
        RunStatus::Done
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

    fn request(&self, gridview: &GridView) -> CommandRequest {
        let old_id = self.inputs[0];
        let dim = gridview.droplets[&old_id].dimensions;
        CommandRequest {
            name: format!("move({:?}, {:?})", self.inputs[0], self.outputs[0]),
            shape: Grid::rectangle(dim.y as usize, dim.x as usize),
            input_locations: vec![yx(0, 0)],
            offset: Some(self.destination[0]),
        }
    }

    fn run(&mut self, gridview: &mut GridSubView) -> RunStatus {
        let old_id = self.inputs[0];
        let new_id = self.outputs[0];
        let mut d = gridview.remove(&old_id);
        // NOTE this is pretty much the only place it's ok to change an id
        d.id = new_id;
        gridview.insert(d);
        RunStatus::Done
    }
}

//
//  Combine
//

#[derive(Debug)]
pub struct Combine {
    inputs: Vec<DropletId>,
    outputs: Vec<DropletId>,
    pin_d0: bool,
}

impl Combine {
    pub fn new(id1: DropletId, id2: DropletId, out_id: DropletId) -> PuddleResult<Combine> {
        Ok(Combine {
            inputs: vec![id1, id2],
            outputs: vec![out_id],
            pin_d0: false,
        })
    }

    // combines the second into the first, pinning the first
    pub fn combine_into(
        id1: DropletId,
        id2: DropletId,
        out_id: DropletId,
    ) -> PuddleResult<Combine> {
        Ok(Combine {
            inputs: vec![id1, id2],
            outputs: vec![out_id],
            pin_d0: true,
        })
    }

    fn combined(&self, d0: &Droplet, d1: &Droplet) -> SimpleBlob {
        // FIXME this is a hack
        // right now we only support vertical stacking
        if self.pin_d0 {
            assert!(d0.location.y > d1.dimensions.y);
        }
        SimpleBlob {
            location: d0.location - yx(d1.dimensions.y, 0),
            dimensions: Location {
                y: (d0.dimensions.y + d1.dimensions.y),
                x: d0.dimensions.x.max(d1.dimensions.x),
            },
            volume: d0.volume + d1.volume,
        }
    }
}

impl Command for Combine {
    fn input_droplets(&self) -> Vec<DropletId> {
        self.inputs.clone()
    }

    fn output_droplets(&self) -> Vec<DropletId> {
        self.outputs.clone()
    }

    // FIXME remove bypass
    // fn bypass(&self, gridview: &GridView) -> bool {
    //     let droplets = &gridview.snapshot().droplets;
    //     if droplets.contains_key(&self.outputs[0]) {
    //         assert!(!droplets.contains_key(&self.inputs[0]));
    //         assert!(!droplets.contains_key(&self.inputs[1]));
    //         true
    //     } else {
    //         false
    //     }
    // }

    fn request(&self, gridview: &GridView) -> CommandRequest {
        let id0 = &self.inputs[0];
        let id1 = &self.inputs[1];

        // // FIXME what to do about collisions?
        // // set the collision groups to be the same
        // // must scope the mutable borrow
        // {
        //     let droplets = &mut gridview.droplets;
        //     let cg1 = droplets[id1].collision_group;
        //     let d0 = droplets.get_mut(id0).unwrap();
        //     d0.collision_group = cg1;

        //     if self.pin_d0 {
        //         d0.pinned = true;
        //     }
        // }

        let d0 = &gridview.droplets[id0];
        let d1 = &gridview.droplets[id1];

        let combined = self.combined(d0, d1);

        if self.pin_d0 {
            CommandRequest {
                name: format!("combine({:?}, {:?}) pin", d0.id, d1.id),
                shape: Grid::rectangle(
                    combined.dimensions.y as usize,
                    combined.dimensions.x as usize,
                ),
                input_locations: vec![d0.location, combined.location],
                offset: None,
            }
        } else {
            CommandRequest {
                name: format!("combine({:?}, {:?})", d0.id, d1.id),
                shape: Grid::rectangle(
                    // we need the plus 1 to ensure a gap
                    combined.dimensions.y as usize + 1,
                    combined.dimensions.x as usize,
                ),
                input_locations: vec![
                    // we need the plus 1 to ensure a gap
                    yx(d1.dimensions.y + 1, 0),
                    yx(0, 0),
                ],
                offset: None,
            }
        }
    }

    fn run(&mut self, gridview: &mut GridSubView) -> RunStatus {
        let in0 = self.inputs[0];
        let in1 = self.inputs[1];
        let out = self.outputs[0];

        let d0 = gridview.remove(&in0);
        let d1 = gridview.remove(&in1);
        // TODO right now this only mixes vertical
        // it should somehow communicate with the Combine command to control the mixed droplets dimensions
        let combined = self.combined(&d0, &d1);

        // assert_eq!(d0.location.y, d1.location.y);
        // assert_eq!(d0.location.x + d0.dimensions.x, d1.location.x);
        gridview.insert(combined.to_droplet(out));
        RunStatus::Done
    }
}

//
//  Agitate
//

#[derive(Debug)]
pub struct Agitate {
    inputs: Vec<DropletId>,
    outputs: Vec<DropletId>,
    n_agitation_loops: u32,
    current_step: usize,
    current_loop: usize,
}

impl Agitate {
    pub fn new(in_id: DropletId, out_id: DropletId) -> PuddleResult<Agitate> {
        Ok(Agitate {
            inputs: vec![in_id],
            outputs: vec![out_id],
            n_agitation_loops: 1,
            current_step: 0,
            current_loop: 0,
        })
    }
}

const AGITATE_PADDING: usize = 1;

impl Command for Agitate {
    fn input_droplets(&self) -> Vec<DropletId> {
        self.inputs.clone()
    }

    fn output_droplets(&self) -> Vec<DropletId> {
        self.outputs.clone()
    }

    fn request(&self, gridview: &GridView) -> CommandRequest {
        let droplet = &gridview.droplets[&self.inputs[0]];

        CommandRequest {
            name: format!("agitate({:?})", self.inputs[0]),
            shape: Grid::rectangle(
                droplet.dimensions.y as usize + AGITATE_PADDING,
                droplet.dimensions.x as usize + AGITATE_PADDING,
            ),
            input_locations: vec![yx(0, 0)],
            offset: None,
        }
    }

    fn run(&mut self, gridview: &mut GridSubView) -> RunStatus {
        let in_id = self.inputs[0];

        match self.current_step {
            0 => gridview.move_south(in_id),
            1 => gridview.move_east(in_id),
            2 => gridview.move_north(in_id),
            3 => gridview.move_west(in_id),
            n => panic!("invalid state {}", n),
        }

        self.current_step += 1;

        if self.current_step == 4 {
            self.current_step = 0;
            self.current_loop += 1;
            if self.current_loop < self.n_agitation_loops as usize {
                RunStatus::KeepGoing
            } else {
                let out_id = self.outputs[0];
                let mut droplet = gridview.remove(&in_id);
                droplet.id = out_id;
                gridview.insert(droplet);
                RunStatus::Done
            }
        } else {
            RunStatus::KeepGoing
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
    state: usize,
}

impl Split {
    pub fn new(id: DropletId, out_id1: DropletId, out_id2: DropletId) -> PuddleResult<Split> {
        Ok(Split {
            inputs: vec![id],
            outputs: vec![out_id1, out_id2],
            state: 0,
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

    // FIXME skip bypass
    // fn bypass(&self, gridview: &GridView) -> bool {
    //     let droplets = &gridview.snapshot().droplets;
    //     // if it has one, it better have both
    //     if droplets.contains_key(&self.outputs[0]) || droplets.contains_key(&self.outputs[1]) {
    //         assert!(droplets.contains_key(&self.outputs[0]));
    //         assert!(droplets.contains_key(&self.outputs[1]));
    //         assert!(!droplets.contains_key(&self.inputs[0]));
    //         true
    //     } else {
    //         false
    //     }
    // }

    fn request(&self, gridview: &GridView) -> CommandRequest {
        let d0 = &gridview.droplets[&self.inputs[0]];
        // we only split in the x right now, so we don't need y padding
        let x_dim = (d0.dimensions.x as usize) + SPLIT_PADDING;
        let y_dim = d0.dimensions.y as usize;
        let grid = Grid::rectangle(y_dim, x_dim);

        let input_locations = vec![yx(0, 2)];

        CommandRequest {
            name: format!("split({:?})", self.inputs[0]),
            shape: grid,
            input_locations: input_locations,
            offset: None,
        }
    }

    fn run(&mut self, gridview: &mut GridSubView) -> RunStatus {
        let inp = self.inputs[0];
        let out0 = self.outputs[0];
        let out1 = self.outputs[1];

        if self.state == 0 {
            self.state += 1;

            let x_dim = {
                // limit the scope of d0 borrow
                let d0 = gridview.get(&self.inputs[0]);
                (d0.dimensions.x as usize) + SPLIT_PADDING
            };

            let d = gridview.remove(&inp);
            let vol = d.volume / 2.0;

            // TODO: this should be related to volume in some fashion
            // currently, take the ceiling of the division of the split by two
            let dim = Location {
                y: d.dimensions.y,
                x: (d.dimensions.x + 1) / 2,
            };

            let loc0 = yx(0, 1);
            let loc1 = yx(0, x_dim as i32 - (dim.x + 1));

            gridview.insert(Droplet::new(out0, vol, loc0, dim));
            gridview.insert(Droplet::new(out1, vol, loc1, dim));

            RunStatus::KeepGoing
        } else {
            gridview.move_west(out0);
            gridview.move_east(out1);
            RunStatus::Done
        }
    }
}

#[derive(Debug)]
pub struct Heat {
    inputs: Vec<DropletId>,
    outputs: Vec<DropletId>,
    temperature: f32,
    duration: Duration,
    heater: Option<Peripheral>,
}

impl Heat {
    pub fn new(
        id: DropletId,
        out_id: DropletId,
        temperature: f32,
        duration: Duration,
    ) -> PuddleResult<Heat> {
        Ok(Heat {
            inputs: vec![id],
            outputs: vec![out_id],
            temperature,
            duration,
            heater: None,
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

    fn request(&self, gridview: &GridView) -> CommandRequest {
        let d = &gridview.droplets[&self.inputs[0]];
        // we only split in the x right now, so we don't need y padding
        let x_dim = d.dimensions.x as usize;
        let y_dim = d.dimensions.y as usize;

        // right now we can only heat droplets that are 1x1
        // assert_eq!(y_dim, 1);
        assert_eq!(x_dim, 1);
        let mut grid = Grid::rectangle(y_dim, x_dim);

        // the parameters of heater here don't matter, as it's just used to
        // match up with the "real" heater in the actual grid
        let loc = yx(y_dim as i32 - 1, 0);
        grid.get_cell_mut(loc).unwrap().peripheral = Some(Peripheral::Heater {
            pwm_channel: 0,
            spi_channel: 0,
        });

        let input_locations = vec![loc];

        CommandRequest {
            name: format!("heat({:?})", d.id),
            shape: grid,
            input_locations: input_locations,
            offset: None,
        }
    }

    fn run(&mut self, gridview: &mut GridSubView) -> RunStatus {
        let old_id = self.inputs[0];
        let new_id = self.outputs[0];

        let mut d = gridview.remove(&old_id);
        // NOTE this is a rare place it's ok to change an id, like move
        d.id = new_id;
        gridview.insert(d);
        RunStatus::Done
    }
}

#[derive(Debug)]
pub struct Input {
    substance: String,
    volume: f64,
    dimensions: Location,
    outputs: Vec<DropletId>,
    input: Option<Peripheral>,
}

impl Input {
    pub fn new(
        substance: String,
        volume: f64,
        dimensions: Location,
        out_id: DropletId,
    ) -> PuddleResult<Input> {
        Ok(Input {
            substance,
            volume,
            dimensions,
            outputs: vec![out_id],
            input: None,
        })
    }
}

impl Command for Input {
    fn input_droplets(&self) -> Vec<DropletId> {
        vec![]
    }

    fn output_droplets(&self) -> Vec<DropletId> {
        self.outputs.clone()
    }

    fn request(&self, _gridview: &GridView) -> CommandRequest {
        let mut grid = Grid::rectangle(self.dimensions.y as usize, self.dimensions.x as usize + 1);

        // fake peripheral used to match up with the real one
        // FIXME: this is a total hack to assume that input is always on the right-hand side
        let loc = Location {
            y: self.dimensions.y / 2,
            x: self.dimensions.x - 1 + 1,
        };
        grid.get_cell_mut(loc).unwrap().peripheral = Some(Peripheral::Input {
            pwm_channel: 0,
            name: self.substance.clone(),
        });

        debug!("Input location will be at {}", loc);

        CommandRequest {
            name: format!("input -> {:?}", self.outputs[0]),
            shape: grid,
            input_locations: vec![],
            offset: None,
        }
    }

    fn run(&mut self, _gridview: &mut GridSubView) -> RunStatus {
        RunStatus::Done
    }
}

#[derive(Debug)]
pub struct Output {
    name: String,
    inputs: Vec<DropletId>,
    volume: Option<f64>,
    output: Option<Peripheral>,
}

impl Output {
    pub fn new(name: String, id: DropletId) -> PuddleResult<Output> {
        Ok(Output {
            name,
            inputs: vec![id],
            volume: None,
            output: None,
        })
    }
}

impl Command for Output {
    fn input_droplets(&self) -> Vec<DropletId> {
        self.inputs.clone()
    }

    fn output_droplets(&self) -> Vec<DropletId> {
        vec![]
    }

    fn request(&self, gridview: &GridView) -> CommandRequest {
        let d = &gridview.droplets[&self.inputs[0]];

        let mut grid = Grid::rectangle(d.dimensions.y as usize, d.dimensions.x as usize);

        // fake peripheral used to match up with the real one
        // FIXME: this is a total hack to assume that output is always on the left-hand side
        let loc = yx(d.dimensions.y / 2, 0);
        grid.get_cell_mut(loc).unwrap().peripheral = Some(Peripheral::Output {
            pwm_channel: 0,
            name: self.name.clone(),
        });

        debug!("Output location will be at {}", loc);

        CommandRequest {
            name: format!("output({:?})", d.id),
            shape: grid,
            input_locations: vec![loc],
            offset: None,
        }
    }

    fn run(&mut self, _gridview: &mut GridSubView) -> RunStatus {
        RunStatus::Done
    }
}

#[cfg(test)]
pub mod tests {

    use super::*;

    #[derive(Debug)]
    pub struct Dummy {
        ins: Vec<DropletId>,
        outs: Vec<DropletId>,
    }

    impl Dummy {
        pub fn new(ins: &[usize], outs: &[usize]) -> Dummy {
            Dummy {
                ins: ins.iter().map(|u| (*u).into()).collect(),
                outs: outs.iter().map(|u| (*u).into()).collect(),
            }
        }
        pub fn boxed(self) -> BoxedCommand {
            Box::new(self)
        }
    }

    impl Command for Dummy {
        fn input_droplets(&self) -> Vec<DropletId> {
            self.ins.clone()
        }

        fn output_droplets(&self) -> Vec<DropletId> {
            self.outs.clone()
        }

        fn request(&self, _gridview: &GridView) -> CommandRequest {
            unimplemented!()
        }

        fn run(&mut self, _gridview: &mut GridSubView) -> RunStatus {
            unimplemented!()
        }
    }
}
