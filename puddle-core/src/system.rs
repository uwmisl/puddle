use crate::command::BoxedCommand;
use crate::exec::Executor;
use crate::grid::{DropletId, Grid, GridView};
use crate::process::PuddleResult;

use crate::plan::graph::Graph;
use crate::plan::{sched::SchedError, PlanError, Planner};

pub struct System {
    #[allow(dead_code)]
    grid: Grid,
    graph: Graph,
    planner: Planner,
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
            graph: Graph::default(),
            planner,
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
            let phase = match self.planner.plan(&self.graph, droplets) {
                Ok(phase) => phase,
                Err(PlanError::SchedError(SchedError::NothingToSchedule)) => break,
                Err(e) => panic!("{:?}", e),
            };

            // TODO For now this is blocking
            self.executor.run(phase, &mut self.graph);

            // TODO this is a little hacky
            self.planner.gridview = self.executor.gridview.clone();
        }

        info!("Flushed!");

        Ok(())
    }

    pub fn ticks(&self) -> usize {
        self.executor.ticks()
    }
}
