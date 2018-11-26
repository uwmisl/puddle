
use ::{PuddleResult};
use command::{BoxedCommand};
use grid::{Grid, DropletId, GridView};
use exec::Executor;

use plan::{Planner, PlanError, sched::SchedError};
use plan::graph::{Graph};
use std::sync::{Arc, Mutex};

pub struct System {
    #[allow(dead_code)]
    grid: Grid,
    graph: Graph,
    // TODO probably don't wanna have arc/mutex here
    planner: Arc<Mutex<Planner>>,
    executor: Executor,
}

impl System {

    pub fn new(grid: Grid) -> System {
        info!("Creating a system");
        let planner = {
            let gv = GridView::new(grid.clone());
            Planner::new(gv)
        };
        System {
            grid: grid.clone(),
            graph: Graph::new(),
            planner: Arc::new(Mutex::new(planner)),
            executor: Executor::new(grid.clone()),
        }
    }

    pub fn add(&mut self, cmd: BoxedCommand) -> PuddleResult<()> {
        // TODO unwrap
        info!("Adding command {:?}", cmd);
        let _cmd_id = self.graph.add_command(cmd).unwrap();
        Ok(())
    }

    // TODO switch to event loop here
    pub fn flush(&mut self, droplets: &[DropletId]) -> PuddleResult<()> {

        info!("Flushing...");
        loop {
            let phase = {
                // scope the planner lock
                let mut planner = self.planner.lock().unwrap();
                // FIXME unwrap
                match planner.plan(&self.graph, droplets) {
                    Ok(phase) => phase,
                    Err(PlanError::SchedError(SchedError::NothingToSchedule)) => break,
                    Err(e) => panic!("{:?}", e),
                }
            };

            // TODO For now this is blocking
            self.executor.run(phase, &mut self.graph);

            // TODO this is a little hacky
            let mut planner = self.planner.lock().unwrap();
            planner.gridview = self.executor.gridview.clone();
        }

        info!("Flushed!");

        Ok(())
    }
}
