use std::sync::mpsc::Receiver;

use command::Command;
use grid::{DropletId, DropletInfo, PreGridSubView, RootGridView};
use plan::Path;
use util::collections::Map;
use util::endpoint::Endpoint;

pub struct Executor {
    blocking: bool,
    gridview: RootGridView,
}

pub struct ExecItem {
    pub routes: Map<DropletId, Path>,
    pub command: Box<Command>,
    pub placement: PreGridSubView,
}

impl Executor {
    pub fn new(blocking: bool, gridview: RootGridView) -> Self {
        Executor { blocking, gridview }
    }

    pub fn run(&mut self, action_rx: Receiver<ExecItem>, endpoint: Endpoint<Vec<DropletInfo>, ()>) {
        let should_block = self.blocking;
        let block = move |gv: &RootGridView| {
            if should_block {
                // wait on the visualizer then reply
                match endpoint.recv() {
                    Ok(()) => {}
                    Err(_) => return,
                }
                endpoint.send(gv.droplet_info(None)).unwrap();
            }
        };

        loop {
            let item = match action_rx.recv() {
                Ok(item) => item,
                Err(_) => return,
            };

            self.gridview.take_paths(&item.routes, &block);

            // NOTE: invariant
            // this gridview (the executor's) should be identical to the
            // Planner's gridview at the time this thing was planned
            let info = item.command.dynamic_info(&self.gridview);
            let mut subview = item.placement.back(&mut self.gridview);

            for action in info.actions {
                subview.run_action(action);
            }
        }
    }
}
