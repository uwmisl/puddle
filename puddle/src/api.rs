
use std::collections::HashMap;
use arch::{DropletId, Architecture, Location};
use command::*;

pub struct Session {
    pub arch: Architecture,
    commands: Vec<Box<Command>>
}

#[derive(Debug)]
enum ExecutionError {
    RouteError,
    PlaceError,
}

impl Session {

    pub fn new(arch: Architecture) -> Session {
        Session {
            arch: arch,
            commands: Vec::new()
        }
    }

    fn register<Cmd: Command>(&mut self, cmd: Cmd)
        where Cmd: 'static {
        self.commands.push(Box::new(cmd));
    }

    fn execute(&mut self, cmd: Box<Command>) -> Result<(), ExecutionError> {

        cmd.pre_run(&mut self.arch);

        let mut mapping : HashMap<Location, Location>;

        if cmd.trust_placement() {
            mapping = HashMap::new();
            for loc in cmd.input_locations() {
                mapping.insert(*loc, *loc);
            }
        } else {
            mapping = match self.arch.grid.place(cmd.shape()) {
                None => return Err(ExecutionError::PlaceError),
                Some(m) => m,
            }
        }

        // let mapping = match self.arch.grid.place(cmd.shape()) {
        //     None => return Err(ExecutionError::PlaceError),
        //     Some(m) => m,
        // };

        println!("Mapping: {:?}", mapping);

        let in_locs = cmd.input_locations();
        let in_ids = cmd.input_droplets();

        assert_eq!(in_locs.len(), in_ids.len());

        // TODO: this needs to go over all droplets...
        for (loc, id) in in_locs.iter().zip(in_ids) {
            // this should have been put to none last time
            let droplet = self.arch.droplets.get_mut(id).expect(
                "Command gave back and invalid DropletId"
            );
            assert!(droplet.destination.is_none());
            let mapped_loc = mapping.get(loc)
                .expect("input location wasn't in mapping");
            droplet.destination = Some(*mapped_loc);
        }
        let paths = match self.arch.route() {
            None => return Err(ExecutionError::RouteError),
            Some(p) => p,
        };

        self.arch.take_paths(paths);
        cmd.run(&mut self.arch, &mapping);

        // teardown destinations if the droplets are still there
        // TODO is this ever going to be true?
        for id in in_ids {
            self.arch.droplets.get_mut(id).map(
                |droplet| {
                    assert_eq!(Some(droplet.location), droplet.destination);
                    droplet.destination = None;
                }
            );
        }

        Ok(())
    }

    pub fn flush(&mut self) {
        self.commands.reverse();
        while !self.commands.is_empty() {
            let cmd : Box<Command> = self.commands.pop().unwrap();
            self.execute(cmd).expect("can't hanlde api failures yet");
        }
    }

    pub fn input(&mut self, loc: Location) -> DropletId {
        let input_cmd = Input::new(&mut self.arch, loc);
        let out = input_cmd.output_droplets()[0];
        self.register(input_cmd);
        out
    }

    pub fn move_droplet(&mut self, d1: DropletId, loc: Location) -> DropletId {
        let move_cmd = Move::new(&mut self.arch, d1, loc);
        let out = move_cmd.output_droplets()[0];
        self.register(move_cmd);
        out
    }

    pub fn mix(&mut self, d1: DropletId, d2: DropletId) -> DropletId {
        let mix_cmd = Mix::new(&mut self.arch, d1, d2);
        let out = mix_cmd.output_droplets()[0];
        self.register(mix_cmd);
        out
    }

    pub fn split(&mut self, d1: DropletId) -> (DropletId, DropletId) {
        let split_cmd = Split::new(&mut self.arch, d1);
        let out1 = split_cmd.output_droplets()[0];
        let out2 = split_cmd.output_droplets()[1];
        self.register(split_cmd);
        (out1, out2)
    }
}


#[cfg(test)]
mod tests {

    use super::*;

    use arch::{Location, Droplet};
    use arch::grid::Grid;

    //
    //  Non-Lazy Tests
    //

    #[test]
    fn execute_input_command() {
        let mut arch = Architecture::from_grid(
            Grid::rectangle(1,1)
        );

        let mut session = Session::new(arch);

        // todo: this shouldn't work
        let loc = Location {y: 3, x: 3};
        let id = session.input(loc);
        session.flush();

        let ids: Vec<&DropletId> = session.arch.droplets.keys().collect();
        assert_eq!(ids, vec![&id]);

        let dr = session.arch.droplets.get(&id).unwrap();
        assert_eq!(loc, dr.location)
    }

    #[test]
    fn execute_move_command() {
        // let mut arch = Architecture::from_grid(
        //     Grid::rectangle(2,2)
        // );

        // let mut session = Session::new(arch);

        // let loc = Location {y: 1, x: 1};

        // let id = session.input(loc);
        // let id1 = session.move_droplet(id, loc);
        // session.flush();

        // let ids: Vec<&DropletId> = session.arch.droplets.keys().collect();

        // assert_eq!(ids, vec![&id1]);

        // let dr = session.arch.droplets.get(&id1).unwrap();
        // assert_eq!(loc, dr.location)
    }

    #[test]
    fn execute_mix_command() {
        let mut arch = Architecture::from_grid(
            Grid::rectangle(4,3)
        );

        let mut session = Session::new(arch);

        let id1 = session.input(Location {x: 2, y: 3});
        let id2 = session.input(Location {x: 1, y: 1});
        let id3 = session.mix(id1, id2);

        session.flush();

        let ids: Vec<&DropletId> = session.arch.droplets.keys().collect();

        assert_eq!(ids, vec![&id3])
    }

    #[test]
    fn execute_split_command() {
        let mut arch = Architecture::from_grid(
            Grid::rectangle(1,5)
        );

        let mut session = Session::new(arch);

        let id = session.input(Location {x: 2, y: 0});
        let (id1, id2) = session.split(id);

        session.flush();

        let ids : Vec<&DropletId> = session.arch.droplets.keys().collect();

        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&&id1));
        assert!(ids.contains(&&id2))
    }

    #[test]
    fn execute_all_commands() {
        let mut arch = Architecture::from_grid(
            Grid::rectangle(10,10)
        );

        let mut session = Session::new(arch);
        // todo: add move into here.
        let id1 = session.input(Location{y: 2, x: 2});
        let id2 = session.input(Location{y: 0, x: 0});
        let id3 = session.mix(id1, id2);
        let (id4, id5) = session.split(id3);
        let id6 = session.input(Location{y: 7, x: 7});
        let id7 = session.mix(id4, id6);

        session.flush();

        let ids : Vec<&DropletId> = session.arch.droplets.keys().collect();

        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&&id5));
        assert!(ids.contains(&&id7))
    }

    //
    //  Laziness Tests
    //

    #[test]
    fn lazy_input_command() {
        let mut arch = Architecture::from_grid(
            Grid::rectangle(1,1)
        );

        let mut session = Session::new(arch);

        // todo: this shouldn't work
        let loc = Location {y: 3, x: 3};
        let id = session.input(loc);

        let ids: Vec<&DropletId> = session.arch.droplets.keys().collect();
        assert_eq!(ids.len(), 0);
    }

    #[test]
    fn lazy_move_command() {
        // let mut arch = Architecture::from_grid(
        //     Grid::rectangle(1,1)
        // );

        // let mut session = Session::new(arch);

        // // todo: this shouldn't work
        // let loc = Location {y: 3, x: 3};
        // let id = session.input(loc);
        // session.flush();

        // let ids: Vec<&DropletId> = session.arch.droplets.keys().collect();
        // assert_eq!(ids, vec![&id]);

        // let dr = session.arch.droplets.get(&id).unwrap();
        // assert_eq!(loc, dr.location)
    }

    #[test]
    fn lazy_mix_command() {
        let mut arch = Architecture::from_grid(
            Grid::rectangle(8,8)
        );

        let mut session = Session::new(arch);

        // todo: this shouldn't work
        let id1 = session.input(Location {y: 1, x: 1});
        let id2 = session.input(Location {y: 4, x: 4});

        session.flush();

        let id3 = session.mix(id1, id2);

        let ids : Vec<&DropletId> = session.arch.droplets.keys().collect();

        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&&id1));
        assert!(ids.contains(&&id2))
    }

    #[test]
    fn lazy_split_command() {
        let mut arch = Architecture::from_grid(
            Grid::rectangle(1,1)
        );

        let mut session = Session::new(arch);

        // todo: this shouldn't work
        let id = session.input(Location {y: 1, x: 1});
        session.flush();
        let (id1, id2) = session.split(id);

        let ids: Vec<&DropletId> = session.arch.droplets.keys().collect();
        assert_eq!(ids, vec![&id]);
    }
}
