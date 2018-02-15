use grid::{Droplet, DropletId, DropletInfo, GridView, Location};
use exec::Action;
use std::sync::mpsc::Sender;
use process::ProcessId;
use command::Command;
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
    gridview: GridView,
    exec_tx: Sender<Action>,
}

impl Planner {
    pub fn new(gridview: GridView, exec_tx: Sender<Action>) -> Planner {
        Planner {
            gridview: gridview,
            exec_tx: exec_tx,
        }
    }

    pub fn droplet_info(&self, pid: ProcessId) -> Vec<DropletInfo> {
        self.gridview.droplet_info(Some(pid))
    }

    pub fn plan<C: Command>(&mut self, cmd: C) -> Result<(), PlanError> {
        eprintln!("Running: {:?}", cmd);

        cmd.pre_plan(&mut self.gridview);

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
                .place(cmd.shape(), &self.gridview.droplets)
                .ok_or(PlanError::PlaceError)?
        };

        eprintln!("Placement: {:?}", placement);

        let in_locs = cmd.input_locations();
        let in_ids = cmd.input_droplets();

        assert_eq!(in_locs.len(), in_ids.len());

        for (loc, id) in in_locs.iter().zip(in_ids) {
            // this should have been put to none last time
            let droplet = self.gridview
                .droplets
                .get_mut(id)
                .expect("Command gave back and invalid DropletId");
            assert!(droplet.destination.is_none());
            let mapped_loc = placement
                .get(loc)
                .expect("input location wasn't in placement");
            droplet.destination = Some(*mapped_loc);
        }

        let paths = match self.gridview.route() {
            None => {
                return Err(PlanError::RouteError {
                    placement: placement,
                    droplets: self.gridview.droplets.values().map(|d| d.clone()).collect(),
                })
            }
            Some(p) => p,
        };

        assert_eq!(
            self.gridview.droplets.keys().collect::<Set<&DropletId>>(),
            paths.keys().collect::<Set<&DropletId>>(),
        );

        let max_len = paths.values().map(|p| p.len()).max().unwrap_or(0);
        let mut actions: Vec<Action> = (0..max_len)
            .map(|i| Action::Lockstep {
                actions: paths
                    .iter()
                    .filter(|&(_id, path)| i < path.len())
                    .map(|(id, path)| Action::MoveDroplet {
                        id: *id,
                        location: path[i],
                    })
                    .collect(),
            })
            .collect();

        let mut cmd_actions = cmd.actions();
        for mut a in &mut cmd_actions {
            a.translate(&placement);
        }
        eprintln!("{:?}", cmd_actions);
        actions.append(&mut cmd_actions);

        for ref a in &actions {
            self.gridview.execute(a);
        }

        for a in actions {
            self.exec_tx.send(a).unwrap();
        }

        // teardown destinations if the droplets are still there
        // TODO is this ever going to be true?
        for id in in_ids {
            self.gridview.droplets.get_mut(id).map(|droplet| {
                assert_eq!(Some(droplet.location), droplet.destination);
                droplet.destination = None;
            });
        }

        Ok(())
    }
}
