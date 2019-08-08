use crate::command::CommandRequest;
use crate::grid::{DropletId, Grid, GridView, Location, Rectangle};
use crate::util::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Placement {
    // TODO idk if this should be pub
    pub mapping: HashMap<Location, Location>,
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

struct Context<'a> {
    req: PlacementRequest<'a>,
    bad_locs: HashSet<Location>,
    resp: PlacementResponse,
}

impl<'a> Context<'a> {
    fn new(req: PlacementRequest<'a>) -> Self {
        Context {
            req,
            bad_locs: HashSet::default(),
            resp: PlacementResponse {
                commands: Vec::new(),
                stored_droplets: Vec::new(),
            },
        }
    }

    fn place_cmd(&self, cmd_req: &CommandRequest) -> Result<Placement, PlacementError> {
        debug!("Placing {:?}", cmd_req);
        if let Some(offset) = cmd_req.offset {
            let mapping: HashMap<_, _> = cmd_req
                .shape
                .locations()
                .map(|(loc, _cell)| (loc, loc + offset))
                .collect();

            // check to make sure this forced placement is valid
            for loc in mapping.values() {
                let nbrs = self.req.gridview.grid.neighbors9(*loc);
                if nbrs.iter().any(|n| self.bad_locs.contains(n)) {
                    return Err(PlacementError::Bad)
                }
            }

            let placement = Placement { mapping };
            debug!("Placed at {:?}", placement);
            return Ok(placement);
        }

        let mut potential_offsets: Vec<Location> = self
            .req
            .gridview
            .grid
            .locations()
            .map(|(loc, _cell)| loc)
            .collect();

        potential_offsets.sort();

        // simply find an offset by testing all of them.
        let offset = potential_offsets
            .iter()
            .find(|&&loc| self.is_compatible(&cmd_req.shape, loc))
            .ok_or(PlacementError::Bad)?;

        let mapping = cmd_req
            .shape
            .locations()
            .map(|(loc, _)| (loc, loc + *offset))
            .collect();

        let placement = Placement { mapping };

        // save this for returning
        debug!("Placed at {:?}", placement);
        Ok(placement)
    }

    fn place_droplet(&self, id: DropletId) -> Result<Location, PlacementError> {
        debug!("Placing droplet {:?}", id);
        // simply find an offset by testing all of them.

        let droplet = &self.req.gridview.droplets[&id];

        let mut locations_by_distance: Vec<(u32, Location)> = self
            .req
            .gridview
            .grid
            .locations()
            .map(|(loc, _cell)| (loc.distance_to(droplet.location), loc))
            .collect();
        locations_by_distance.sort();

        let Location { y, x } = droplet.dimensions;
        let shape = Grid::rectangle(y as usize, x as usize);

        let offset = locations_by_distance
            .iter()
            .map(|&(_distance, loc)| loc)
            .find(|loc| self.is_compatible(&shape, *loc))
            .ok_or(PlacementError::Bad)?;

        debug!("Placed at {:?}", offset);
        Ok(offset)
    }

    fn is_compatible(&self, smaller: &Grid, offset: Location) -> bool {
        is_compatible(&self.req.gridview.grid, smaller, offset, &self.bad_locs)
    }

    fn place(mut self) -> PlacementResult {
        assert_eq!(self.req.fixed_commands.len(), 0);

        for cmd_req in self.req.commands {
            let placement = self.place_cmd(cmd_req)?;
            self.bad_locs.extend(placement.mapping.values().cloned());
            self.resp.commands.push(placement);
        }

        trace!("Bad locs: {:?}", self.bad_locs);

        // iteratively place the droplets
        for id in self.req.stored_droplets {
            let offset = self.place_droplet(*id)?;
            self.bad_locs.extend(
                Rectangle {
                    location: offset,
                    dimensions: self.req.gridview.droplets[&id].dimensions,
                }
                .locations(),
            );
            self.resp.stored_droplets.push(offset)
        }

        Ok(self.resp)
    }
}

#[derive(Default)]
pub struct Placer {}

impl Placer {
    pub fn place(&self, req: PlacementRequest) -> PlacementResult {
        let ctx = Context::new(req);
        ctx.place()
    }
}

fn is_compatible(
    bigger: &Grid,
    smaller: &Grid,
    offset: Location,
    bad_locs: &HashSet<Location>,
) -> bool {
    smaller.locations().all(|(small_loc, small_cell)| {
        let big_loc = small_loc + offset;
        let nbrs = bigger.neighbors9(big_loc);
        if nbrs.iter().any(|n| bad_locs.contains(n)) {
            return false;
        }

        // return the compatibility
        bigger
            .get_cell(big_loc)
            .map_or(false, |big_cell| small_cell.is_compatible(&big_cell))
    })
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn grid_self_compatible() {
        let grid = Grid::rectangle(5, 4);
        let shape = Grid::rectangle(5, 4);
        let offset = Location { y: 0, x: 0 };
        let bad_locs = HashSet::default();

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

    //     let identity_mapping: HashMap<_, _> = grid.locations().map(|(loc, _)| (loc, loc)).collect();
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
