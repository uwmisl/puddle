
use arch::{DropletId, Architecture};
use command::*;

pub struct Session {
    pub arch: Architecture
}

#[derive(Debug)]
enum ExecutionError {
    RouteError,
    PlaceError,
}

impl Session {

    pub fn new_droplet(&mut self) -> DropletId {
        self.arch.new_droplet_id()
    }

    fn execute<Cmd: Command>(&mut self, cmd: &Cmd) -> Result<(), ExecutionError> {
        let mapping = match self.arch.grid.place(cmd.shape()) {
            None => return Err(ExecutionError::PlaceError),
            Some(m) => m,
        };

        let in_locs = cmd.input_locations();
        let in_ids = cmd.input_droplets();

        assert_eq!(in_locs.len(), in_ids.len());

        // set up destinations
        for (loc, id) in in_locs.iter().zip(in_ids) {
            // this should have been put to none last time
            let droplet = self.arch.droplets.get_mut(id).expect(
                "Command gave back and invalid DropletId"
            );
            assert!(droplet.destination.is_none());
            droplet.destination = Some(*loc);
        }

        let paths = match self.arch.route() {
            None => return Err(ExecutionError::RouteError),
            Some(p) => p,
        };

        self.arch.take_paths(paths);
        cmd.run(&mut self.arch, &mapping);

        // teardown destinations
        for id in in_ids {
            let droplet = self.arch.droplets.get_mut(id).unwrap();
            assert_eq!(Some(droplet.location), droplet.destination);
            droplet.destination = None;
        }

        Ok(())
    }

    pub fn mix(&mut self, d1: DropletId, d2: DropletId) -> DropletId {
        let mix_cmd = Mix::new(&mut self.arch, d1, d2);
        self.execute(&mix_cmd).expect("can't handle failures in api yet");
        mix_cmd.output_droplets()[0]
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use arch::{Location, Droplet};
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

        let mut session = Session {
            arch: arch
        };

        let id3 = session.mix(id1, id2);
        let ids: Vec<&DropletId> = session.arch.droplets.keys().collect();
        assert_eq!(ids, vec![&id3])
    }
}
