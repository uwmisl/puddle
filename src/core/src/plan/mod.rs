// TODO move graph
pub mod graph;
pub mod place;
mod route;
pub mod sched;

use self::graph::{CmdIndex, Graph};
use self::place::{Placement, PlacementRequest, Placer};
use self::route::{Router, RoutingRequest};
use self::sched::{SchedRequest, Scheduler};

pub use self::route::Path;

use command::{BoxedCommand, Command, CommandRequest};
use grid::{
    droplet::{Droplet, DropletId},
    Grid, GridView, Location,
};
use std::collections::HashMap;
use util::collections::Map;

#[derive(Debug)]
pub enum PlanError {
    RouteError {
        placement: Placement,
        droplets: Vec<Droplet>,
    },
    PlaceError,
    SchedError(self::sched::SchedError),
}

pub struct PlannedCommand {
    pub cmd_id: CmdIndex,
    pub placement: Placement,
}

pub struct PlanPhase {
    pub routes: Map<DropletId, Path>,
    pub planned_commands: Vec<PlannedCommand>,
}

type PlanResult = Result<PlanPhase, PlanError>;

pub struct Planner {
    pub gridview: GridView,
    scheduler: Scheduler,
    placer: Placer,
    router: Router,
}

impl Planner {
    pub fn new(gridview: GridView) -> Planner {
        Planner {
            gridview: gridview,
            scheduler: Scheduler::new(),
            placer: Placer::new(),
            router: Router::new(),
        }
    }

    pub fn plan(&mut self, graph: &Graph, _droplets: &[DropletId]) -> PlanResult {
        debug!("Planning GV: {:#?}", self.gridview.droplets);
        self.gridview.check_no_collision();

        // FIXME get rid of unwraps
        let sched_resp = {
            let req = SchedRequest { graph };
            debug!("Schedule request");
            let resp = self.scheduler.schedule(&req)?;
            debug!("{:?}", resp);
            for cmd_id in &resp.commands_to_run {
                debug!("Gonna schedule {:?}: {:?}", cmd_id, graph.graph[*cmd_id])
            }
            resp
        };

        let command_requests: Vec<_> = sched_resp
            .commands_to_run
            .iter()
            .map(|cmd_id: &CmdIndex| {
                let cmd = graph.graph[*cmd_id].as_ref().expect("Command was unbound!");
                let cmd_req = cmd.request(&self.gridview);
                // TODO update the outputs
                // for out in cmd_req.outputs {
                //     self.droplets.insert(out.id, out);
                // }
                cmd_req
            }).collect();

        let place_resp = {
            let req = PlacementRequest {
                gridview: &self.gridview,
                fixed_commands: vec![],
                commands: command_requests.as_slice(),
                stored_droplets: sched_resp.droplets_to_store.as_slice(),
            };
            let resp = self.placer.place(&req).unwrap();
            debug!("{:?}", resp);
            resp
        };

        let route_resp = {

            let mut droplets = Vec::new();

            let stored = sched_resp.droplets_to_store.iter().zip(place_resp.stored_droplets);
            for (id, loc) in stored {
                let droplet = self.gridview.droplets.get_mut(id).unwrap();
                droplet.destination = Some(loc);
                droplets.push(droplet.clone())
            }

            // TODO getting these input droplets is pretty painful
            let placed = sched_resp.commands_to_run.iter().zip(command_requests).zip(&place_resp.commands);
            for ((cmd_id, req), placement) in placed {
                let cmd = graph.graph[*cmd_id].as_ref().expect("Command was unbound!");
                let in_ids = cmd.input_droplets();
                let ins = in_ids.iter().zip(req.input_locations);
                for (droplet_id, location) in ins {
                    let droplet = self.gridview.droplets.get_mut(droplet_id).unwrap();
                    debug!("mapping: {:#?}", placement);
                    droplet.destination = Some(placement.mapping[&location]);
                    droplets.push(droplet.clone())
                }
            }


            let req = RoutingRequest {
                grid: &self.gridview.grid,
                blockages: vec![],
                droplets: droplets,
            };
            let resp = self.router.route(&req).unwrap();
            debug!("{:?}", resp);
            resp
        };

        let routes = route_resp.routes;
        let planned_commands: Vec<_> = sched_resp
            .commands_to_run
            .iter()
            .zip(place_resp.commands)
            .map(|(&cmd_id, placement)| PlannedCommand { cmd_id, placement })
            .collect();


        // now commit to the schedule
        self.scheduler.commit(&sched_resp);

        Ok(PlanPhase {
            routes,
            planned_commands,
        })
    }
}

// impl GridView {
//     pub fn plan(&mut self, mut cmd: Box<dyn Command>) -> Result<(), (Box<dyn Command>, PlanError)> {
//         info!("Planning {:?}", cmd);

//         // make sure there's a snapshot available to plan into
//         self.snapshot_ensure();
//         if cmd.bypass(&self) {
//             info!("Bypassing command: {:#?}", cmd);
//             return Ok(());
//         }

//         let in_ids = cmd.input_droplets();
//         // FIXME
//         let req = cmd.request(unimplemented!());

//         debug!(
//             "Command requests a shape of w={w},h={h}",
//             w = req.shape.max_width(),
//             h = req.shape.max_height(),
//         );

//         debug!(
//             "Input droplets: {:?}",
//             cmd.input_droplets()
//                 .iter()
//                 .map(|id| &self.snapshot().droplets[id])
//                 .collect::<Vec<_>>()
//         );

//         let placement_mapping = if req.trusted {
//             // if we are trusting placement, just use an identity map
//             self.grid
//                 .locations()
//                 .map(|(loc, _cell)| (loc, loc))
//                 .collect::<Map<_, _>>()
//         } else {
//             // TODO place should be a method of gridview
//             let mut snapshot: Snapshot = self.snapshot().new_with_same_droplets();

//             for id in &in_ids {
//                 snapshot.droplets.remove(id);
//             }
//             match self.grid.place(&req.shape, &snapshot, &self.bad_edges) {
//                 None => return Err((cmd, PlanError::PlaceError)),
//                 Some(placement_mapping) => placement_mapping,
//             }
//         };

//         debug!("placement for {:#?}: {:#?}", cmd, placement_mapping);

//         assert_eq!(req.input_locations.len(), in_ids.len());

//         for (loc, id) in req.input_locations.iter().zip(&in_ids) {
//             // this should have been put to none last time
//             let droplet = self
//                 .snapshot_mut()
//                 .droplets
//                 .get_mut(&id)
//                 .expect("Command gave back and invalid DropletId");

//             assert!(droplet.destination.is_none());

//             let mapped_loc = placement_mapping.get(loc).unwrap_or_else(|| {
//                 panic!(
//                     "Input location {} wasn't in placement.\n  All input locations: {:?}",
//                     loc, req.input_locations
//                 )
//             });
//             droplet.destination = Some(*mapped_loc);
//         }

//         debug!("routing {:?}", cmd);
//         let paths = match self.route() {
//             Some(p) => p,
//             None => {
//                 return Err((
//                     cmd,
//                     PlanError::RouteError {
//                         placement: Placement {
//                             mapping: placement_mapping,
//                         },
//                         droplets: self.snapshot().droplets.values().cloned().collect(),
//                     },
//                 ))
//             }
//         };
//         debug!("route for {:#?}: {:#?}", cmd, paths);

//         trace!("Taking paths...");
//         // FIXME final tick is a hack
//         // we *carefully* pre-run the command before making the final tick
//         let final_tick = false;
//         self.take_paths(&paths, final_tick);

//         {
//             let mut subview = self.subview(in_ids.iter().cloned(), placement_mapping.clone());

//             trace!("Pre-Running command {:?}", cmd);
//             cmd.pre_run(&mut subview);
//             subview.tick();

//             trace!("Running command {:?}", cmd);
//             cmd.run(&mut subview);
//         }

//         self.register(cmd);

//         // teardown destinations if the droplets are still there
//         // TODO is this ever going to be true?
//         for id in in_ids {
//             if let Some(droplet) = self.snapshot_mut().droplets.get_mut(&id) {
//                 assert_eq!(Some(droplet.location), droplet.destination);
//                 droplet.destination = None;
//             };
//         }

//         Ok(())
//     }
// }

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

    // #[test]
    // fn plan_input() {
    //     let mut gv = mk_gv("tests/arches/purpledrop.json");
    //     let cmd = {
    //         let substance = "input".into();
    //         let volume = 1.0;
    //         let dimensions = Location { y: 3, x: 3 };
    //         let out_id = DropletId {
    //             id: 0,
    //             process_id: 0,
    //         };
    //         command::Input::new(substance, volume, dimensions, out_id).unwrap()
    //     };
    //     gv.plan(Box::new(cmd)).unwrap();
    // }

    // #[test]
    // fn plan_output() {
    //     let mut gv = mk_gv("tests/arches/purpledrop.json");

    //     let id = DropletId {
    //         id: 0,
    //         process_id: 0,
    //     };
    //     let droplet = {
    //         let volume = 1.0;
    //         let location = Location { y: 7, x: 0 };
    //         let dimensions = Location { y: 1, x: 1 };
    //         Droplet::new(id, volume, location, dimensions)
    //     };
    //     gv.snapshot_mut().droplets.insert(id, droplet);

    //     let cmd = {
    //         let substance = "output".into();
    //         command::Output::new(substance, id).unwrap()
    //     };

    //     gv.plan(Box::new(cmd)).unwrap();
    // }

}
