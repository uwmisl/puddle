mod minheap;
mod place;
mod route;

pub use self::route::Path;

use command::{Command, DynamicCommandInfo};
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

impl GridView {
    pub fn plan(&mut self, cmd: Box<dyn Command>) -> Result<(), PlanError> {
        info!("Planning {:?}", cmd);

        // make sure there's a snapshot available to plan into
        self.snapshot_ensure();
        if cmd.bypass(&self) {
            info!("Bypassing command: {:#?}", cmd);
            return Ok(());
        }

        let in_ids = cmd.input_droplets();
        let DynamicCommandInfo {
            shape,
            input_locations,
            trusted,
        } = cmd.dynamic_info(self);

        debug!(
            "Command requests a shape of w={w},h={h}",
            w = shape.max_width(),
            h = shape.max_height(),
        );

        debug!(
            "Input droplets: {:?}",
            cmd.input_droplets()
                .iter()
                .map(|id| &self.snapshot().droplets[id])
                .collect::<Vec<_>>()
        );

        let placement = if trusted {
            // if we are trusting placement, just use an identity map
            self.grid
                .locations()
                .map(|(loc, _cell)| (loc, loc))
                .collect::<Map<_, _>>()
        } else {
            // TODO place should be a method of gridview
            self.grid
                .place(&shape, self.snapshot(), &self.bad_edges)
                .ok_or(PlanError::PlaceError)?
        };

        debug!("placement for {:#?}: {:#?}", cmd, placement);

        assert_eq!(input_locations.len(), in_ids.len());

        for (loc, id) in input_locations.iter().zip(&in_ids) {
            // this should have been put to none last time
            let droplet = self.snapshot_mut()
                .droplets
                .get_mut(&id)
                .expect("Command gave back and invalid DropletId");
            assert!(droplet.destination.is_none());
            let mapped_loc = placement
                .get(loc)
                .expect("input location wasn't in placement");
            droplet.destination = Some(*mapped_loc);
        }

        debug!("routing {:?}", cmd);
        let paths = match self.route() {
            Some(p) => p,
            None => {
                return Err(PlanError::RouteError {
                    placement: placement,
                    droplets: self.snapshot().droplets.values().cloned().collect(),
                })
            }
        };
        debug!("route for {:#?}: {:#?}", cmd, paths);

        trace!("Taking paths...");
        // FIXME final tick is a hack
        // we *carefully* pre-run the command before making the final tick
        let final_tick = false;
        self.take_paths(&paths, final_tick);

        {
            let mut subview = self.subview(in_ids.iter().cloned(), placement.clone());

            trace!("Pre-Running command {:?}", cmd);
            cmd.pre_run(&mut subview);
            subview.tick();

            trace!("Running command {:?}", cmd);
            cmd.run(&mut subview);
        }

        self.register(cmd);

        // teardown destinations if the droplets are still there
        // TODO is this ever going to be true?
        for id in in_ids {
            if let Some(droplet) = self.snapshot_mut().droplets.get_mut(&id) {
                assert_eq!(Some(droplet.location), droplet.destination);
                droplet.destination = None;
            };
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use env_logger;
    use std::fs::File;
    use tests::project_path;

    use super::*;

    use command;
    use grid::{DropletId, Grid};

    fn mk_gv(path: &str) -> GridView {
        let _ = env_logger::try_init();
        let f = File::open(project_path(path)).unwrap();
        GridView::new(Grid::from_reader(f).unwrap())
    }

    #[test]
    fn plan_input() {
        let mut gv = mk_gv("tests/arches/purpledrop.json");
        let cmd = {
            let substance = "something".into();
            let volume = 1.0;
            let dimensions = Location { y: 3, x: 3 };
            let out_id = DropletId {
                id: 0,
                process_id: 0,
            };
            command::Input::new(substance, volume, dimensions, out_id).unwrap()
        };
        gv.plan(Box::new(cmd)).unwrap();
    }

    #[test]
    fn plan_output() {
        let mut gv = mk_gv("tests/arches/purpledrop.json");

        let id = DropletId {
            id: 0,
            process_id: 0,
        };
        let droplet = {
            let volume = 1.0;
            let location = Location { y: 7, x: 0 };
            let dimensions = Location { y: 4, x: 3 };
            Droplet::new(id, volume, location, dimensions)
        };
        gv.snapshot_mut().droplets.insert(id, droplet);

        let cmd = {
            let substance = "something else".into();
            command::Output::new(substance, id).unwrap()
        };

        gv.plan(Box::new(cmd)).unwrap();
    }

}
