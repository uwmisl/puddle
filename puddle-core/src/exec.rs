use crate::command::RunStatus;
use crate::grid::{DropletId, Grid, GridView};
use crate::plan::{
    graph::{CmdIndex, Graph},
    Path, PlanPhase, PlannedCommand,
};
use crate::util::HashMap;

pub struct Executor {
    pub gridview: GridView,
    running_commands: HashMap<CmdIndex, PlannedCommand>,
    ticks: usize,
}

pub enum ExecResponse {
    Ok,
}

impl Executor {
    pub fn new(grid: Grid) -> Executor {
        info!("Creating an Executor");
        Executor {
            gridview: GridView::new(grid),
            running_commands: HashMap::default(),
            ticks: 0,
        }
    }

    fn run_all_commands(&mut self, graph: &mut Graph) {
        let mut done = Vec::new();

        debug!("Run step, {} active commands", self.running_commands.len());

        // run each of the running commands one step
        for (&cmd_id, planned_cmd) in self.running_commands.iter() {
            let cmd = graph
                .graph
                .node_weight_mut(cmd_id)
                .expect("node not in graph")
                .as_mut()
                .expect("node unbound");
            let subview = &mut self.gridview.subview(&planned_cmd.placement);

            // write down if they are done
            debug!("Running command: {:?}", cmd);
            match cmd.run(subview) {
                RunStatus::Done => {
                    info!("Finalizing a command");

                    cmd.finalize(subview);
                    done.push(planned_cmd.cmd_id);
                }
                RunStatus::KeepGoing => (),
            }
        }

        self.commit();

        // clean up all the done ones
        for cmd_id in done {
            self.running_commands.remove(&cmd_id).unwrap();
        }
    }

    fn commit(&mut self) {
        self.ticks += 1;
    }

    fn take_routes(&mut self, paths: &HashMap<DropletId, Path>, graph: &mut Graph) {
        let max_len = paths.values().map(Vec::len).max().unwrap_or(0);

        // make sure that all droplets start where they are at this time step
        for (id, path) in paths.iter() {
            let droplet = &self.gridview.droplets[&id];
            assert_eq!(droplet.location, path[0]);
        }

        for i in 1..max_len {
            for (id, path) in paths.iter() {
                if i < path.len() {
                    let droplet = self.gridview.droplets.get_mut(id).unwrap();
                    assert!(droplet.location.distance_to(path[i]) <= 1);
                    droplet.location = path[i];
                }
            }
            self.run_all_commands(graph);
        }
    }

    pub fn run(&mut self, phase: PlanPhase, graph: &mut Graph) -> ExecResponse {
        info!("Run step");

        // this could be inefficient if one route is much much longer than another
        self.take_routes(&phase.routes, graph);

        // add all the planned commands
        for planned_cmd in phase.planned_commands {
            let was_there = self
                .running_commands
                .insert(planned_cmd.cmd_id, planned_cmd);
            assert!(was_there.is_none());
        }

        // just drive all commands to completion for now
        while !self.running_commands.is_empty() {
            self.run_all_commands(graph);
        }

        ExecResponse::Ok
    }

    pub fn ticks(&self) -> usize {
        self.ticks
    }
}
