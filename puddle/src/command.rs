
use std::collections::HashMap;
use std::collections::hash_map::Entry::*;

use arch::{Architecture, Location, Droplet, DropletId};
use arch::grid::Grid;

type Mapping = HashMap<Location, Location>;

pub trait Command {
    fn input_droplets(&self) -> &[DropletId];
    fn input_locations(&self) -> &[Location];
    fn output_droplets(&self) -> &[DropletId];
    fn output_locations(&self) -> &[Location];
    fn shape(&self) -> &Grid;
    fn run(&self, arch: &mut Architecture, mapping: &Mapping);
}

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
        vec![(0,0), (1,0), (1,1), (1,2), (0,2), (0,1), (0,0)]
        .iter()
        .map(|&(y,x)| Location {y, x})
        .collect();
}


pub struct Mix {
    // TODO use `FixedSizeArray` when it lands on stable
    inputs: Vec<DropletId>,
    outputs: Vec<DropletId>,
}

impl Mix {
    pub fn new(arch: &mut Architecture, d1: DropletId, d2: DropletId) -> Mix {
        let output = arch.new_droplet_id();

        Mix {
            inputs: vec![d1, d2],
            outputs: vec![output]
        }
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
