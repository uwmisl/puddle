use std::time::Instant;

use hashbrown::{
    hash_map::{Entry, HashMap},
    HashSet,
};

use grid::{grid::NEIGHBORS_5, Droplet, DropletId, Grid, GridView, Location, Rectangle};
use util::minheap::MinHeap;

pub type Path = Vec<Location>;

pub struct RoutingRequest<'a> {
    pub gridview: &'a GridView,
    pub agents: Vec<Agent>,
    pub blockages: Vec<Grid>,
}

// TODO this is the beginning of the router interface
pub struct Router {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Agent {
    pub id: DropletId,
    pub location: Location,
    pub destination: Location,
    pub dimensions: Location,
}

impl Agent {
    fn from_droplet(d: &Droplet, destination: Location) -> Agent {
        Agent {
            id: d.id,
            location: d.location,
            dimensions: d.dimensions,
            destination,
        }
    }

    fn heuristic(&self) -> u32 {
        self.location.distance_to(&self.destination)
    }

    fn step(&self, offset: &Location) -> Agent {
        Agent {
            location: &self.location + offset,
            ..self.clone()
        }
    }

    fn rectangle(&self) -> Rectangle {
        Rectangle::new(self.location, self.dimensions)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Node {
    // TODO try using smallvecs here
    agents: Vec<Agent>,
    time: u32,
}

// TODO use conflict count as second param
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Cost {
    // total cost (f) = so far cost (g) + "to go" cost (h)
    total_estimated_cost: u32,
    cost_to_go: u32,
    cost_so_far: u32,
}

type EdgeCost = u32;
const STAY_COST: EdgeCost = 1;
const MOVE_COST: EdgeCost = 2;

fn step_cost(loc: &Location) -> EdgeCost {
    let sit_still = Location { y: 0, x: 0 };
    if loc == &sit_still {
        STAY_COST
    } else {
        MOVE_COST
    }
}

impl Node {
    fn singleton(agent: Agent) -> Node {
        Node {
            agents: vec![agent],
            time: 0,
        }
    }

    fn merge(&self, other: &Node) -> Node {
        // merge should only be used in search set up,
        // therefore the time should be zero
        assert_eq!(self.time, 0);
        assert_eq!(other.time, 0);
        let mut agents = self.agents.clone();
        agents.extend(other.agents.clone());
        Node { agents, time: 0 }
    }

    fn new_cost(&self, cost_so_far: u32) -> Cost {
        let h = self.heuristic();
        Cost {
            total_estimated_cost: cost_so_far + h,
            cost_to_go: h,
            cost_so_far,
        }
    }

    fn heuristic(&self) -> u32 {
        let n_steps: u32 = self.agents.iter().map(|d| d.heuristic()).sum();
        MOVE_COST * n_steps
    }

    fn is_done(&self) -> bool {
        self.agents.iter().all(|a| a.location == a.destination)
    }

    fn is_valid(&self, ctx: &RoutingContext) -> bool {
        // make sure all the agents are in the grid
        for a in &self.agents {
            for loc in a.rectangle().locations() {
                if ctx.gridview.grid.get_cell(&loc).is_none() {
                    return false;
                }
            }
        }

        let mut iter = self.agents.iter();
        while let Some(a1) = iter.next() {
            for a2 in iter.clone() {
                let dist = a1.rectangle().collision_distance(&a2.rectangle());
                // collision distance is the number of spaces between, so
                // anything above 0 is good
                if dist <= 0 {
                    return false;
                }
            }
        }

        true
    }

    fn take_action(&self, ctx: &RoutingContext, offsets: &[Location]) -> Option<(EdgeCost, Node)> {
        assert_eq!(self.agents.len(), offsets.len());

        let new_agents: Vec<_> = self
            .agents
            .iter()
            .zip(offsets)
            .map(|(agent, offset)| agent.step(offset))
            .collect();

        let edge_cost = offsets.iter().map(step_cost).sum();

        let node = Node {
            agents: new_agents,
            time: self.time + 1,
        };

        if node.is_valid(ctx) {
            Some((edge_cost, node))
        } else {
            None
        }
    }

    // This is rather naive for now, it pretty much always generates
    // exponentially many new agents
    fn open(&self, ctx: &mut RoutingContext, new_agents: &mut Vec<(EdgeCost, Node)>) {
        let nbrs = NEIGHBORS_5;
        let mut assignments = vec![0; self.agents.len()];
        let mut new_locations = Vec::with_capacity(nbrs.len());

        'outer: loop {
            // commit this assignment
            new_locations.clear();
            new_locations.extend(assignments.iter().map(|a| nbrs[*a]));

            if let Some(agent) = self.take_action(ctx, &new_locations) {
                new_agents.push(agent)
            }

            // advance the assignments by basically doing carry addition
            for a in assignments.iter_mut() {
                if *a + 1 < nbrs.len() {
                    // don't have to carry, addition is complete
                    *a += 1;
                    continue 'outer;
                } else {
                    *a = 0;
                }
            }

            // if we got here, we carried off the edges, so just stop
            assert_eq!(assignments, vec![0; self.agents.len()]);
            break;
        }
    }
}

struct RoutingContext<'a> {
    gridview: &'a GridView,
    // node_timestamp: usize,
}

impl<'a> RoutingContext<'a> {
    // fn next_timestamp(&mut self) -> usize {
    //     let t = self.node_timestamp;
    //     self.node_timestamp += 1;
    //     t
    // }
}

fn path_nth(path: &[Location], i: usize) -> Location {
    *path.get(i).unwrap_or_else(|| path.last().unwrap())
}

type PathMap = HashMap<Agent, Vec<Location>>;

fn find_collisions(paths: &PathMap) -> Vec<(&Agent, &Agent, usize)> {
    let mut collisions = Vec::new();

    let max_length = paths.values().map(|p| p.len()).max().unwrap();

    for i in 0..max_length {
        let mut iter = paths.iter();
        while let Some((a1, p1)) = iter.next() {
            let p1 = p1.as_ref();
            let loc1 = path_nth(p1, i);
            let rect1 = Rectangle::new(loc1, a1.dimensions);
            for (a2, p2) in iter.clone() {
                let p2 = p2.as_ref();
                let loc2 = path_nth(p2, i);
                let rect2 = Rectangle::new(loc2, a2.dimensions);
                if rect1.collision_distance(&rect2) <= 0 {
                    collisions.push((a1, a2, i))
                }
            }
        }
    }

    collisions
}

impl<'a> RoutingContext<'a> {
    // FIXME this is not deterministic!
    fn route_one(&mut self, initial: &Node, max_time: u32) -> Option<Vec<Path>> {
        let start_time = Instant::now();

        let mut todo: MinHeap<Cost, Node> = MinHeap::new();
        let mut best_so_far: HashMap<Node, u32> = HashMap::new();
        let mut came_from: HashMap<Node, Node> = HashMap::new();
        // TODO remove done in favor of came_from
        let mut done: HashSet<Node> = HashSet::new();

        let mut n_explored = 0;
        let mut next_nodes = Vec::new();

        todo.push(initial.new_cost(0), initial.clone());
        best_so_far.insert(initial.clone(), 0);

        let result = loop {
            let (popped_cost, node) = match todo.pop() {
                Some(cn) => cn,
                _ => break None,
            };

            // node must be in best_so_far because it was inserted when we put
            // it in the minheap
            let node_cost_so_far: u32 = best_so_far[&node];

            trace!(
                "Popped: time={} best_cost_so_far={} h={} cost={:?} agents:{}",
                node.time,
                node_cost_so_far,
                node.heuristic(),
                popped_cost,
                {
                    // print each agent on a new line
                    let mut s = String::new();
                    for a in &node.agents {
                        s.extend(format!("\n  {:?}", a).chars())
                    }
                    s
                }
            );

            n_explored += 1;

            if node.is_done() {
                let path = build_path(came_from, node.clone());
                break Some(path);
            }

            // insert returns false if value was already there
            if !done.insert(node.clone()) || node.time >= max_time {
                continue;
            }

            assert_eq!(next_nodes.len(), 0);
            node.open(self, &mut next_nodes);

            for (edge_cost, next) in next_nodes.drain(..) {
                debug_assert!(next.is_valid(self));

                if done.contains(&next) {
                    continue;
                }

                trace!(
                    "  Pushed: time={} edge_cost={} h={}",
                    next.time,
                    edge_cost,
                    next.heuristic(),
                );

                let mut next_cost = node_cost_so_far + edge_cost;

                match best_so_far.entry(next.clone()) {
                    Entry::Occupied(entry) => {
                        let old_cost = *entry.get();
                        if next_cost < old_cost {
                            *entry.into_mut() = next_cost;
                            came_from.insert(next.clone(), node.clone());
                        } else {
                            next_cost = old_cost;
                        }
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(next_cost);
                        came_from.insert(next.clone(), node.clone());
                    }
                };

                let new_cost = next.new_cost(next_cost);
                todo.push(new_cost, next)
            }
        };

        // trace!(
        //     "Routing droplet {id} from {src} to {dst}",
        //     id = droplet.id.id,
        //     src = droplet.location,
        //     dst = destination
        // );
        let duration = start_time.elapsed();
        debug!(
            "Routing took {}.{:06} sec. Nodes: {} + {} = {} (explored + unseen = total)",
            duration.as_secs(),
            duration.subsec_micros(),
            n_explored,
            todo.len(),
            todo.timestamp(),
        );

        result
    }
}

fn build_path(mut came_from: HashMap<Node, Node>, end_node: Node) -> Vec<Path> {
    let mut paths: Vec<Path> = vec![vec![]; end_node.agents.len()];
    let mut current = end_node;

    while let Some(prev) = came_from.remove(&current) {
        for (p, a) in paths.iter_mut().zip(current.agents) {
            p.push(a.location)
        }
        current = prev;
    }

    for (p, a) in paths.iter_mut().zip(current.agents) {
        p.push(a.location);
        p.reverse();
    }

    paths
}

#[cfg(test)]
mod tests {

    use super::*;
    use grid::gridview::tests::{c2id, parse_gridview};

    fn draw_path(path: &[Location], ch: char, gridview: &GridView) -> Vec<String> {
        let strs = gridview.grid.to_strs();
        let replace_char = |y, x, grid_char| {
            let loc = Location { y, x };
            if path.contains(&loc) {
                assert_eq!(grid_char, '.');
                if loc == path[0] {
                    ch.to_ascii_uppercase()
                } else {
                    ch
                }
            } else {
                grid_char
            }
        };

        strs.iter()
            .enumerate()
            .map(|(y, row)| {
                row.char_indices()
                    .map(|(x, grid_char)| replace_char(y as i32, x as i32, grid_char))
                    .collect()
            })
            .collect()
    }

    fn mk_route_request<'a>(gv_start: &'a GridView, gv_end: &GridView) -> RoutingRequest<'a> {
        let ids_start: HashSet<_> = gv_start.droplets.keys().collect();
        let ids_end: HashSet<_> = gv_end.droplets.keys().collect();

        assert_eq!(gv_start.grid, gv_end.grid);
        assert_eq!(ids_start, ids_end);

        let agents = ids_start
            .iter()
            .map(|id| {
                let d0 = &gv_start.droplets[id];
                let d1 = &gv_end.droplets[id];
                Agent::from_droplet(d0, d1.location)
            })
            .collect();

        // TODO parse blockages
        let blockages = Vec::new();

        RoutingRequest {
            agents,
            blockages,
            gridview: &gv_start,
        }
    }

    type ExpectedRoutes = HashMap<char, &'static [&'static str]>;

    fn test_routes(
        gv_start: &GridView,
        gv_end: &GridView,
        expected_time: u32,
        expected_routes: &ExpectedRoutes,
    ) {
        let req = mk_route_request(&gv_start, &gv_end);

        let mut ctx = RoutingContext {
            gridview: &gv_start,
        };

        let node = Node::singleton(req.agents[0].clone());

        let route = ctx.route_one(&node, expected_time).unwrap();
        println!("{:#?}", route);

        let agent_paths: PathMap = req.agents.iter().cloned().zip(route.clone()).collect();
        let collisions = find_collisions(&agent_paths);
        assert_eq!(collisions, vec![]);

        let actual = draw_path(&route[0], 'a', &gv_start);
        let expected = expected_routes[&'a'];

        let max_length = route.iter().map(|p| p.len()).max().unwrap();
        assert_eq!(max_length as u32 - 1, expected_time);

        if actual != expected {
            panic!(
                "Route check failed\nExpected: {:#?}\nActual: {:#?}",
                expected, actual
            )
        }
    }

    #[test]
    fn test_simple_route() {
        #[rustfmt::skip]
        let gv0 = parse_gridview(&[
            "a..",
            ". .",
            "...",
        ]);

        #[rustfmt::skip]
        let gv1 = parse_gridview(&[
            "...",
            ". .",
            "..a",
        ]);

        let mut expected = ExpectedRoutes::new();
        #[rustfmt::skip]
        expected.insert('a', &[
            "A..",
            "a .",
            "aaa",
        ]);

        test_routes(&gv0, &gv1, 4, &expected);
    }

    #[test]
    #[should_panic]
    fn test_impossible_route_fail() {
        let gv0 = parse_gridview(&["a.. ..."]);
        let gv1 = parse_gridview(&["... ..a"]);
        let mut expected = ExpectedRoutes::new();
        expected.insert('a', &[""]);
        test_routes(&gv0, &gv1, 100, &expected);
    }

    #[test]
    #[should_panic]
    fn test_big_droplet_route_fail() {
        let gv0 = parse_gridview(&[
            "aa..........................",
            "aa..............     .......",
        ]);
        let gv1 = parse_gridview(&[
            ".........................aa.",
            "................     ....aa.",
        ]);
        let mut expected = ExpectedRoutes::new();
        expected.insert('a', &[""]);
        test_routes(&gv0, &gv1, 100, &expected);
    }

    #[test]
    fn test_easy_cooperative_route() {
        #[rustfmt::skip]
        let gv0 = parse_gridview(&[
            "a...b",
            "  .  ",
            "  .  "]
        );
        #[rustfmt::skip]
        let gv1 = parse_gridview(&[
            "b...a",
            "  .  ",
            "  .  "]
        );

        let req = mk_route_request(&gv0, &gv1);

        let mut ctx = RoutingContext { gridview: &gv0 };

        let node0 = Node::singleton(req.agents[0].clone());
        let node1 = Node::singleton(req.agents[1].clone());
        let node = node0.merge(&node1);

        let expected_time = 10;
        let route = ctx.route_one(&node, expected_time).unwrap();
        println!("{:#?}", route);

        let max_length = route.iter().map(|p| p.len()).max().unwrap();
        assert_eq!(max_length as u32 - 1, expected_time);
    }

    #[test]
    #[ignore("can only be run with release profile")]
    fn test_hard_cooperative_route() {
        #[rustfmt::skip]
        let gv0 = parse_gridview(&[
            "a.....b",
            "    .  ",
            "c.....d"]
        );
        #[rustfmt::skip]
        let gv1 = parse_gridview(&[
            "d.....c",
            "    .  ",
            "b.....a"]
        );

        let req = mk_route_request(&gv0, &gv1);

        let mut ctx = RoutingContext { gridview: &gv0 };

        let node0 = Node::singleton(req.agents[0].clone());
        let node1 = Node::singleton(req.agents[1].clone());
        let node2 = Node::singleton(req.agents[2].clone());
        let node3 = Node::singleton(req.agents[3].clone());
        let node = node0.merge(&node1).merge(&node2).merge(&node3);

        let expected_time = 21;
        let route = ctx.route_one(&node, expected_time).unwrap();
        println!("{:#?}", route);

        let max_length = route.iter().map(|p| p.len()).max().unwrap();
        assert_eq!(max_length as u32 - 1, expected_time);
    }
}
