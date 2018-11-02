use std::collections::HashMap;

use petgraph::{
    algo::toposort,
    prelude::*,
    visit::{IntoEdgeReferences, IntoNeighbors, Reversed},
};

use grid::DropletId;
use plan::graph::{CmdIndex, Graph};

type Schedule = usize;

pub struct Scheduler {
    debug: bool,
    node_sched: HashMap<CmdIndex, Schedule>,
}

#[derive(Debug)]
pub enum SchedError {
    NothingToSchedule,
}

pub struct SchedRequest<'a> {
    graph: &'a Graph,
}

pub struct SchedResponse {
    commands_to_run: Vec<CmdIndex>,
    droplets_to_store: Vec<DropletId>,
}

type Result<T> = std::result::Result<T, SchedError>;

impl Scheduler {
    pub fn new() -> Scheduler {
        Scheduler {
            debug: if cfg!(test) { true } else { false },
            node_sched: HashMap::new(),
        }
    }

    pub fn maybe_validate(&self, req: &SchedRequest) {
        if self.debug {
            self.validate(req);
        }
    }

    pub fn validate(&self, req: &SchedRequest) {
        let graph = &req.graph.graph;

        // make sure there are no cycles
        // toposort will err if there was one
        let working_space = None;
        toposort(graph, working_space)
            .unwrap_or_else(|n| panic!("There was a cycle that included node {:?}", n));

        for e_ref in graph.edge_references() {
            let src_sched = self.node_sched.get(&e_ref.source());
            let tgt_sched = self.node_sched.get(&e_ref.target());

            match (src_sched, tgt_sched) {
                (None, Some(_)) => panic!("Bad transition: unscheduled before scheduled!"),
                (Some(i), Some(j)) if (i >= j) => panic!("Bad transition: {} goes into {}!", i, j),
                _ => (),
            }
        }

        // make sure the scheduled nodes are subset of the graph nodes
        for node_id in self.node_sched.keys() {
            if !graph.contains_node(*node_id) {
                panic!("Graph doesn't contain node {:?}", node_id)
            }
        }

        for node_id in graph.node_indices() {
            let node = &graph[node_id];
            let in_degree = graph.edges_directed(node_id, Incoming).count();
            let out_degree = graph.edges_directed(node_id, Outgoing).count();

            // make sure there are no isolates
            if in_degree == 0 && out_degree == 0 {
                panic!("{:?} is isolated!: {:#?}", node_id, node);
            }

            // FIXME allow Output node to have 0 out degree
            // make sure that node has 0 out degree <=> it's an "unused" placeholder
            // also make sure placeholder are always Todo
            if node.is_none() {
                assert_eq!(self.node_sched.get(&node_id), None);
                assert_eq!(out_degree, 0);
            } else if out_degree == 0 {
                panic!("Node is real but has out degree 0! {:#?}", node)
            }
        }

        trace!("Graph validated!")
    }

    fn set_node_schedule(&mut self, cmd_id: CmdIndex, sched: Schedule) {
        let was_there = self.node_sched.insert(cmd_id, sched);
        assert_eq!(was_there, None);
    }

    // pub fn schedule(&mut self, req: &SchedRequest) -> Result<SchedResponse> {
    //     let criticality = req.graph.critical_paths();
    //     let (&most_critical_todo, _max_criticality) = criticality
    //         .iter()
    //         .filter(|&(node, _crit)| req.graph.get_node_state(*node) == State::Todo)
    //         .max_by_key(|&(_node, crit)| crit)
    //         .ok_or(SchedError::NothingToSchedule)?;

    //     // the most critical node must be ready, otherwise something above it
    //     // (more critical) would also be `todo`
    //     assert!(req.graph.is_ready(most_critical_todo));

    //     unimplemented!()
    // }
}

fn critical_paths(graph: &Graph) -> HashMap<CmdIndex, usize> {
    let mut distances = HashMap::<CmdIndex, usize>::new();

    // do a reverse toposort so we can count the critical path lengths
    let working_space = None;
    let rev = Reversed(&graph.graph);
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

#[cfg(test)]
mod tests {

    use super::*;
    use command::{tests::Dummy, BoxedCommand};

    fn input(id: usize) -> BoxedCommand {
        Dummy::new(&[], &[id]).boxed()
    }

    fn mix(in_id1: usize, in_id2: usize, out_id: usize) -> BoxedCommand {
        Dummy::new(&[in_id1, in_id2], &[out_id]).boxed()
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
        let (graph, in0, _, mix) = simple_graph();
        let req = SchedRequest { graph: &graph };
        let mut sched = Scheduler::new();

        sched.set_node_schedule(in0, 9);
        sched.set_node_schedule(mix, 1);
        // Can't have a schedules go backward
        sched.validate(&req);
    }

    #[test]
    fn test_validate_okay_transitions() {
        let (graph, in0, _, _) = simple_graph();
        let req = SchedRequest { graph: &graph };
        let mut sched = Scheduler::new();

        sched.set_node_schedule(in0, 1);
        sched.validate(&req);
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

        let crit = critical_paths(&graph);

        assert_eq!(crit[&mix], 1);
        assert_eq!(crit[&short], 2);
        assert_eq!(crit[&pass3], 2);
        assert_eq!(crit[&pass2], 3);
        assert_eq!(crit[&pass1], 4);
        assert_eq!(crit[&split], 5);
        assert_eq!(crit[&input], 6);
    }
}
