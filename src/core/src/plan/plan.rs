use std::sync::{Arc, Mutex};

use command::Command;
use grid::{Droplet, GridView, Location};
use util::collections::Map;

#[derive(Debug)]
pub enum PlanError {
    RouteError {
        placement: Placement,
        droplets: Vec<Droplet>,
    },
    PlaceError,
}

pub type Placement = Map<Location, Location>;

pub struct Planner {
    gridview: Arc<Mutex<GridView>>,
}

impl Planner {
    pub fn new(gridview: Arc<Mutex<GridView>>) -> Planner {
        Planner { gridview: gridview }
    }

    pub fn plan(&mut self, cmd: Box<Command>) -> Result<(), PlanError> {
        info!("Planning {:?}", cmd);
        debug!("placing (trusted = {}) {:?}", cmd.trust_placement(), cmd);

        let mut gv = self.gridview.lock().unwrap();

        let in_ids = cmd.input_droplets();
        let (shape, in_locs) = {
            let command_info = cmd.dynamic_info(&gv);
            (command_info.shape, command_info.input_locations)
        };

        debug!(
            "Command requests a shape of w={w},h={h}",
            w = shape.max_width(),
            h = shape.max_height(),
        );

        debug!(
            "Input droplets: {:?}",
            cmd.input_droplets()
                .iter()
                .map(|id| &gv.droplets()[id])
                .collect::<Vec<_>>()
        );

        let placement = if cmd.trust_placement() {
            // if we are trusting placement, just use an identity map
            gv.grid
                .locations()
                .map(|(loc, _cell)| (loc, loc))
                .collect::<Map<_, _>>()
        } else {
            // TODO place should be a method of gridview
            gv.grid
                .place(&shape, gv.droplets())
                .ok_or(PlanError::PlaceError)?
        };

        debug!("placement for {:?}: {:?}", cmd, placement);

        assert_eq!(in_locs.len(), in_ids.len());

        for (loc, id) in in_locs.iter().zip(&in_ids) {
            // this should have been put to none last time
            let droplet = gv.droplets_mut()
                .get_mut(&id)
                .expect("Command gave back and invalid DropletId");
            assert!(droplet.destination.is_none());
            let mapped_loc = placement
                .get(loc)
                .expect("input location wasn't in placement");
            droplet.destination = Some(*mapped_loc);
        }

        debug!("routing {:?}", cmd);
        let paths = match gv.route() {
            Some(p) => p,
            None => {
                return Err(PlanError::RouteError {
                    placement: placement,
                    droplets: gv.droplets().values().map(|d| d.clone()).collect(),
                })
            }
        };
        debug!("route for {:?}: {:?}", cmd, paths);

        trace!("Taking paths...");
        gv.take_paths(&paths);

        trace!("Running command {:?}", cmd);
        cmd.run(&mut gv.subview(in_ids.iter().cloned(), placement));

        // teardown destinations if the droplets are still there
        // TODO is this ever going to be true?
        for id in in_ids {
            gv.droplets_mut().get_mut(&id).map(|droplet| {
                assert_eq!(Some(droplet.location), droplet.destination);
                droplet.destination = None;
            });
        }

        Ok(())
    }
}
