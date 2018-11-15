
use ::{PuddleResult};
use command::{BoxedCommand, CommandRequest};
use grid::{Grid, DropletId, Droplet, GridView, Location};
use exec::Executor;

use plan::{Planner};
use plan::graph::{Graph, CmdIndex};
use std::sync::{Arc, Mutex};

struct System {
    grid: Grid,
    graph: Graph,
    // TODO probably don't wanna have arc/mutex here
    planner: Arc<Mutex<Planner>>,
    executor: Executor,
}

impl System {

    fn add(&mut self, cmd: BoxedCommand) -> Result<(), ()> {
        // TODO unwrap
        let _cmd_id = self.graph.add_command(cmd).unwrap();
        Ok(())
    }

    // TODO switch to event loop here
    fn flush(&mut self, droplets: &[DropletId]) -> PuddleResult<()> {
        let phase = {
            // scope the planner lock
            let mut planner = self.planner.lock().unwrap();
            // FIXME unwrap
            planner.plan(&self.graph, droplets).unwrap()
        };

        // FIXME For now this is blocking
        self.executor.run(&phase, &mut self.graph);

        Ok(())
    }
}
