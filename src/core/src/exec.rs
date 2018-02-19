use std::sync::mpsc::{Sender, Receiver};

use util::endpoint::Endpoint;
use grid::{DropletId, DropletInfo, Grid, GridView, Location};
use plan::plan::Placement;

#[derive(Debug)]
pub enum Action {
    AddDroplet {
        id: DropletId,
        location: Location,
    },
    RemoveDroplet {
        id: DropletId,
    },
    Mix {
        in0: DropletId,
        in1: DropletId,
        out: DropletId,
    },
    Split {
        inp: DropletId,
        out0: DropletId,
        out1: DropletId,
    },
    UpdateDroplet {
        old_id: DropletId,
        new_id: DropletId,
        // TODO take a closure here
    },
    MoveDroplet {
        id: DropletId,
        location: Location,
    },
    Tick,
    // TODO should be more general
    Ping {
        tx: Sender<()>,
    },
}

impl Action {
    #[allow(unused_variables)]
    pub fn translate(&mut self, placement: &Placement) {
        use self::Action::*;
        match *self {
            AddDroplet { id, ref mut location } => {
                *location = placement[location];
            }
            RemoveDroplet { id } => {}
            Mix { in0, in1, out } => {}
            Split { inp, out0, out1 } => {}
            UpdateDroplet { old_id, new_id } => {}
            MoveDroplet { id, ref mut location } => {
                *location = placement[location];
            }
            Tick => {},
            Ping { ref tx } => {}
        }
    }
}

pub struct Executor {
    blocking: bool,
    gridview: GridView,
}

impl Executor {
    pub fn new(blocking: bool, grid: Grid) -> Self {
        Executor {
            blocking: blocking,
            gridview: GridView::new(grid),
        }
    }

    fn execute(&mut self, action: &Action) -> bool {
        debug!("executing {:?}", action);
        use self::Action::*;
        let keep_going = match action {
            &Ping { ref tx } => {
                tx.send(()).unwrap();
                true
            }
            &Tick => false,
            _ => true
        };
        self.gridview.execute(action);
        keep_going
    }

    pub fn run(&mut self, action_rx: Receiver<Action>, endpoint: Endpoint<Vec<DropletInfo>, ()>) {
        loop {
            // wait on the visualizer then reply
            if self.blocking {
                match endpoint.recv() {
                    Ok(()) => {},
                    Err(_) => return
                }
                endpoint.send(self.gridview.droplet_info(None)).unwrap();
            }

            // now execute until we see a tick
            let mut keep_going = true;
            while keep_going {
                let action = match action_rx.recv() {
                    Ok(action) => action,
                    Err(_) => return
                };
                keep_going = self.execute(&action);
            }
        }
    }
}
