
use ::{PuddleResult};
use command::{BoxedCommand, CommandRequest};
use grid::{Grid, DropletId, Droplet, GridView, Location, Snapshot};
use exec::Executor;

use plan::{Planner};
use std::sync::{Arc, Mutex};

// struct Runner {
//     plan: Plan,
// }

struct System {
    grid: Grid,
    planner: Arc<Mutex<Planner>>,
    // TODO probably don't wanna have arc/mutex here
    execuctor: Arc<Mutex<Executor>>,
}

impl System {

    fn add(&self, cmd: BoxedCommand) -> Result<(), ()> {
        let mut planner = self.planner.lock().unwrap();
        let _cmd_id = planner.add(&self.grid, cmd)?;
        Ok(())
    }

    fn flush(&self, droplets: &[DropletId]) -> PuddleResult<()> {
        let planned_phases = {
            // scope the planner lock
            let mut planner = self.planner.lock().unwrap();
            // FIXME unwrap
            planner.plan(&self.grid, droplets).unwrap()
        };

        let mut executor = self.execuctor.lock().unwrap();
        // FIXME For now this is blocking
        for phase in planned_phases {
            executor.run(&phase)
        }

        Ok(())
    }
}
