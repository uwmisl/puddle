use plan::{PlanPhase, Path, PlannedCommand, graph::{Graph, CmdIndex}};
use grid::{GridView, DropletId};
use util::collections::{Map, Set};
use command::{Command, RunStatus};

pub struct Executor {
    gridview: GridView,
    running_commands: Map<CmdIndex, PlannedCommand>,
    done_commands: Vec<CmdIndex>,
}

pub enum ExecResponse {
    Ok
}

impl Executor {
    pub fn new() -> Executor {
        unimplemented!()
    }

    fn run_all_commands(&mut self, graph: &mut Graph) {
        let mut done = Vec::new();

        // run each of the running commands one step
        for (&cmd_id, planned_cmd) in self.running_commands.iter() {
            let cmd = graph.graph.node_weight_mut(cmd_id)
                .expect("node not in graph")
                .as_mut()
                .expect("node unbound");
            let mut subview = &mut self.gridview.subview(&planned_cmd.placement);

            // write down if they are done
            match cmd.run(subview) {
                RunStatus::Done => {
                    cmd.finalize(subview);
                    done.push(planned_cmd.cmd_id);
                }
                RunStatus::KeepGoing => (),
            }
        }

        // clean up all the done ones
        for cmd_id in done {
            self.running_commands.remove(&cmd_id).unwrap();
        }
    }

    fn commit(&mut self) {}

    fn take_routes(&mut self, paths: &Map<DropletId, Path>, graph: &mut Graph) {
        let max_len = paths.values().map(|path| path.len()).max().unwrap_or(0);

        // make sure that all droplets start where they are at this time step
        for (id, path) in paths.iter() {
            let droplet = &self.gridview.droplets[&id];
            assert_eq!(droplet.location, path[0]);
        }

        for i in 1..max_len {
            for (id, path) in paths.iter() {
                if i < path.len() {
                    let droplet = self.gridview.droplets.get_mut(id).unwrap();
                    assert!(droplet.location.distance_to(&path[i]) <= 1);
                    droplet.location = path[i];
                }
            }
            self.run_all_commands(graph);
            self.commit();
        }
    }

    pub fn run(&mut self, phase: &PlanPhase, graph: &mut Graph) -> ExecResponse {
        //
        assert_eq!(self.done_commands, []);

        // this could be inefficient if one route is much much longer than another
        self.take_routes(&phase.routes, graph);
        ExecResponse::Ok
    }
}
