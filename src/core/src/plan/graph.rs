use petgraph::prelude::*;

use std::collections::HashMap;

use util::find_duplicate;

use command::{BoxedCommand, CommandRequest};
use grid::{DropletId, Grid};

type NodeData = Option<BoxedCommand>;
type EdgeData = DropletId;

type Ix = u32;
pub type CmdIndex = NodeIndex<Ix>;

pub struct Graph {
    pub graph: StableDiGraph<NodeData, EdgeData, Ix>,
    pub droplet_idx: HashMap<DropletId, EdgeIndex<Ix>>,
    debug: bool,
}

#[derive(Debug)]
pub enum GraphError {
    AlreadyExists(DropletId),
    AlreadyBound(DropletId),
    DoesNotExist(DropletId),
    Duplicate(DropletId),
}

type GraphResult<T> = Result<T, GraphError>;

impl Graph {
    pub fn new() -> Graph {
        Graph {
            graph: StableDiGraph::new(),
            droplet_idx: HashMap::new(),
        }
    }

    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug
    }

    pub fn check_add_command(&self, cmd: &BoxedCommand) -> GraphResult<()> {
        // make sure there aren't duplicates in the inputs
        let in_droplets = cmd.input_droplets();
        if let Some((i, _)) = find_duplicate(&in_droplets) {
            return Err(GraphError::Duplicate(in_droplets[i]));
        }

        for id in &in_droplets {
            // make sure that the droplet id points to an edge
            let e_idx = self
                .droplet_idx
                .get(&id)
                .ok_or_else(|| GraphError::DoesNotExist(*id))?;

            // make sure that the edge exists
            let (_src, tgt) = self
                .graph
                .edge_endpoints(*e_idx)
                .ok_or_else(|| GraphError::DoesNotExist(*id))?;

            // tgt guaranteed to exist, we just looked it up
            // now check to make sure that edge is "unbound", i.e., that the
            // destination node data is None
            if let Some(_cmd) = &self.graph[tgt] {
                return Err(GraphError::AlreadyBound(*id));
            }
        }

        // make sure there are no duplicates in the output
        let out_droplets = cmd.output_droplets();
        if let Some((i, _)) = find_duplicate(&out_droplets) {
            return Err(GraphError::Duplicate(out_droplets[i]));
        }

        // validate that outgoing edges don't exist
        for id in out_droplets {
            if let Some(_e_idx) = self.droplet_idx.get(&id) {
                return Err(GraphError::AlreadyExists(id));
            }
        }

        Ok(())
    }

    pub fn add_command(&mut self, cmd: BoxedCommand) -> GraphResult<CmdIndex> {
        // check before mutating
        self.check_add_command(&cmd)?;

        // temporarily leave the cmd as "unbound" (None), so we can still use the command
        let cmd_id = self.graph.add_node(None);

        // insert the edges from the input commands, replacing the unbound edges and removing the unbound nodes
        for id in cmd.input_droplets() {
            let e_idx = self.droplet_idx.get_mut(&id).unwrap();
            let (src, tgt) = self.graph.edge_endpoints(*e_idx).unwrap();
            self.graph.remove_node(tgt).unwrap();
            *e_idx = self.graph.add_edge(src, cmd_id, id);
        }

        // insert the edges to the unbound output nodes, and update the droplet_idx map
        for id in cmd.output_droplets() {
            let unbound = self.graph.add_node(None);
            let e_idx = self.graph.add_edge(cmd_id, unbound, id);
            let was_there = self.droplet_idx.insert(id, e_idx);
            assert_eq!(was_there, None);
        }

        // now move the cmd into the graph
        self.graph[cmd_id] = Some(cmd);

        Ok(cmd_id)
    }

}

#[cfg(test)]
mod tests {

    // NOTE: graphs are automatically in debug mode in testing
    // so they will frequently validate themselves

    use super::*;

    // make sure we always check on drop
    impl Drop for Graph {
        fn drop(&mut self) {
            // don't risk panicking again if we are already panicking
            if !std::thread::panicking() {
                self.validate()
            }
        }
    }

    use command::{tests::Dummy, BoxedCommand};

    fn input(id: usize) -> BoxedCommand {
        Dummy::new(&[], &[id]).boxed()
    }

    fn mix(in_id1: usize, in_id2: usize, out_id: usize) -> BoxedCommand {
        Dummy::new(&[in_id1, in_id2], &[out_id]).boxed()
    }

    #[test]
    fn test_add_command_validate() {
        // we can test validation errors all in one go because they shouldn't modify the graph
        let mut graph = Graph::new();
        graph.add_command(input(0)).unwrap();
        graph.add_command(input(1)).unwrap();

        let r = graph.add_command(mix(0, 0, 2));
        assert_matches!(r, Err(GraphError::Duplicate(_)));

        let r = graph.add_command(input(0));
        assert_matches!(r, Err(GraphError::AlreadyExists(_)));

        let r = graph.add_command(mix(5, 6, 2));
        assert_matches!(r, Err(GraphError::DoesNotExist(_)));

        // now go ahead and do the ok mix
        let r = graph.add_command(mix(0, 1, 2));
        assert_matches!(r, Ok(_));

        let r = graph.add_command(mix(0, 1, 2));
        assert_matches!(r, Err(GraphError::AlreadyBound(_)));
    }

    fn simple_graph() -> (Graph, CmdIndex, CmdIndex, CmdIndex) {
        let mut graph = Graph::new();
        let in0 = graph.add_command(input(0)).unwrap();
        let in1 = graph.add_command(input(1)).unwrap();
        let mix = graph.add_command(mix(0, 1, 2)).unwrap();
        (graph, in0, in1, mix)
    }

    #[test]
    #[should_panic(expected = "Bad transition")]
    fn test_validate_bad_transitions() {
        let (mut graph, in0, _, mix) = simple_graph();
        graph.set_node_schedule(in0, 9);
        graph.set_node_schedule(mix, 1);
        // Can't have a schedules go backward
        graph.validate();
    }

    #[test]
    fn test_validate_okay_transitions() {
        let (mut graph, in0, _, _) = simple_graph();
        graph.set_node_schedule(in0, 1);
        graph.set_edge_schedule(0.into(), 2);
        graph.validate();
    }

    #[test]
    fn test_critical_path() {
        //
        //             /------------> short ------------\
        // input -> split                               mix -->
        //             \--> pass1 --> pass2 --> pass3 --/
        //

        let pass = |x, y| Dummy::new(&[x], &[y]).boxed();
        let split = |x, y1, y2| Dummy::new(&[x], &[y1, y2]).boxed();

        let mut graph = Graph::new();
        let input = graph.add_command(input(0)).unwrap();
        let split = graph.add_command(split(0, 1, 2)).unwrap();
        let pass1 = graph.add_command(pass(1, 10)).unwrap();
        let pass2 = graph.add_command(pass(10, 11)).unwrap();
        let pass3 = graph.add_command(pass(11, 12)).unwrap();
        let short = graph.add_command(pass(2, 20)).unwrap();
        let mix = graph.add_command(mix(20, 12, 3)).unwrap();

        let crit = graph.critical_paths();
        assert_eq!(crit[&mix], 1);
        assert_eq!(crit[&short], 2);
        assert_eq!(crit[&pass3], 2);
        assert_eq!(crit[&pass2], 3);
        assert_eq!(crit[&pass1], 4);
        assert_eq!(crit[&split], 5);
        assert_eq!(crit[&input], 6);
    }
}
