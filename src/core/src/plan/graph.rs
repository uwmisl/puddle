use petgraph::{
    algo::toposort,
    prelude::*,
    visit::{IntoEdgeReferences, IntoNeighbors, Reversed},
    // stable_graph::{EdgeIndex, NodeIndex, StableDiGraph},
};
use std::collections::HashMap;

use util::find_duplicate;

use command::{BoxedCommand, CommandRequest};
use grid::{DropletId, Grid};
use plan::place::Placement;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum State {
    Todo,
    Active,
    Done,
}

#[derive(Debug)]
struct NodeData {
    state: State,
    cmd: Option<BoxedCommand>,
}

struct EdgeData {
    state: State,
}

type Ix = u32;

pub type CmdIndex = NodeIndex<Ix>;

pub struct Graph {
    graph: StableDiGraph<NodeData, EdgeData, Ix>,
    droplet_idx: HashMap<DropletId, EdgeIndex<Ix>>,
    debug: bool,
}

#[derive(Debug)]
pub enum GraphError {
    AlreadyExists(DropletId),
    AlreadyBound(DropletId),
    DoesNotExist(DropletId),
    Duplicate(DropletId),
    Bad(String),
}

type GraphResult<T> = Result<T, GraphError>;

impl Graph {
    pub fn new() -> Graph {
        Graph {
            graph: StableDiGraph::new(),
            droplet_idx: HashMap::new(),
            debug: if cfg!(test) { true } else { false },
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
            if let Some(_cmd) = &self.graph[tgt].cmd {
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
        // validation and check before mutating
        self.maybe_validate();
        self.check_add_command(&cmd)?;

        // temporarily leave the cmd as "unbound" (None), so we can still use the command
        let cmd_id = self.graph.add_node(NodeData {
            state: State::Todo,
            cmd: None,
        });

        // insert the edges from the input commands, replacing the unbound edges and removing the unbound nodes
        for id in cmd.input_droplets() {
            let e_idx = self.droplet_idx.get_mut(&id).unwrap();
            let (src, tgt) = self.graph.edge_endpoints(*e_idx).unwrap();
            self.graph.remove_node(tgt).unwrap();
            let e_data = EdgeData { state: State::Todo };
            *e_idx = self.graph.add_edge(src, cmd_id, e_data);
        }

        // insert the edges to the unbound output nodes, and update the droplet_idx map
        for id in cmd.output_droplets() {
            let unbound = self.graph.add_node(NodeData {
                state: State::Todo,
                cmd: None,
            });

            let e_data = EdgeData { state: State::Todo };
            let e_idx = self.graph.add_edge(cmd_id, unbound, e_data);
            let was_there = self.droplet_idx.insert(id, e_idx);
            assert_eq!(was_there, None);
        }

        // now move the cmd into the graph
        self.graph[cmd_id].cmd = Some(cmd);

        self.maybe_validate();

        Ok(cmd_id)
    }

    pub fn toposort(&self) -> Vec<CmdIndex> {
        let working_space = None;
        toposort(&self.graph, working_space)
            .unwrap_or_else(|n| panic!("There was a cycle that included node {:?}", n))
    }

    pub fn validate(&self) {
        // make sure there are no cycles
        // toposort will err if there was one
        self.toposort();

        for e_ref in self.graph.edge_references() {
            let edge = e_ref.weight();
            let src = &self.graph[e_ref.source()];
            let tgt = &self.graph[e_ref.target()];

            // there are only 5 valid state transitions, just make sure each edge is one of them
            use self::State::*;
            match (&src.state, &edge.state, &tgt.state) {
                (Todo, Todo, Todo) => (),
                (Active, Todo, Todo) => (),
                (Done, Active, Todo) => (),
                (Done, Done, Active) => (),
                (Done, Done, Done) => (),
                (s, e, t) => panic!(
                    "Bad graph state: {:?} --- {:?} --> {:?}\n\
                     src = {:#?}\n\
                     tgt = {:#?}",
                    s, e, t, src, tgt
                ),
            }
        }

        for node_id in self.graph.node_indices() {
            let node = &self.graph[node_id];
            let in_degree = self.graph.edges_directed(node_id, Incoming).count();
            let out_degree = self.graph.edges_directed(node_id, Outgoing).count();

            // make sure there are no isolates
            if in_degree == 0 && out_degree == 0 {
                panic!("{:?} is isolated!: {:#?}", node_id, node);
            }

            // FIXME allow Output node to have 0 out degree
            // make sure that node has 0 out degree <=> it's an "unused" placeholder
            // also make sure placeholder are always Todo
            if node.cmd.is_none() {
                assert_eq!(node.state, State::Todo);
                assert_eq!(out_degree, 0);
            } else if out_degree == 0 {
                panic!("Node is real but has out degree 0! {:#?}", node)
            }
        }

        trace!("Graph validated!")
    }

    pub fn maybe_validate(&self) {
        if self.debug {
            self.validate();
        }
    }

    pub fn get_node_state(&self, cmd_id: CmdIndex) -> State {
        let node_data = &self.graph[cmd_id];
        node_data.state
    }

    pub fn set_node_state(&mut self, cmd_id: CmdIndex, state: State) {
        let node_data = &mut self.graph[cmd_id];
        node_data.state = state;
    }

    pub fn is_ready(&self, cmd_id: CmdIndex) -> bool {
        // an edge is ready if it is `todo` and the incoming edges are `active`,
        // i.e. they are being held on the board
        (self.get_node_state(cmd_id) == State::Todo) && self
            .graph
            .edges_directed(cmd_id, Direction::Incoming)
            .all(|e| e.weight().state == State::Active)
    }

    pub fn get_edge_state(&self, id: DropletId) -> State {
        let edge_id = self.droplet_idx[&id];
        let edge_data = self.graph.edge_weight(edge_id).unwrap();
        edge_data.state
    }

    pub fn set_edge_state(&mut self, id: DropletId, state: State) {
        let edge_id = self.droplet_idx[&id];
        let edge_data = self.graph.edge_weight_mut(edge_id).unwrap();
        edge_data.state = state;
    }

    pub fn critical_paths(&self) -> HashMap<CmdIndex, usize> {
        let mut distances = HashMap::<CmdIndex, usize>::new();

        // do a reverse toposort so we can count the critical path lengths
        let working_space = None;
        let rev = Reversed(&self.graph);
        let bottom_up = toposort(rev, working_space)
            .unwrap_or_else(|n| panic!("There was a cycle that included node {:?}", n));

        for n in bottom_up {
            let n_dist = *distances.entry(n).or_insert(0);
            for n2 in rev.neighbors(n) {
                // take the max of the existing distance and the new one
                distances
                    .entry(n2)
                    .and_modify(|d| *d = (*d).max(n_dist + 1))
                    .or_insert(n_dist + 1);
            }
        }

        distances
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
    #[should_panic(expected = "Bad graph state")]
    fn test_validate_bad_transitions() {
        let (mut graph, in0, _, _) = simple_graph();
        graph.set_node_state(in0, State::Done);
        // Can't have a Done node go into a Todo edge without an Active in between
        graph.validate();
    }

    #[test]
    fn test_validate_okay_transitions() {
        let (mut graph, in0, _, _) = simple_graph();
        graph.set_node_state(in0, State::Done);
        graph.set_edge_state(0.into(), State::Active);
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
