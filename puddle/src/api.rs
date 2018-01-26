use std::sync::{RwLock};
use std::convert::From;

use jsonrpc_core as rpc;

use arch::{DropletId, Architecture};
use command::*;


pub struct Session {
    lock: RwLock<Architecture>
}

#[derive(Debug)]
pub enum ExecutionError {
    RouteError,
    PlaceError,
}

#[derive(Debug)]
pub enum PuddleError {
    ExecutionError(ExecutionError),
    NonExistentDropletId(DropletId),
}

pub type PResult<T> = Result<T, PuddleError>;


build_rpc_trait! {
	  pub trait Rpc {
		    #[rpc(name = "mix")]
		    fn mix(&self, DropletId, DropletId) -> PResult<DropletId>;
	  }
}

impl From<PuddleError> for rpc::Error {
    fn from(p_err: PuddleError) -> Self {
        let code = rpc::ErrorCode::ServerError(0);
        let mut err = rpc::Error::new(code);
        err.message = format!("PuddleError: {:?}", p_err);
        err
    }
}

impl Session {

    pub fn new(arch: Architecture) -> Self {
        Session {
            lock: RwLock::new(arch),
        }
    }

    fn execute<Cmd: Command>(&self, cmd: &Cmd) -> Result<(), ExecutionError> {
        let mut arch = self.lock.write().unwrap();
        let mapping = match arch.grid.place(cmd.shape()) {
            None => return Err(ExecutionError::PlaceError),
            Some(m) => m,
        };

        println!("Mapping: {:?}", mapping);

        let in_locs = cmd.input_locations();
        let in_ids = cmd.input_droplets();

        assert_eq!(in_locs.len(), in_ids.len());

        // set up destinations
        for (loc, id) in in_locs.iter().zip(in_ids) {
            // this should have been put to none last time
            let droplet = arch.droplets.get_mut(id).expect(
                "Command gave back and invalid DropletId"
            );
            assert!(droplet.destination.is_none());
            let mapped_loc = mapping.get(loc)
                .expect("input location wasn't in mapping");
            droplet.destination = Some(*mapped_loc);
        }

        let paths = match arch.route() {
            None => return Err(ExecutionError::RouteError),
            Some(p) => p,
        };

        arch.take_paths(paths);
        cmd.run(&mut arch, &mapping);

        // teardown destinations if the droplets are still there
        // TODO is this ever going to be true?
        for id in in_ids {
            arch.droplets.get_mut(id).map(
                |droplet| {
                    assert_eq!(Some(droplet.location), droplet.destination);
                    droplet.destination = None;
                }
            );
        }

        Ok(())
    }

}

impl Rpc for Session {
    fn mix(&self, d1: DropletId, d2: DropletId) -> PResult<DropletId> {
        // have to pattern match here so the lock use is properly scoped
        // TODO think of a better pattern
        let mix_cmd = match self.lock.write() {
            Err(e) => panic!("Lock failed with: {}", e),
            Ok(mut arch) => {
                let mix_cmd = Mix::new(&mut arch, d1, d2)?;
                // safe to unwrap here because Mix::new checked them
                assert_eq!(
                    arch.droplets.get(&d1).unwrap().collision_group,
                    arch.droplets.get(&d2).unwrap().collision_group,
                );
                mix_cmd
            }
        };
        self.execute(&mix_cmd).expect("can't handle failures in api yet");
        Ok(mix_cmd.output_droplets()[0])
    }
}


#[cfg(test)]
mod tests {

    use super::*;

    use arch::{Location};
    use arch::grid::Grid;

    #[test]
    fn execute_mix_command() {
        let mut arch = Architecture::from_grid(
            Grid::rectangle(4,3)
        );

        let id1 = arch.new_droplet_id();
        let id2 = arch.new_droplet_id();

        let dr1 = arch.droplet_from_location(Location {x: 2, y: 3});
        arch.droplets.insert(id1, dr1);
        let dr2 = arch.droplet_from_location(Location {x: 1, y: 1});
        arch.droplets.insert(id2, dr2);

        let session = Session::new(arch);

        let id3 = session.mix(id1, id2).expect("Mix failed");
        let arch = session.lock.read().unwrap();
        let ids: Vec<&DropletId> = arch.droplets.keys().collect();
        assert_eq!(ids, vec![&id3])
    }
}
