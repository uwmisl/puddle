use petgraph::{
    algo::toposort,
    prelude::*,
    visit::{IntoEdgeReferences, IntoNeighbors, Reversed},
};

use crate::grid::DropletId;
use crate::plan::graph::{CmdIndex, Graph};
use indexmap::IndexMap;

type Schedule = usize;

pub struct Scheduler {
    debug: bool,
    node_sched: IndexMap<CmdIndex, Schedule>,
    current_sched: usize,
}

#[derive(Debug)]
pub enum SchedError {
    NothingToSchedule,
}

pub struct SchedRequest<'a> {
    pub graph: &'a Graph,
    pub limit: Option<usize>,
}

#[derive(Debug)]
pub struct SchedResponse {
    pub commands_to_run: Vec<CmdIndex>,
    pub droplets_to_store: Vec<DropletId>,
}

type Result<T> = std::result::Result<T, SchedError>;

impl Default for Scheduler {
    fn default() -> Scheduler {
        Scheduler {
            debug: cfg!(test),
            node_sched: IndexMap::default(),
            current_sched: 0,
        }
    }
}

impl Scheduler {
    pub fn maybe_validate(&self, req: &SchedRequest) {
        if self.debug {
            self.validate(req);
        }
    }

    fn add_droplets_to_response(&self, req: &SchedRequest, resp: &mut SchedResponse) {
        let graph = &req.graph.graph;

        // assert some stuff about the schedule commands
        for cmd in &resp.commands_to_run {
            // make sure is in the graph
            assert!(graph.contains_node(*cmd));
            // make sure it's not already scheduled
            assert_eq!(self.node_sched.get(cmd), None);
        }

        // make sure we haven't put anything in here yet
        assert_eq!(resp.droplets_to_store, vec![]);

        for (cmd, &sched) in self.node_sched.iter() {
            assert!(sched < self.current_sched);

            for e in graph.edges(*cmd) {
                let cmd2 = e.target();

                if !(self.node_sched.contains_key(&cmd2) || resp.commands_to_run.contains(&cmd2)) {
                    let droplet_id = e.weight();
                    resp.droplets_to_store.push(*droplet_id);
                }
            }
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

        // make sure the scheduled nodes are subset of the graph nodes, and that
        // they are less than the current sched
        for (cmd, &sched) in self.node_sched.iter() {
            if !graph.contains_node(*cmd) {
                panic!("Graph doesn't contain node {:?}", cmd)
            }
            assert!(sched < self.current_sched)
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

    pub fn set_node_schedule(&mut self, cmd_id: CmdIndex, sched: Schedule) {
        let was_there = self.node_sched.insert(cmd_id, sched);
        assert_eq!(was_there, None);
    }

    fn is_ready(&self, req: &SchedRequest, cmd: CmdIndex) -> bool {
        let graph = &req.graph.graph;
        graph
            .neighbors_directed(cmd, Incoming)
            .all(|c| self.node_sched.contains_key(&c))
    }

    pub fn schedule(&self, req: &SchedRequest) -> Result<SchedResponse> {
        let criticality = critical_paths(&req.graph);
        let mut todos: Vec<_> = criticality
            .iter()
            // only consider nodes that we have not yet scheduled
            .filter(|&(node, _crit)| !self.node_sched.contains_key(node))
            // ignore nodes the "unbound" nodes
            .filter(|&(&node, _crit)| req.graph.graph[node].is_some() && self.is_ready(req, node))
            .collect();

        // we want to do the nodes first the reduce the number of droplets
        todos.sort_by_key(|&(&node, crit)| {
            let neg_crit = -(*crit as isize);
            let graph = &req.graph.graph;
            let in_degree = graph.edges_directed(node, Incoming).count() as i32;
            let out_degree = graph.edges_directed(node, Outgoing).count() as i32;
            (out_degree - in_degree, neg_crit)
        });

        if todos.is_empty() {
            return Err(SchedError::NothingToSchedule);
        }

        if let Some(limit) = req.limit {
            todos.truncate(limit);
        }

        let mut resp = SchedResponse {
            commands_to_run: todos.iter().map(|(n, _)| **n).collect(),
            droplets_to_store: vec![],
        };
        self.add_droplets_to_response(&req, &mut resp);
        Ok(resp)
    }

    pub fn commit(&mut self, resp: &SchedResponse) {
        for cmd_id in &resp.commands_to_run {
            let was_there = self.node_sched.insert(*cmd_id, self.current_sched);
            assert!(was_there.is_none());
        }
        self.current_sched += 1;
    }
}

fn critical_paths(graph: &Graph) -> IndexMap<CmdIndex, usize> {
    let mut distances = IndexMap::<CmdIndex, usize>::default();

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
    use crate::command::{tests::Dummy, BoxedCommand};

    fn input(id: usize) -> BoxedCommand {
        Dummy::new(&[], &[id]).boxed()
    }

    fn mix(in_id1: usize, in_id2: usize, out_id: usize) -> BoxedCommand {
        Dummy::new(&[in_id1, in_id2], &[out_id]).boxed()
    }

    fn simple_graph() -> (Graph, CmdIndex, CmdIndex, CmdIndex) {
        let mut graph = Graph::default();
        let in0 = graph.add_command(input(0)).unwrap();
        let in1 = graph.add_command(input(1)).unwrap();
        let mix = graph.add_command(mix(0, 1, 2)).unwrap();
        (graph, in0, in1, mix)
    }

    #[test]
    #[should_panic(expected = "Bad transition")]
    fn test_validate_bad_transitions() {
        let (graph, in0, _, mix) = simple_graph();
        let req = SchedRequest {
            graph: &graph,
            limit: None,
        };

        let mut sched = Scheduler::default();
        sched.current_sched = 100;

        sched.set_node_schedule(in0, 9);
        sched.set_node_schedule(mix, 1);
        // Can't have a schedules go backward
        sched.validate(&req);
    }

    #[test]
    fn test_validate_okay_transitions() {
        let (graph, in0, _, _) = simple_graph();
        let req = SchedRequest {
            graph: &graph,
            limit: None,
        };

        let mut sched = Scheduler::default();
        sched.current_sched = 100;

        sched.set_node_schedule(in0, 1);
        sched.validate(&req);
    }

    fn long_graph() -> (Graph, IndexMap<&'static str, CmdIndex>) {
        //
        //                 /-----------(2)---------> short ----------(20)--------\
        // input -(0)-> split                                                    mix --(3)-->
        //                 \--(1)--> pass1 --(10)--> pass2 --(11)--> pass3 -(12)-/
        //

        let pass = |x, y| Dummy::new(&[x], &[y]).boxed();
        let split = |x, y1, y2| Dummy::new(&[x], &[y1, y2]).boxed();

        let mut graph = Graph::default();
        let mut map = IndexMap::default();

        map.insert("input", graph.add_command(input(0)).unwrap());
        map.insert("split", graph.add_command(split(0, 1, 2)).unwrap());
        map.insert("pass1", graph.add_command(pass(1, 10)).unwrap());
        map.insert("pass2", graph.add_command(pass(10, 11)).unwrap());
        map.insert("pass3", graph.add_command(pass(11, 12)).unwrap());
        map.insert("short", graph.add_command(pass(2, 20)).unwrap());
        map.insert("mix", graph.add_command(mix(20, 12, 3)).unwrap());

        (graph, map)
    }

    #[test]
    fn test_critical_path() {
        let (graph, map) = long_graph();

        let crit = critical_paths(&graph);

        assert_eq!(crit[&map["mix"]], 1);
        assert_eq!(crit[&map["short"]], 2);
        assert_eq!(crit[&map["pass3"]], 2);
        assert_eq!(crit[&map["pass2"]], 3);
        assert_eq!(crit[&map["pass1"]], 4);
        assert_eq!(crit[&map["split"]], 5);
        assert_eq!(crit[&map["input"]], 6);
    }

    #[test]
    fn test_storing_droplets() {
        let (graph, map) = long_graph();

        let mut sched = Scheduler::default();
        sched.current_sched = 3;

        sched.set_node_schedule(map["input"], 0);
        sched.set_node_schedule(map["split"], 1);
        sched.set_node_schedule(map["short"], 2);
        sched.set_node_schedule(map["pass1"], 2);

        let req = SchedRequest {
            graph: &graph,
            limit: None,
        };
        let mut resp = SchedResponse {
            commands_to_run: vec![map["pass2"]],
            droplets_to_store: vec![],
        };

        sched.add_droplets_to_response(&req, &mut resp);

        assert_eq!(resp.droplets_to_store, &[20.into()]);
    }
}
