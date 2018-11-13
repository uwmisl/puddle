use std::collections::HashSet;

use grid::{Grid, Location, DropletId};
use command::{CommandRequest};
use util::collections::Map;


#[derive(Debug, Clone)]
pub struct Placement {
    // TODO idk if this should be pub
    pub mapping: Map<Location, Location>,
}

pub struct PlacementResponse {
    pub commands: Vec<Placement>,
    // droplets only need to be "placed" by their upper left corner
    pub stored_droplets: Vec<Location>,
}

pub struct PlacementRequest<'a> {
    pub grid: &'a Grid,
    pub fixed_commands: Vec<Placement>,
    pub commands: &'a [CommandRequest],
    pub stored_droplets: &'a [DropletId],
}

#[derive(Debug)]
pub enum PlacementError {}

type PlacementResult = Result<Placement, PlacementError>;

pub struct Placer {}

impl Placer {
    pub fn new() -> Placer {
        Placer {}
    }

    pub fn place(&self, req: &PlacementRequest) -> PlacementResult {
        unimplemented!()
        // let snapshot = self.snapshot_at(start_tick);

        // let bad_locs: HashSet<Location> = snapshot
        //     .cmd_shapes
        //     .iter()
        //     // should refer to placement, but that's not what command gives you right now
        //     .flat_map(|placement| placement.mapping.values())
        //     .cloned()
        //     .collect();

        // let offset = self
        //     .grid
        //     .locations()
        //     .map(|(loc, _cell)| loc)
        //     .find(|loc| is_compatible(&self.grid, shape, *loc, &bad_locs));

        // if let Some(offset) = offset {
        //     let mapping = self
        //         .grid
        //         .locations()
        //         .map(|(loc, _)| (loc, &loc + &offset))
        //         .collect();
        //     Ok(Placement { mapping })
        // } else {
        //     Err(PlacementError::Bad)
        // }
    }
}

fn is_compatible(
    bigger: &Grid,
    smaller: &Grid,
    offset: Location,
    bad_locs: &HashSet<Location>,
) -> bool {
    smaller.locations().all(|(small_loc, small_cell)| {
        let big_loc = &small_loc + &offset;
        if bad_locs.contains(&big_loc) {
            return false;
        }

        // return the compatibility
        bigger
            .get_cell(&big_loc)
            .map_or(false, |big_cell| small_cell.is_compatible(&big_cell))
    })
}

#[cfg(test)]
mod tests {

    use super::*;
    use grid::Peripheral;

    #[test]
    fn grid_self_compatible() {
        let grid = Grid::rectangle(5, 4);
        let shape = Grid::rectangle(5, 4);
        let offset = Location { y: 0, x: 0 };
        let bad_locs = HashSet::new();

        assert!(is_compatible(&grid, &shape, offset, &bad_locs))
    }

    // #[test]
    // fn grid_self_place() {
    //     let grid = Grid::rectangle(5, 4);
    //     let shape = Grid::rectangle(5, 4);
    //     let plan = Plan::new(grid.clone());

    //     let start_tick = 0;
    //     let end_tick = Some(5);
    //     let placement = plan.place(&shape, start_tick, end_tick).unwrap();

    //     let identity_mapping: Map<_, _> = grid.locations().map(|(loc, _)| (loc, loc)).collect();
    //     assert_eq!(identity_mapping, placement.mapping)
    // }

    // #[test]
    // fn test_place_heater() {
    //     let mut grid = Grid::rectangle(3, 3);
    //     let heater_loc = Location { y: 2, x: 1 };
    //     grid.get_cell_mut(&heater_loc).unwrap().peripheral = Some(Peripheral::Heater {
    //         // these don't matter, they shouldn't be used for compatibility
    //         pwm_channel: 10,
    //         spi_channel: 42,
    //     });

    //     let mut shape = Grid::rectangle(1, 1);
    //     shape
    //         .get_cell_mut(&Location { y: 0, x: 0 })
    //         .unwrap()
    //         .peripheral = Some(Peripheral::Heater {
    //         pwm_channel: 0,
    //         spi_channel: 0,
    //     });

    //     let plan = Plan::new(grid.clone());
    //     let start_tick = 0;
    //     let end_tick = Some(5);

    //     let placement = plan.place(&shape, start_tick, end_tick).unwrap();

    //     assert_eq!(
    //         placement.mapping.get(&Location { y: 0, x: 0 }),
    //         Some(&heater_loc)
    //     );
    // }

}
