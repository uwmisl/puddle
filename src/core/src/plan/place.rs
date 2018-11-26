use std::collections::HashSet;

use command::CommandRequest;
use grid::{DropletId, Grid, GridView, Location};
use util::collections::Map;

#[derive(Debug, Clone)]
pub struct Placement {
    // TODO idk if this should be pub
    pub mapping: Map<Location, Location>,
}

#[derive(Debug)]
pub struct PlacementResponse {
    pub commands: Vec<Placement>,
    // droplets only need to be "placed" by their upper left corner
    pub stored_droplets: Vec<Location>,
}

pub struct PlacementRequest<'a> {
    pub gridview: &'a GridView,
    pub fixed_commands: Vec<Placement>,
    pub commands: &'a [CommandRequest],
    pub stored_droplets: &'a [DropletId],
}

#[derive(Debug)]
pub enum PlacementError {
    Bad,
}

type PlacementResult = Result<PlacementResponse, PlacementError>;

pub struct Placer {}

impl Placer {
    pub fn new() -> Placer {
        Placer {}
    }

    pub fn place(&self, req: &PlacementRequest) -> PlacementResult {
        let mut bad_locs = HashSet::new();

        // initialize bad locs with the fixed commands
        for placement in &req.fixed_commands {
            bad_locs.extend(placement.mapping.values().cloned())
        }

        for cmd_req in req.commands {
            // TODO assert that these are disjoint!
            if cmd_req.trusted {
                bad_locs.extend(cmd_req.shape.locations().map(|(loc, _cell)| loc))
            }
        }

        // TODO we only support one placement at a time for now
        assert_eq!(req.commands.len(), 1);

        // build an empty response for now
        let mut response = PlacementResponse {
            commands: Vec::new(),
            stored_droplets: Vec::new(),
        };

        // build up a set of location that currently hold droplets, and would
        // therefore require moving them if a command was placed on top of them.
        // Use this to avoid unnecessary moves.
        let mut locations_initially_holding_droplets = HashSet::new();
        for id in req.stored_droplets {
            let droplet = &req.gridview.droplets[id];
            for y in -1..(droplet.dimensions.y + 1) {
                for x in -1..(droplet.dimensions.x + 1) {
                    locations_initially_holding_droplets
                        .insert(&droplet.location + &Location { x, y });
                }
            }
        }

        // iteratively place the commands
        for cmd_req in req.commands {
            debug!("Placing {:?}", cmd_req);
            if cmd_req.trusted {
                let identity_mapping: Map<_, _> = req
                    .gridview
                    .grid
                    .locations()
                    .map(|(loc, _cell)| (loc, loc))
                    .collect();
                response.commands.push(Placement {
                    mapping: identity_mapping,
                });
                continue;
            }

            let mut potential_offsets: Vec<_> = req
                .gridview
                .grid
                .locations()
                .map(|(loc, _cell)| {
                    let would_require_move = locations_initially_holding_droplets.contains(&loc);
                    let i = if would_require_move { 1 } else { 0 };
                    (i, loc)
                }).collect();

            potential_offsets.sort();

            // simply find an offset by testing all of them.
            let offset = potential_offsets
                .iter()
                .map(|(_, loc)| *loc)
                .find(|loc| is_compatible(&req.gridview.grid, &cmd_req.shape, *loc, &bad_locs))
                .ok_or(PlacementError::Bad)?;

            let mapping = cmd_req
                .shape
                .locations()
                .map(|(loc, _)| (loc, &loc + &offset))
                .collect();

            let placement = Placement { mapping };

            // mark these spots as taken
            bad_locs.extend(placement.mapping.values().cloned());

            // save this for returning
            response.commands.push(placement)
        }

        // iteratively place the droplets
        for id in req.stored_droplets {
            debug!("Placing {:?}", id);
            // simply find an offset by testing all of them.

            let droplet = &req.gridview.droplets[id];

            let mut locations_by_distance: Vec<(u32, Location)> = req
                .gridview
                .grid
                .locations()
                .map(|(loc, _cell)| (loc.distance_to(&droplet.location), loc))
                .collect();
            locations_by_distance.sort();

            let Location { y, x } = droplet.dimensions;
            let shape = Grid::rectangle(y as usize, x as usize);

            let offset = locations_by_distance
                .iter()
                .map(|&(_distance, loc)| loc)
                .find(|loc| is_compatible(&req.gridview.grid, &shape, *loc, &bad_locs))
                .ok_or(PlacementError::Bad)?;

            // mark these spots as taken
            bad_locs.extend(shape.locations().map(|(loc, _cell)| &offset + &loc));

            response.stored_droplets.push(offset)
        }

        Ok(response)
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
        let nbrs = bigger.neighbors9(&big_loc);
        if nbrs.iter().any(|n| bad_locs.contains(n)) {
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
