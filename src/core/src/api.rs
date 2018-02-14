use std::sync::{RwLock, Mutex, Condvar};
use std::convert::From;

use jsonrpc_core as rpc;

use std::collections::{HashMap, HashSet};
use arch::{DropletId, DropletInfo, Architecture, Location};
use command::*;


pub struct Session {
    // TODO is fine grained locking the right thing there?
    arch: RwLock<Architecture>,
    commands: Mutex<Vec<Box<Command>>>,
    sync: bool,
    step_seen: Mutex<bool>,
    step_signal: Condvar,
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

pub type PuddleResult<T> = Result<T, PuddleError>;


build_rpc_trait! {
	  pub trait Rpc {
		    #[rpc(name = "flush")]
        fn flush(&self) -> PuddleResult<()>;

		    #[rpc(name = "input")]
        fn input(&self, Location) -> PuddleResult<DropletId>;

		    #[rpc(name = "move")]
        fn move_droplet(&self, DropletId, Location) -> PuddleResult<DropletId>;

		    #[rpc(name = "mix")]
        fn mix(&self, DropletId, DropletId) -> PuddleResult<DropletId>;

		    #[rpc(name = "split")]
        fn split(&self, DropletId) -> PuddleResult<(DropletId, DropletId)>;

		    #[rpc(name = "droplets")]
        fn droplets(&self) -> PuddleResult<HashMap<DropletId, DropletInfo>>;

		    #[rpc(name = "visualize_droplets")]
        fn visualize_droplets(&self) -> PuddleResult<HashMap<DropletId, DropletInfo>>;
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
    pub fn new(arch: Architecture) -> Session {
        Session {
            arch: RwLock::new(arch),
            commands: Mutex::new(Vec::new()),
            sync: false,
            // start with true because every step we've taken so far has been seen
            step_seen: Mutex::new(true),
            step_signal: Condvar::new(),
        }
    }

    pub fn sync(mut self, should_sync: bool) -> Self {
        self.sync = should_sync;
        self
    }

    pub fn wait(&self) {
        if self.sync {
            println!(">>> WAITING");
            let mut seen = self.step_seen.lock().unwrap();
            while !*seen {
                seen = self.step_signal.wait(seen).unwrap();
            }
            *seen = false;
            println!("<<< WAITING");
        }
    }

    fn register<Cmd>(&self, cmd: Cmd)
    where
        Cmd: Command,
    {
        let mut commands = self.commands.lock().unwrap();
        commands.push(Box::new(cmd));
    }

    fn execute(&self, cmd: Box<Command>) -> Result<(), ExecutionError> {

        let mut arch = self.arch.write().unwrap();
        println!("Running: {:?}", cmd);

        cmd.pre_run(&mut arch);

        let mapping: HashMap<Location, Location> = if cmd.trust_placement() {
            // if we are trusting placement, just use an identity map
            arch.grid
                .locations()
                .map(|(loc, _cell)| (loc, loc))
                .collect()
        } else {
            match arch.grid.place(cmd.shape(), &arch.droplets) {
                None => return Err(ExecutionError::PlaceError),
                Some(m) => m,
            }
        };

        // println!("Mapping: {:?}", mapping);

        let in_locs = cmd.input_locations();
        let in_ids = cmd.input_droplets();

        assert_eq!(in_locs.len(), in_ids.len());

        for (loc, id) in in_locs.iter().zip(in_ids) {
            // this should have been put to none last time
            let droplet = arch.droplets.get_mut(id).expect(
                "Command gave back and invalid DropletId",
            );
            assert!(droplet.destination.is_none());
            let mapped_loc = mapping.get(loc).expect("input location wasn't in mapping");
            droplet.destination = Some(*mapped_loc);
        }

        let paths = match arch.route() {
            None => return Err(ExecutionError::RouteError),
            Some(p) => p,
        };

        assert_eq!(
            arch.droplets.keys().collect::<HashSet<&DropletId>>(),
            paths.keys().collect::<HashSet<&DropletId>>(),
        );

        use std::mem::drop;
        drop(arch);

        Architecture::take_paths(&self.arch, paths, || { self.wait(); });

        cmd.run(&self.arch, &mapping, &|| self.wait());

        let mut arch = self.arch.write().unwrap();

        // teardown destinations if the droplets are still there
        // TODO is this ever going to be true?
        for id in in_ids {
            arch.droplets.get_mut(id).map(|droplet| {
                assert_eq!(Some(droplet.location), droplet.destination);
                droplet.destination = None;
            });
        }

        Ok(())
    }
}

impl Rpc for Session {
    fn flush(&self) -> PuddleResult<()> {
        let mut commands = self.commands.lock().unwrap();

        for cmd in commands.drain((0..)) {
            self.execute(cmd).expect("can't handle api failures yet");
        }
        Ok(())
    }

    fn input(&self, loc: Location) -> PuddleResult<DropletId> {
        let mut arch = self.arch.write().unwrap();
        let input_cmd = Input::new(&mut arch, loc)?;
        let out = input_cmd.output_droplets()[0];
        self.register(input_cmd);
        Ok(out)
    }

    fn move_droplet(&self, d1: DropletId, loc: Location) -> PuddleResult<DropletId> {
        let mut arch = self.arch.write().unwrap();
        let move_cmd = Move::new(&mut arch, d1, loc)?;
        let out = move_cmd.output_droplets()[0];
        self.register(move_cmd);
        Ok(out)
    }

    fn mix(&self, d1: DropletId, d2: DropletId) -> PuddleResult<DropletId> {
        let mut arch = self.arch.write().unwrap();
        let mix_cmd = Mix::new(&mut arch, d1, d2)?;
        let out = mix_cmd.output_droplets()[0];
        self.register(mix_cmd);
        Ok(out)
    }

    fn split(&self, d1: DropletId) -> PuddleResult<(DropletId, DropletId)> {
        let mut arch = self.arch.write().unwrap();
        let split_cmd = Split::new(&mut arch, d1)?;
        let out0 = split_cmd.output_droplets()[0];
        let out1 = split_cmd.output_droplets()[1];
        self.register(split_cmd);
        Ok((out0, out1))
    }

    fn droplets(&self) -> PuddleResult<HashMap<DropletId, DropletInfo>> {
        self.flush()?;
        let arch = self.arch.read().unwrap();

        let droplets = arch.droplets
            .iter()
            .map(|(id, droplet)| (*id, droplet.info()))
            .collect();

        Ok(droplets)
    }

    fn visualize_droplets(&self) -> PuddleResult<HashMap<DropletId, DropletInfo>> {
        // DONT FLUSH
        let arch = self.arch.read().unwrap();

        let droplets = arch.droplets
            .iter()
            .map(|(id, droplet)| (*id, droplet.info()))
            .collect();

        let mut seen = self.step_seen.lock().unwrap();
        *seen = true;
        self.step_signal.notify_one();

        Ok(droplets)
    }
}


#[cfg(test)]
mod tests {

    use super::*;

    use arch::Location;
    use arch::grid::Grid;

    //
    //  Non-Lazy Tests
    //

    #[test]
    fn execute_input_command() {
        let session = Session::new(Architecture::from_grid(Grid::rectangle(1, 1)));

        // todo: this shouldn't work
        let loc = Location { y: 3, x: 3 };
        let id = session.input(loc).unwrap();
        session.flush().unwrap();

        let arch = session.arch.read().unwrap();
        let ids: Vec<&DropletId> = arch.droplets.keys().collect();
        assert_eq!(ids, vec![&id]);

        let dr = arch.droplets.get(&id).unwrap();
        assert_eq!(loc, dr.location)
    }

    #[test]
    fn execute_move_command() {
        let session = Session::new(Architecture::from_grid(Grid::rectangle(2, 2)));

        let loc = Location { y: 1, x: 1 };

        let id = session.input(loc).unwrap();
        let id1 = session.move_droplet(id, loc).unwrap();
        session.flush().unwrap();

        let arch = session.arch.read().unwrap();
        let ids: Vec<&DropletId> = arch.droplets.keys().collect();

        assert_eq!(ids, vec![&id1]);

        let dr = arch.droplets.get(&id1).unwrap();
        assert_eq!(loc, dr.location)
    }

    #[test]
    fn execute_mix_command() {
        let session = Session::new(Architecture::from_grid(Grid::rectangle(4, 3)));

        let id1 = session.input(Location { x: 2, y: 3 }).unwrap();
        let id2 = session.input(Location { x: 1, y: 1 }).unwrap();
        let id3 = session.mix(id1, id2).unwrap();
        session.flush().unwrap();

        let arch = session.arch.read().unwrap();
        let ids: Vec<&DropletId> = arch.droplets.keys().collect();

        assert_eq!(ids, vec![&id3])
    }

    #[test]
    fn execute_split_command() {
        let session = Session::new(Architecture::from_grid(Grid::rectangle(1, 5)));

        let id = session.input(Location { x: 2, y: 0 }).unwrap();
        let (id1, id2) = session.split(id).unwrap();

        session.flush().unwrap();

        let arch = session.arch.read().unwrap();
        let ids: Vec<&DropletId> = arch.droplets.keys().collect();

        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&&id1));
        assert!(ids.contains(&&id2))
    }

    #[test]
    fn execute_all_commands() {
        let session = Session::new(Architecture::from_grid(Grid::rectangle(10, 10)));

        // todo: add move into here.
        let id1 = session.input(Location { y: 2, x: 2 }).unwrap();
        let id2 = session.input(Location { y: 0, x: 0 }).unwrap();
        let id3 = session.mix(id1, id2).unwrap();
        let (id4, id5) = session.split(id3).unwrap();
        let id6 = session.input(Location { y: 7, x: 7 }).unwrap();
        let id7 = session.move_droplet(id6, Location { y: 5, x: 5 }).unwrap();
        let id8 = session.mix(id4, id7).unwrap();

        session.flush().unwrap();

        let arch = session.arch.read().unwrap();
        let ids: Vec<&DropletId> = arch.droplets.keys().collect();

        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&&id5));
        assert!(ids.contains(&&id8))
    }

    //
    //  Laziness Tests
    //

    #[test]
    fn lazy_input_command() {
        let session = Session::new(Architecture::from_grid(Grid::rectangle(1, 1)));

        // TODO: this shouldn't work?
        let loc = Location { y: 3, x: 3 };
        let _ = session.input(loc).unwrap();

        let arch = session.arch.read().unwrap();
        let ids: Vec<&DropletId> = arch.droplets.keys().collect();
        assert_eq!(ids.len(), 0);
    }

    #[test]
    fn lazy_move_command() {
        let session = Session::new(Architecture::from_grid(Grid::rectangle(2, 2)));

        let loc = Location { y: 1, x: 1 };

        let id = session.input(loc).unwrap();

        session.flush().unwrap();

        let _ = session.move_droplet(id, loc).unwrap();

        let arch = session.arch.read().unwrap();
        let ids: Vec<&DropletId> = arch.droplets.keys().collect();

        assert_eq!(ids, vec![&id]);
    }

    #[test]
    fn lazy_mix_command() {
        let session = Session::new(Architecture::from_grid(Grid::rectangle(8, 8)));

        let id1 = session.input(Location { y: 1, x: 1 }).unwrap();
        let id2 = session.input(Location { y: 4, x: 4 }).unwrap();

        session.flush().unwrap();

        let _ = session.mix(id1, id2).unwrap();

        let arch = session.arch.read().unwrap();
        let ids: Vec<&DropletId> = arch.droplets.keys().collect();

        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&&id1));
        assert!(ids.contains(&&id2))
    }

    #[test]
    fn lazy_split_command() {
        let session = Session::new(Architecture::from_grid(Grid::rectangle(1, 1)));

        let id = session.input(Location { y: 1, x: 1 }).unwrap();

        session.flush().unwrap();

        let (_, _) = session.split(id).unwrap();

        let arch = session.arch.read().unwrap();
        let ids: Vec<&DropletId> = arch.droplets.keys().collect();
        assert_eq!(ids, vec![&id]);
    }
}
