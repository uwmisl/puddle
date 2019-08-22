use std::rc::Rc;

use crate::grid::{grid::NEIGHBORS_5, Droplet, DropletId, Grid, GridView, Location, Rectangle};
use indexmap::IndexMap;

pub type Path = Vec<Location>;

pub struct RoutingRequest<'a> {
    pub gridview: &'a GridView,
    pub agents: Vec<Agent>,
    pub blockages: Vec<Grid>,
}

#[derive(Debug, Clone)]
pub struct RoutingResponse {
    pub routes: IndexMap<DropletId, Path>,
}

#[derive(Debug)]
pub enum RoutingError {
    NoRoute { agents: Vec<Agent> },
}

#[derive(Default)]
pub struct Router {}

impl Router {
    pub fn route(&mut self, req: &RoutingRequest) -> Result<RoutingResponse, RoutingError> {
        debug!("Routing agents: {:#?}", req.agents);

        let mut ctx = Context::from_request(req);
        match ctx.route() {
            Some(paths) => Ok(RoutingResponse {
                routes: paths.into_iter().collect(),
            }),
            None => {
                warn!("Failed to route agents: {:#?}", req.agents);
                Err(RoutingError::NoRoute {
                    agents: req.agents.clone(),
                })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Agent {
    pub id: DropletId,
    pub source: Location,
    pub destination: Location,
    pub dimensions: Location,
}

impl Agent {
    pub fn from_droplet(d: &Droplet, destination: Location) -> Agent {
        Agent {
            id: d.id,
            source: d.location,
            dimensions: d.dimensions,
            destination,
        }
    }

    fn rectangle(&self, loc: Location) -> Rectangle {
        Rectangle::new(loc, self.dimensions)
    }
}

#[derive(Debug)]
struct Group {
    agents: Vec<Agent>,
}

impl Group {
    fn singleton(agent: Agent) -> Group {
        Group {
            agents: vec![agent],
        }
    }

    fn start(&self) -> Node {
        Node {
            locations: self.agents.iter().map(|a| a.source).collect(),
            time: 0,
        }
    }

    fn merge(&self, other: &Group) -> Group {
        let mut agents = self.agents.clone();
        agents.extend(other.agents.clone());
        Group { agents }
    }
}

type EdgeCost = u32;
const STAY_COST: EdgeCost = 4;
const MOVE_COST: EdgeCost = 5;
const COLLISION_COST: EdgeCost = 50;

fn step_cost(loc: Location) -> EdgeCost {
    let sit_still = Location { y: 0, x: 0 };
    if loc == sit_still {
        STAY_COST
    } else {
        MOVE_COST
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Node {
    locations: Vec<Location>,
    time: u32,
}

impl Node {
    fn with_group<'a>(
        &'a self,
        group: &'a Group,
    ) -> impl Clone + Iterator<Item = (&'a Location, &'a Agent)> {
        self.locations.iter().zip(&group.agents)
    }

    fn heuristic(&self, group: &Group) -> u32 {
        let n_steps: u32 = self
            .with_group(group)
            .map(|(l, a)| l.distance_to(a.destination))
            .sum();
        MOVE_COST * n_steps
    }

    fn is_done(&self, group: &Group) -> bool {
        self.with_group(group)
            .all(|(loc, agent)| loc == &agent.destination)
    }

    fn is_valid(&self, ctx: &Context, group: &Group) -> bool {
        // make sure all the agents are in the grid
        for (&loc, agent) in self.with_group(group) {
            let rect = agent.rectangle(loc);
            for rloc in rect.locations() {
                if ctx.grid.get_cell(rloc).is_none() {
                    return false;
                }
            }
        }

        let mut iter = self.with_group(group);
        while let Some((&loc1, a1)) = iter.next() {
            let r1 = a1.rectangle(loc1);
            for (&loc2, a2) in iter.clone() {
                let r2 = a2.rectangle(loc2);
                let dist = r1.collision_distance(&r2);
                // collision distance is the number of spaces between, so
                // anything above 0 is good
                if dist <= 0 {
                    return false;
                }
            }
        }

        true
    }

    fn take_action(
        &self,
        ctx: &Context,
        group: &Group,
        offsets: &[Location],
    ) -> Option<(EdgeCost, Node)> {
        assert_eq!(self.locations.len(), offsets.len());

        let new_locs: Vec<_> = self
            .locations
            .iter()
            .zip(offsets)
            .map(|(&agent, &offset)| agent + offset)
            .collect();

        let edge_cost = offsets.iter().cloned().map(step_cost).sum();

        let node = Node {
            locations: new_locs,
            time: self.time + 1,
        };

        if node.is_valid(ctx, group) {
            Some((edge_cost, node))
        } else {
            None
        }
    }

    // This is rather naive for now, it pretty much always generates
    // exponentially many new agents
    fn open(&self, ctx: &Context, group: &Group, new_nodes: &mut Vec<(EdgeCost, Node)>) {
        let nbrs = NEIGHBORS_5;
        let mut assignments = vec![0; self.locations.len()];
        let mut new_locations = Vec::with_capacity(nbrs.len());

        'outer: loop {
            // commit this assignment
            new_locations.clear();
            new_locations.extend(assignments.iter().map(|a| nbrs[*a]));

            if let Some(agent) = self.take_action(ctx, group, &new_locations) {
                new_nodes.push(agent)
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
            assert_eq!(assignments, vec![0; self.locations.len()]);
            break;
        }
    }
}

fn path_nth(path: &[Location], i: usize) -> Location {
    *path.get(i).unwrap_or_else(|| path.last().unwrap())
}

#[derive(Debug)]
struct Collision {
    id1: DropletId,
    id2: DropletId,
    time: usize,
}

// borrows from request
struct Context<'req> {
    grid: &'req Grid,
    agents: IndexMap<DropletId, Agent>,
    groups: IndexMap<DropletId, Rc<Group>>,
}

type PathMap = IndexMap<DropletId, Vec<Location>>;

impl Context<'_> {
    fn from_request<'a>(req: &'a RoutingRequest<'a>) -> Context<'a> {
        let agents = || req.agents.iter().cloned();

        Context {
            grid: &req.gridview.grid,
            // TODO we can make agents ourselves instead of the request doing it
            agents: agents().map(|a| (a.id, a)).collect(),
            // each group is a singleton node for now,
            groups: agents()
                .map(|a| (a.id, Rc::new(Group::singleton(a))))
                .collect(),
        }
    }

    fn find_collisions(&self, paths: &PathMap) -> Vec<Collision> {
        let mut collisions = Vec::new();

        let max_length = paths.values().map(Vec::len).max().unwrap();

        for time in 0..max_length {
            let mut iter = paths.iter();

            while let Some((&id1, p1)) = iter.next() {
                let a1 = &self.agents[&id1];
                let p1 = p1.as_ref();
                let loc1 = path_nth(p1, time);
                let rect1 = Rectangle::new(loc1, a1.dimensions);

                if cfg!(debug_assertions) {
                    for loc in rect1.clone().locations() {
                        assert!(self.grid.get_cell(loc).is_some())
                    }
                }

                for (&id2, p2) in iter.clone() {
                    let a2 = &self.agents[&id2];
                    let p2 = p2.as_ref();
                    let loc2 = path_nth(p2, time);
                    let rect2 = Rectangle::new(loc2, a2.dimensions);
                    if rect1.collision_distance(&rect2) <= 0 {
                        let c = Collision { id1, id2, time };
                        collisions.push(c)
                    }
                }
            }
        }

        collisions
    }

    fn find_collisions_with(
        &self,
        paths: &PathMap,
        group: &Group,
        node: &Node,
    ) -> Option<DropletId> {
        for (id, path) in paths {
            let location = path_nth(path, node.time as usize);
            let dimensions = self.agents[id].dimensions;
            let path_rect = Rectangle {
                location,
                dimensions,
            };
            for (a, &location) in group.agents.iter().zip(node.locations.iter()) {
                assert_ne!(*id, a.id);
                let dimensions = a.dimensions;
                let rect = Rectangle {
                    location,
                    dimensions,
                };
                if rect.collision_distance(&path_rect) <= 0 {
                    return Some(*id);
                }
            }
        }
        None
    }

    fn merge_groups(&mut self, id1: &DropletId, id2: &DropletId) -> Rc<Group> {
        let group1 = &self.groups[id1];
        let group2 = &self.groups[id2];
        let new_group = Rc::new(group1.merge(&group2));
        for a in &new_group.agents {
            self.groups.insert(a.id, Rc::clone(&new_group));
        }
        new_group
    }

    fn route(&mut self) -> Option<PathMap> {
        // route everyone independently
        let mut paths = PathMap::default();
        // we assume that groups, agents are non-empty, so just return if there's nothing to plan
        if self.groups.is_empty() {
            return Some(paths);
        }

        let mut group_costs = Vec::new();
        for group in self.groups.values() {
            let (group_paths, cost) = self.route_group(group, &paths)?;
            group_costs.push((Rc::clone(&group), cost));
            for (id, path) in group_paths {
                let was_there = paths.insert(id, path);
                assert_eq!(was_there, None);
            }
        }

        loop {
            const MAX_GROUP_SIZE: usize = 4;

            // if we there are no collisions, we're good!
            let collisions = self.find_collisions(&paths);
            if collisions.is_empty() {
                break;
            }

            // After a routing failure, before trying to merge groups, sort the
            // agents by cost of the (failed) routes. The logic here is that the most
            // expensive routes probably included some collisions (due to the
            // penalty), and we can probably avoid having to merge by
            // routing those problematic routes first and then letting
            // the "simpler" ones route around it
            debug!("Collision, trying sorted...");
            group_costs.sort_by_key(|&(_, c)| -(c as isize));
            paths.clear();
            for (g, c) in &mut group_costs {
                let (new_paths, cost) = self.route_group(g, &paths)?;
                paths.extend(new_paths);
                *c = cost
            }

            // check again!
            let collisions = self.find_collisions(&paths);
            if collisions.is_empty() {
                debug!("Routing worked after sorting");
                break;
            }

            // for now we only use the first collision
            let coll = &collisions[0];
            debug!("Collision, merging groups: {:?}", coll);
            let old_group1 = Rc::clone(&self.groups[&coll.id1]);
            let old_group2 = Rc::clone(&self.groups[&coll.id2]);
            let new_group = self.merge_groups(&coll.id1, &coll.id2);
            if new_group.agents.len() > MAX_GROUP_SIZE {
                return None;
            }

            let old_len = group_costs.len();
            group_costs
                .retain(|(g, _)| !(Rc::ptr_eq(g, &old_group1) || Rc::ptr_eq(g, &old_group2)));
            assert_eq!(old_len, group_costs.len() + 2);

            for a in &new_group.agents {
                paths.remove(&a.id);
            }
            let (new_paths, cost) = self.route_group(&new_group, &paths)?;
            group_costs.push((new_group, cost));
            paths.extend(new_paths);
        }

        Some(paths)
    }

    fn route_group(&self, group: &Group, paths: &PathMap) -> Option<(PathMap, EdgeCost)> {
        debug!(
            "Routing ids: {:?}",
            group.agents.iter().map(|a| a.id).collect::<Vec<_>>()
        );

        #[cfg(not(target_arch = "wasm32"))]
        let start_time = std::time::Instant::now();
        let start = group.start();

        let successors = |n: &Node| {
            let mut buf = Vec::new();
            n.open(self, group, &mut buf);
            buf.into_iter().map(|(c, n)| (n, c))
        };

        let heuristic = |n: &Node| {
            let mut h = n.heuristic(group);
            if self.find_collisions_with(paths, group, n).is_some() {
                h += COLLISION_COST;
            }
            h
        };

        let max_length = paths.values().map(Vec::len).max().unwrap_or(0) as u32;
        let limit = 20_000 * group.agents.len();
        let mut seen = 0;
        let success = |n: &Node| {
            seen += 1;
            // if we've hit the limit, we're done
            // otherwise, make sure we route until max_length, so
            // collision avoidance actually works
            seen == limit || (n.time >= max_length && n.is_done(group))
        };

        let result = pathfinding::directed::astar::astar(&start, successors, heuristic, success);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let duration = start_time.elapsed();
            debug!(
                "Routing g={} {status} {}.{:06} sec. Saw {:7} nodes.",
                group.agents.len(),
                duration.as_secs(),
                duration.subsec_micros(),
                seen,
                status = if seen < limit && result.is_some() {
                    "passed"
                } else {
                    "failed"
                },
            );
        }

        if seen == limit {
            return None;
        }

        result.map(|(path, cost)| {
            debug!("Solution has cost {}.", cost);
            let mut map: PathMap = group
                .agents
                .iter()
                .map(|a| a.id)
                .zip(std::iter::repeat_with(Vec::new))
                .collect();
            for step in path {
                for (a, loc) in group.agents.iter().zip(step.locations) {
                    map.get_mut(&a.id).unwrap().push(loc)
                }
            }
            (map, cost)
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::grid::gridview::tests::{c2id, id2c, parse_gridview};
    use indexmap::IndexSet;

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
        let ids_start: IndexSet<_> = gv_start.droplets.keys().collect();
        let ids_end: IndexSet<_> = gv_end.droplets.keys().collect();

        assert_eq!(gv_start.grid, gv_end.grid);
        assert_eq!(ids_start, ids_end);

        let agents = ids_start
            .iter()
            .map(|&id| {
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

    type ExpectedPaths = IndexMap<char, &'static [&'static str]>;

    fn check_paths(gv: &GridView, paths: &PathMap, expected_paths: &ExpectedPaths) {
        for (&ch, &expected) in expected_paths.iter() {
            let id = c2id(ch);
            let actual = draw_path(&paths[&id], 'a', &gv);
            if actual != expected {
                panic!(
                    "Route check failed\nExpected: {:#?}\nActual: {:#?}",
                    expected, actual
                )
            }
        }
    }

    fn check_groups(ctx: &Context, expected_groups: &[&str]) {
        let mut actual: Vec<String> = ctx
            .groups
            .iter()
            .filter(|(&id, g)| id == g.agents[0].id)
            .map(|(_, g)| {
                let mut chars: Vec<_> = g.agents.iter().map(|a| id2c(&a.id)).collect();
                chars.sort();
                chars.iter().collect()
            })
            .collect();
        actual.sort();

        let mut expected: Vec<String> = expected_groups
            .iter()
            .map(|s| {
                let mut chars: Vec<_> = s.chars().collect();
                chars.sort();
                chars.iter().collect()
            })
            .collect();
        expected.sort();

        assert_eq!(actual, expected)
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

        let mut expected = ExpectedPaths::default();
        #[rustfmt::skip]
        expected.insert('a', &[
            "A..",
            "a .",
            "aaa",
        ]);

        let req = &mk_route_request(&gv0, &gv1);
        let mut ctx = Context::from_request(req);
        let paths = ctx.route().unwrap();

        check_paths(&gv0, &paths, &expected);
    }

    #[test]
    fn test_impossible_route_fail() {
        let gv0 = parse_gridview(&["a.. ..."]);
        let gv1 = parse_gridview(&["... ..a"]);

        let req = &mk_route_request(&gv0, &gv1);
        let mut ctx = Context::from_request(req);
        assert_eq!(ctx.route(), None)
    }

    #[test]
    fn test_big_droplet_route_fail() {
        let gv0 = parse_gridview(&[
            "aa..........................",
            "aa..............     .......",
        ]);
        let gv1 = parse_gridview(&[
            ".........................aa.",
            "................     ....aa.",
        ]);

        let req = &mk_route_request(&gv0, &gv1);
        let mut ctx = Context::from_request(req);
        assert_eq!(ctx.route(), None)
    }

    #[test]
    fn test_slack_cooperative_route() {
        #[rustfmt::skip]
        let gv0 = parse_gridview(&[
            "  b  ",
            "  .  ",
            "a....",
            "  .  ",
            "  .  ",
        ]);
        #[rustfmt::skip]
        let gv1 = parse_gridview(&[
            "  .  ",
            "  .  ",
            "....a",
            "  .  ",
            "  b  ",
        ]);

        let req = &mk_route_request(&gv0, &gv1);
        let mut ctx = Context::from_request(req);
        let paths = ctx.route().unwrap();

        println!("{:#?}", paths);
        println!("{:#?}", ctx.groups);

        // This shouldn't require grouped routing, because the slack
        // system should avoid the collision

        check_groups(&ctx, &["a", "b"]);
    }

    #[test]
    fn test_easy_cooperative_route() {
        #[rustfmt::skip]
        let gv0 = parse_gridview(&[
            "a...b",
            "  .  ",
            "  .  ",
        ]);
        #[rustfmt::skip]
        let gv1 = parse_gridview(&[
            "b...a",
            "  .  ",
            "  .  ",
        ]);

        let req = &mk_route_request(&gv0, &gv1);
        let mut ctx = Context::from_request(req);
        let paths = ctx.route().unwrap();

        // let node0 = Node::singleton(req.agents[0].clone());
        // let node1 = Node::singleton(req.agents[1].clone());
        // let node = node0.merge(&node1);

        // let route = ctx.route_one(&node, expected_time).unwrap();
        println!("{:#?}", paths);
        println!("{:#?}", ctx.groups);

        check_groups(&ctx, &["ab"]);

        // let max_length = route.iter().map(|p| p.len()).max().unwrap();
        // assert_eq!(max_length as u32 - 1, expected_time);
    }

    #[test]
    fn test_split_cooperative_route() {
        #[rustfmt::skip]
        let gv0 = parse_gridview(&[
            "a...b c...d",
            "  .     .  ",
            "  .     .  ",
        ]);
        #[rustfmt::skip]
        let gv1 = parse_gridview(&[
            "b...a d...c",
            "  .     .  ",
            "  .     .  ",
        ]);

        let req = &mk_route_request(&gv0, &gv1);
        let mut ctx = Context::from_request(req);
        let paths = ctx.route().unwrap();

        println!("{:#?}", paths);
        println!("{:#?}", ctx.groups);

        check_groups(&ctx, &["ab", "cd"]);
    }

    #[test]
    #[ignore = "can only be run with release profile"]
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

        let req = &mk_route_request(&gv0, &gv1);
        let mut ctx = Context::from_request(req);
        let paths = ctx.route().unwrap();

        println!("{:#?}", paths);
        println!("{:#?}", ctx.groups);

        check_groups(&ctx, &["abcd"]);
    }

    #[test]
    fn test_grid_cooperative_route() {
        #[rustfmt::skip]
        let gv0 = parse_gridview(&[
            "a.b.c...",
            "........",
            "d.e.f...",
            "........",
            "........",
        ]);
        #[rustfmt::skip]
        let gv1 = parse_gridview(&[
            "..b.c.a.",
            "........",
            "d.e.f...",
            "........",
            "........",
        ]);

        let req = &mk_route_request(&gv0, &gv1);
        let mut ctx = Context::from_request(req);
        let paths = ctx.route().unwrap();

        println!("{:#?}", paths);
        println!("{:#?}", ctx.groups);

        // This shouldn't require grouped routing, because the slack
        // system should avoid the collision

        // cooperative routing is needed so that b can get out of the
        // way in time, but the rest should work because of the cost
        // sorting
        check_groups(&ctx, &["ab", "c", "d", "e", "f"]);
    }
}
