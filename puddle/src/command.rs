
use std::collections::HashMap;
use std::collections::hash_map::Entry::*;

use arch::{Architecture, Location, DropletId};
use arch::grid::Grid;

use api::PuddleError;
use api::PuddleError::*;

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
        vec![(0,0), (0,1), (1,1), (2,1), (2,0), (1,0), (0,0)]
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
    pub fn new(arch: &mut Architecture, id1: DropletId, id2: DropletId) -> Result<Mix, PuddleError> {
        let output = arch.new_droplet_id();

        // we must validate getting these things out of the hashtable
        // TODO wrap this in a method of arch
        let d1_cg = arch.droplets.get(&id1)
            .ok_or(NonExistentDropletId(id1))?.collision_group;
        let d2 = arch.droplets.get_mut(&id2)
            .ok_or(NonExistentDropletId(id1))?;

        // make sure their collision groups are the same so we can mix them
        d2.collision_group = d1_cg;

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
