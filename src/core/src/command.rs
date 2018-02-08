
use std::collections::HashMap;
use std::collections::hash_map::Entry::*;

use arch::{Architecture, Location, DropletId};
use arch::grid::Grid;

use api::PuddleResult;

type Mapping = HashMap<Location, Location>;

// Send and 'static here are necessary to move trait objects around
pub trait Command: Send + 'static {
    fn input_droplets(&self) -> &[DropletId];
    fn input_locations(&self) -> &[Location];
    fn output_droplets(&self) -> &[DropletId];
    fn output_locations(&self) -> &[Location];
    fn shape(&self) -> &Grid;
    fn run(&self, arch: &mut Architecture, mapping: &Mapping);

    fn trust_placement(&self) -> bool {
        false
    }

    fn pre_run(&self, _: &mut Architecture) {}
}

//
//  Input
//

lazy_static! {
    static ref INPUT_SHAPE: Grid = Grid::rectangle(0,0);
    static ref INPUT_INPUT_LOCS: Vec<Location> = vec![];
}

pub struct Input {
    inputs: Vec<DropletId>,
    outputs: Vec<DropletId>,
    destination: [Location; 1],
}

impl Input {
    pub fn new(arch: &mut Architecture, loc: Location) -> PuddleResult<Input> {
        let output = arch.new_droplet_id();

        Ok( Input {
            inputs: vec![],
            outputs: vec![output],
            destination: [loc]
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
        &self.destination
    }

    fn shape(&self) -> &Grid {
        &INPUT_SHAPE
    }

    fn trust_placement(&self) -> bool {
        true
    }

    fn run(&self, arch: &mut Architecture, _: &Mapping) {
        let droplet = arch.droplet_from_location(self.destination[0]);
        let result_id = self.outputs[0];

        let result = match arch.droplets.entry(result_id) {
            Occupied(occ) => panic!("Droplet was already here: {:?}", occ.get()),
            Vacant(spot) => spot.insert(droplet)
        };

        assert!(result.location == self.destination[0]);
    }
}

//
//  Move
//

lazy_static! {
    static ref MOVE_SHAPE: Grid = Grid::rectangle(0,0);
}

pub struct Move {
    inputs: Vec<DropletId>,
    outputs: Vec<DropletId>,
    destination: [Location; 1],
}

impl Move {
    pub fn new(arch: &mut Architecture, id: DropletId, loc: Location) -> PuddleResult<Move> {
        let output = arch.new_droplet_id();

        Ok( Move {
            inputs: vec![id],
            outputs: vec![output],
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

    fn run(&self, arch: &mut Architecture, _: &Mapping) {

        let mut droplet = arch.droplets.remove(&self.inputs[0]).unwrap();
        droplet.destination = None;

        let result_id = self.outputs[0];

        match arch.droplets.entry(result_id) {
            Occupied(occ) => panic!("Droplet was already here: {:?}", occ.get()),
            Vacant(spot) => spot.insert(droplet)
        };
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
            Location {y: 0, x: 0},
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

pub struct Mix {
    inputs: Vec<DropletId>,
    outputs: Vec<DropletId>,
}

impl Mix {
    pub fn new(arch: &mut Architecture, id1: DropletId, id2: DropletId) -> PuddleResult<Mix> {
        let output = arch.new_droplet_id();

        Ok( Mix {
            inputs: vec![id1, id2],
            outputs: vec![output]
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

    fn pre_run(&self, arch: &mut Architecture) {

        let d0_cg = arch.droplets.get(&self.inputs[0]).unwrap().collision_group;
        let d1 = arch.droplets.get_mut(&self.inputs[1]).unwrap();

        d1.collision_group = d0_cg;

        // possibly move creating output droplets here
    }

    fn run(&self, arch: &mut Architecture, mapping: &Mapping) {

        let d0 = arch.droplets.remove(&self.inputs[0]).unwrap();
        let d1 = arch.droplets.remove(&self.inputs[1]).unwrap();

        let droplet = arch.droplet_from_location(d0.location);
        let result_id = self.outputs[0];

        let result = match arch.droplets.entry(result_id) {
            Occupied(occ) => panic!("Droplet was already here: {:?}", occ.get()),
            Vacant(spot) => spot.insert(droplet)
        };

        assert!(d0.location == d1.location);

        for loc in MIX_LOOP.iter() {
            result.location = mapping[loc];
        }
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
    static ref SPLIT_PATH1: Vec<Location> =
        vec![
            Location {y: 0, x: 1},
            Location {y: 0, x: 0},
        ];
    static ref SPLIT_PATH2: Vec<Location> =
        vec![
            Location {y: 0, x: 3},
            Location {y: 0, x: 4},
        ];
}

pub struct Split {
    inputs: Vec<DropletId>,
    outputs: Vec<DropletId>,
}

impl Split {
    pub fn new(arch: &mut Architecture, id: DropletId) -> PuddleResult<Split> {
        let output1 = arch.new_droplet_id();
        let output2 = arch.new_droplet_id();

        Ok( Split {
            inputs: vec![id],
            outputs: vec![output1, output2]
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

    fn run(&self, arch: &mut Architecture, mapping: &Mapping) {

        let d = arch.droplets.remove(&self.inputs[0]).unwrap();

        let mut droplet1 = arch.droplet_from_location(d.location);
        let mut droplet2 = arch.droplet_from_location(d.location);

        let result1_id = self.outputs[0];
        let result2_id = self.outputs[1];

        let droplet2_cg = droplet2.collision_group;
        droplet2.collision_group = droplet1.collision_group;

        match arch.droplets.entry(result1_id) {
            Occupied(occ) => panic!("Droplet was already here: {:?}", occ.get()),
            Vacant(spot) => spot.insert(droplet1)
        };

        match arch.droplets.entry(result2_id) {
            Occupied(occ) => panic!("Droplet was already here: {:?}", occ.get()),
            Vacant(spot) => spot.insert(droplet2)
        };

        for (loc1, loc2) in SPLIT_PATH1.iter().zip(SPLIT_PATH2.iter()) {
            droplet1.location = mapping[loc1];
            droplet2.location = mapping[loc2];
        }

        droplet2.collision_group = droplet2_cg;
    }
}
