use command::Command;
use exec::ExecItem;
use grid::{Droplet, Location, PreGridSubView, RootGridView};
use std::sync::mpsc::Sender;
use util::collections::{Map, Set};

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
    gridview: RootGridView,
    exec_tx: Sender<ExecItem>,
}

impl Planner {
    pub fn new(gridview: RootGridView, exec_tx: Sender<ExecItem>) -> Planner {
        Planner {
            gridview: gridview,
            exec_tx: exec_tx,
        }
    }

    pub fn plan(&mut self, cmd: Box<Command>) -> Result<(), PlanError> {
        info!("Planning {:?}", cmd);
        debug!("placing (trusted = {}) {:?}", cmd.trust_placement(), cmd);

        let in_ids = cmd.input_droplets();
        let (shape, in_locs) = {
            let command_info = cmd.dynamic_info(&self.gridview);
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
                .map(|id| &self.gridview.droplets[id])
                .collect::<Vec<_>>()
        );

        let placement = if cmd.trust_placement() {
            // if we are trusting placement, just use an identity map
            self.gridview
                .grid
                .locations()
                .map(|(loc, _cell)| (loc, loc))
                .collect::<Map<_, _>>()
        } else {
            // TODO place should be a method of gridview
            self.gridview
                .grid
                .place(&shape, &self.gridview.droplets)
                .ok_or(PlanError::PlaceError)?
        };

        debug!("placement for {:?}: {:?}", cmd, placement);

        assert_eq!(in_locs.len(), in_ids.len());

        for (loc, id) in in_locs.iter().zip(&in_ids) {
            // this should have been put to none last time
            let droplet = self.gridview
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
        let paths = match self.gridview.route() {
            Some(p) => p,
            None => {
                return Err(PlanError::RouteError {
                    placement: placement,
                    droplets: self.gridview.droplets.values().map(|d| d.clone()).collect(),
                })
            }
        };
        debug!("route for {:?}: {:?}", cmd, paths);

        let pre_subview = PreGridSubView {
            mapping: placement,
            ids: in_ids.iter().cloned().collect::<Set<_>>(),
        };

        // for ref a in &actions {
        //     self.gridview.execute(a);
        // }

        let do_nothing = |_: &RootGridView| {};
        self.gridview.take_paths(&paths, do_nothing);

        {
            // scope here to limit the range of the self.gridview borrow
            let info = cmd.dynamic_info(&self.gridview);
            let mut subview = pre_subview.clone().back(&mut self.gridview);

            for action in info.actions {
                subview.run_action(action);
            }
        }

        // teardown destinations if the droplets are still there
        // TODO is this ever going to be true?
        for id in in_ids {
            self.gridview.droplets.get_mut(&id).map(|droplet| {
                assert_eq!(Some(droplet.location), droplet.destination);
                droplet.destination = None;
            });
        }

        let item = ExecItem {
            routes: paths,
            command: cmd,
            placement: pre_subview,
        };

        self.exec_tx.send(item).unwrap();

        Ok(())
    }
}
