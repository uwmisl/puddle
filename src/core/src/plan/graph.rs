use petgraph::{
    algo::toposort,
    prelude::*,
    visit::IntoEdgeReferences,
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

pub struct PlanGraph {
    graph: StableDiGraph<NodeData, EdgeData, Ix>,
    droplet_idx: HashMap<DropletId, EdgeIndex<Ix>>,
    debug: bool,
}

#[derive(Debug)]
pub enum Error {
    AlreadyExists(DropletId),
    AlreadyBound(DropletId),
    DoesNotExist(DropletId),
    Duplicate(DropletId),
    Bad(String),
}

type GraphResult<T> = Result<T, Error>;

// TODO none of this should be owned
struct PlacementRequest {
    cmd_to_place: HashMap<CmdIndex, CommandRequest>,
    fixed: Vec<Placement>,
    storage: HashMap<DropletId, Grid>,
}

struct PlacementError {}

// TODO make the actual placer implement this
trait Placer {
    fn place(&self, req: PlacementRequest) -> Result<Placement, PlacementError>;
}

impl PlanGraph {
    pub fn new() -> PlanGraph {
        PlanGraph {
            graph: StableDiGraph::new(),
            droplet_idx: HashMap::new(),
            debug: if cfg!(test) { true } else { false },
        }
    }

    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug
    }

    pub fn check_add_command(&self, cmd: &BoxedCommand) -> GraphResult<()> {
        // validate that incoming edges (droplets) already exist and aren't bound
        let in_droplets = cmd.input_droplets();
        if let Some((i, _)) = find_duplicate(&in_droplets) {
            return Err(Error::Duplicate(in_droplets[i]));
        }

        for id in &in_droplets {
            // make sure that the droplet id points to an edge
            let e_idx = self
                .droplet_idx
                .get(&id)
                .ok_or_else(|| Error::DoesNotExist(*id))?;

            // make sure that the edge exists
            let (_src, tgt) = self
                .graph
                .edge_endpoints(*e_idx)
                .ok_or_else(|| Error::DoesNotExist(*id))?;

            // tgt guaranteed to exist, we just looked it up
            // now check to make sure that edge is "unbound", i.e., that the
            // destination node data is None
            if let Some(_cmd) = &self.graph[tgt].cmd {
                return Err(Error::AlreadyBound(*id));
            }
        }

        // validate that outgoing edges don't exist
        let out_droplets = cmd.output_droplets();
        if let Some((i, _)) = find_duplicate(&out_droplets) {
            return Err(Error::Duplicate(out_droplets[i]));
        }

        for id in out_droplets {
            if let Some(_e_idx) = self.droplet_idx.get(&id) {
                return Err(Error::AlreadyExists(id));
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

    pub fn validate(&self) {
        // make sure there are no cycles
        // toposort will err if there was one
        assert!(toposort(&self.graph, None).is_ok());

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

    fn maybe_validate(&self) {
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
}

#[cfg(test)]
mod tests {

    // NOTE: graphs are automatically in debug mode in testing
    // so they will frequently validate themselves

    use super::*;

    // make sure we always check on drop
    impl Drop for PlanGraph {
        fn drop(&mut self) {
            // don't risk panicking again if we are already panicking
            if !std::thread::panicking() {
                self.validate()
            }
        }
    }

    use command::{Combine, Input};
    use grid::Location;

    fn droplet_id(id: usize) -> DropletId {
        DropletId { id, process_id: 0 }
    }

    fn input(id: usize) -> BoxedCommand {
        let substance = "water".into();
        let volume = 1.0;
        let dimensions = Location { y: 1, x: 1 };
        let out_id = droplet_id(id);
        let cmd = Input::new(substance, volume, dimensions, out_id).unwrap();
        Box::new(cmd)
    }

    fn mix(in_id1: usize, in_id2: usize, out_id: usize) -> BoxedCommand {
        let in_id1 = droplet_id(in_id1);
        let in_id2 = droplet_id(in_id2);
        let out_id = droplet_id(out_id);
        let cmd = Combine::new(in_id1, in_id2, out_id).unwrap();
        Box::new(cmd)
    }

    #[test]
    fn test_add_command_validate() {
        // we can test validation errors all in one go because they shouldn't modify the graph
        let mut graph = PlanGraph::new();
        graph.add_command(input(0)).unwrap();
        graph.add_command(input(1)).unwrap();

        let r = graph.add_command(mix(0, 0, 2));
        assert_matches!(r, Err(Error::Duplicate(_)));

        let r = graph.add_command(input(0));
        assert_matches!(r, Err(Error::AlreadyExists(_)));

        let r = graph.add_command(mix(5, 6, 2));
        assert_matches!(r, Err(Error::DoesNotExist(_)));

        // now go ahead and do the ok mix
        let r = graph.add_command(mix(0, 1, 2));
        assert_matches!(r, Ok(_));

        let r = graph.add_command(mix(0, 1, 2));
        assert_matches!(r, Err(Error::AlreadyBound(_)));
    }

    fn simple_graph() -> (PlanGraph, CmdIndex, CmdIndex, CmdIndex) {
        let mut graph = PlanGraph::new();
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
        graph.set_edge_state(droplet_id(0), State::Active);
        graph.validate();
    }
}
