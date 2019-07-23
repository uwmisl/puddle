// TODO move graph
pub mod graph;
pub mod place;
mod route;
pub mod sched;

use self::graph::{CmdIndex, Graph};
use self::place::{Placement, PlacementRequest, Placer};
use self::route::{Agent, Router, RoutingRequest};
use self::sched::{SchedRequest, Scheduler};

pub use self::route::Path;

use crate::grid::{droplet::DropletId, GridView};
use crate::util::HashMap;

#[derive(Debug)]
pub enum PlanError {
    RouteError(self::route::RoutingError),
    SchedError(self::sched::SchedError),
    PlaceError(self::place::PlacementError),
}

pub struct PlannedCommand {
    pub cmd_id: CmdIndex,
    pub placement: Placement,
}

pub struct PlanPhase {
    pub routes: HashMap<DropletId, Path>,
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
            scheduler: Scheduler::default(),
            placer: Placer::default(),
            router: Router::default(),
        }
    }

    pub fn plan(&mut self, graph: &Graph, _droplets: &[DropletId]) -> PlanResult {
        debug!("Planning GV: {:#?}", self.gridview.droplets);
        self.gridview.check_no_collision();

        let sched_resp = {
            let req = SchedRequest { graph };
            debug!("Schedule request");
            let resp = self
                .scheduler
                .schedule(&req)
                .map_err(PlanError::SchedError)?;
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
                cmd.request(&self.gridview)
                // TODO update the outputs
                // for out in cmd_req.outputs {
                //     self.droplets.insert(out.id, out);
                // }
            })
            .collect();

        let place_resp = {
            let req = PlacementRequest {
                gridview: &self.gridview,
                fixed_commands: vec![],
                commands: command_requests.as_slice(),
                stored_droplets: sched_resp.droplets_to_store.as_slice(),
            };
            let resp = self.placer.place(&req).map_err(PlanError::PlaceError)?;
            debug!("{:?}", resp);
            resp
        };

        let route_resp = {
            let mut agents: Vec<_> = sched_resp
                .droplets_to_store
                .iter()
                .zip(place_resp.stored_droplets)
                .map(|(id, loc)| Agent::from_droplet(&self.gridview.droplets[id], loc))
                .collect();

            // TODO getting these input droplets is pretty painful
            let placed = sched_resp
                .commands_to_run
                .iter()
                .zip(command_requests)
                .zip(&place_resp.commands);
            for ((cmd_id, req), placement) in placed {
                let cmd = graph.graph[*cmd_id].as_ref().expect("Command was unbound!");
                let in_ids = cmd.input_droplets();
                let ins = in_ids.iter().zip(req.input_locations);
                for (&droplet_id, location) in ins {
                    agents.push(self::route::Agent {
                        id: droplet_id,
                        source: self.gridview.droplets[&droplet_id].location,
                        dimensions: self.gridview.droplets[&droplet_id].dimensions,
                        destination: placement.mapping[&location],
                    });
                }
            }

            let req = RoutingRequest {
                agents,
                gridview: &self.gridview,
                blockages: vec![],
            };
            // debug!("{:?}", req);
            let resp = self.router.route(&req).map_err(PlanError::RouteError)?;
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
