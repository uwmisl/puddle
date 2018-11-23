use std::collections::HashSet;
use std::time::Instant;

use grid::{Droplet, DropletId, Grid, GridView, Location};

use util::collections::Entry::*;
use util::collections::{Map, Set};
use util::minheap::MinHeap;
use util::mk_rng;

use rand::Rng;

pub type Path = Vec<Location>;

fn build_path(mut came_from: Map<Node, Node>, end_node: Node) -> Path {
    let mut path = Vec::new();
    let mut current = end_node;
    while let Some(prev) = came_from.remove(&current) {
        path.push(current.location);
        current = prev;
    }
    path.push(current.location);
    path.reverse();
    path
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
struct Node {
    collision_group: usize,
    location: Location,
    dimensions: Location,
    time: Time,
}

#[derive(Debug)]
struct SuperNode {
    collision_groups: Set<usize>,
    location: Location,
    time: Time,
}

type Time = u32;
type Cost = u32;
const MOVE_COST: Cost = 100;
const STAY_COST: Cost = 1;

#[derive(Default)]
struct AvoidanceSet {
    max_time: Time,
    present: Map<(Location, Time), SuperNode>,
    finals: Map<Location, SuperNode>,
}

#[derive(PartialEq)]
enum Collision {
    SameGroup,
    DifferentGroup,
}

impl AvoidanceSet {
    fn should_avoid(&self, node: &Node) -> bool {
        self.collides(&node).is_some() || self.collides_with_final(&node)
    }

    fn collides(&self, node: &Node) -> Option<Collision> {
        // if not present, no collision
        use self::Collision::*;
        let mut collision = None;

        for y in 0..node.dimensions.y {
            for x in 0..node.dimensions.x {
                let loc = &node.location + &Location { y, x };
                if let Some(sn) = self.present.get(&(loc, node.time)) {
                    if sn.collision_groups.contains(&node.collision_group)
                        && sn.collision_groups.len() == 1
                    {
                        collision = Some(SameGroup);
                    } else {
                        return Some(DifferentGroup);
                    }
                };
            }
        }

        collision
    }

    fn collides_with_final(&self, node: &Node) -> bool {
        for y in 0..node.dimensions.y {
            for x in 0..node.dimensions.x {
                let loc = &node.location + &Location { y, x };
                let collides = self
                    .finals
                    .get(&loc)
                    .filter(|sn| {
                        sn.collision_groups
                            .iter()
                            .any(|&cg| cg != node.collision_group)
                    }).map_or(false, |fin| node.time >= fin.time);
                if collides {
                    return true;
                }
            }
        }

        false
    }

    fn would_finally_collide(&self, node: &Node) -> bool {
        (node.time..self.max_time)
            .map(|t| Node { time: t, ..*node })
            .any(|future_node| self.collides(&future_node) == Some(Collision::DifferentGroup))
    }

    // clippy will complain about &Vec (because of &Path)
    #[cfg_attr(feature = "cargo-clippy", allow(ptr_arg))]
    fn avoid_path(&mut self, path: &Path, grid: &Grid, droplet: &Droplet) {
        let node_path = path.clone().into_iter().enumerate().map(|(i, loc)| Node {
            time: i as Time,
            collision_group: droplet.collision_group,
            location: loc,
            dimensions: droplet.dimensions,
        });
        for node in node_path {
            self.avoid_node(grid, node);
        }

        // Add last element to finals
        let last = path.len() - 1;
        for loc in grid.neighbors_dimensions(&path[last], &droplet.dimensions) {
            self.finals
                .entry(loc)
                .and_modify(|sn| {
                    sn.collision_groups.insert(droplet.collision_group);
                    sn.time = sn.time.min(last as Time)
                }).or_insert_with(|| {
                    let mut cgs = Set::new();
                    cgs.insert(droplet.collision_group);
                    SuperNode {
                        collision_groups: cgs,
                        location: loc,
                        time: last as Time,
                    }
                });
        }

        self.max_time = self.max_time.max(last as Time)
    }

    fn avoid_node(&mut self, grid: &Grid, node: Node) {
        for loc in grid.neighbors_dimensions(&node.location, &node.dimensions) {
            for t in -1..2 {
                let time = (node.time as i32) + t;
                if time < 0 {
                    continue;
                }
                self.present
                    .entry((loc, time as Time))
                    .and_modify(|sn| {
                        sn.collision_groups.insert(node.collision_group);
                    }).or_insert_with(|| {
                        let mut cgs = Set::new();
                        cgs.insert(node.collision_group);
                        SuperNode {
                            collision_groups: cgs,
                            location: loc,
                            time: time as Time,
                        }
                    });
            }
        }
    }
}

impl Node {
    /// Returns a vector representing possible locations on the given `Grid` that can be the next
    /// location for this `Node`. This uses `neighbors4`, since droplets only move in the cardinal
    /// directions.
    fn expand(&self, grid: &Grid) -> Vec<(Cost, Node)> {
        let mut vec: Vec<(Cost, Node)> = grid
            .neighbors4(&self.location)
            .iter()
            .map(|&location| {
                (
                    MOVE_COST,
                    Node {
                        location,
                        collision_group: self.collision_group,
                        time: self.time + 1,
                        dimensions: self.dimensions,
                    },
                )
            }).collect();

        vec.push((
            STAY_COST,
            Node {
                location: self.location,
                collision_group: self.collision_group,
                time: self.time + 1,
                dimensions: self.dimensions,
            },
        ));

        vec
    }

    fn stay(&self) -> Vec<(Cost, Node)> {
        vec![(
            STAY_COST,
            Node {
                location: self.location,
                collision_group: self.collision_group,
                time: self.time + 1,
                dimensions: self.dimensions,
            },
        )]
    }
}

// TODO this is the beginning of the router interface
pub struct Router {}

#[derive(Clone)]
pub struct DropletRouteRequest {
    pub id: DropletId,
    pub destination: Location,
}

pub struct RoutingRequest<'a> {
    pub gridview: &'a GridView,
    pub droplets: Vec<DropletRouteRequest>,
    pub blockages: Vec<Grid>,
}

impl std::fmt::Debug for DropletRouteRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, " {:?} -> {:?} ", self.id, self.destination)
    }
}

#[derive(Debug)]
pub struct RoutingResponse {
    pub routes: Map<DropletId, Path>,
}

#[derive(Debug)]
pub enum RoutingError {
    NoRoute,
}

impl Router {
    pub fn new() -> Router {
        Router {}
    }

    pub fn route(&mut self, req: &RoutingRequest) -> Result<RoutingResponse, RoutingError> {
        let mut droplets = req.droplets.clone();
        let mut rng = mk_rng();
        // TODO: we should get rid of bad edges eventually
        let bad_edges = Set::new();
        for i in 1..20 {
            rng.shuffle(&mut droplets);
            let result = route_many(&droplets, req.gridview, &bad_edges);
            if let Some(paths) = result {
                return Ok(RoutingResponse { routes: paths });
            }
            trace!("route failed, trying iteration {}", i);
        }

        Err(RoutingError::NoRoute)
    }
}

fn route_many(
    droplet_destinations: &[DropletRouteRequest],
    gridview: &GridView,
    bad_edges: &Set<(Location, Location)>,
) -> Option<Map<DropletId, Path>> {
    let mut av_set = AvoidanceSet::default();
    let num_cells = gridview.grid.locations().count();

    let mut paths = Map::new();
    let mut max_t = 0;

    debug!("Routing droplets in this order: {:?}", droplet_destinations);

    for req in droplet_destinations.iter() {
        let id = req.id;
        let droplet = &gridview.droplets[&id];
        // route a single droplet

        trace!(
            "Avoidance set before droplet {:#?}: {:#?}",
            droplet,
            av_set.finals
        );
        let result = {
            let max_time = num_cells as Time + max_t;

            let next_fn = |node: &Node| {
                let nodes = if droplet.pinned {
                    node.stay()
                } else {
                    node.expand(&gridview.grid)
                };
                nodes
                    .iter()
                    .filter(|(_cost, n)| {
                        let l1 = node.location;
                        let l2 = n.location;
                        !av_set.should_avoid(n) && !bad_edges.contains(&(l1, l2))
                    }).cloned()
                    .collect::<Vec<_>>()
            };

            let done_fn = |node: &Node| {
                node.location == req.destination && !av_set.would_finally_collide(node)
            };

            route_one(&droplet, req.destination, max_time, next_fn, done_fn)
        };
        let path = match result {
            None => return None,
            Some(path) => path,
        };

        max_t = max_t.max(path.len() as Time);

        // once we know this path works, add to our avoidance set
        av_set.avoid_path(&path, &gridview.grid, &droplet);
        paths.insert(id, path);
    }

    Some(paths)
}

fn route_one<FNext, FDone>(
    droplet: &Droplet,
    destination: Location,
    max_time: Time,
    mut next_fn: FNext,
    mut done_fn: FDone,
) -> Option<Path>
where
    FNext: FnMut(&Node) -> Vec<(Cost, Node)>,
    FDone: FnMut(&Node) -> bool,
{
    let start_time = Instant::now();

    let mut todo: MinHeap<Cost, Node> = MinHeap::new();
    let mut best_so_far: Map<Node, Cost> = Map::new();
    let mut came_from: Map<Node, Node> = Map::new();
    // TODO remove done in favor of came_from
    let mut done: HashSet<Node> = HashSet::new();
    let mut n_explored = 0;

    let start_node = Node {
        location: droplet.location,
        collision_group: droplet.collision_group,
        dimensions: droplet.dimensions,
        time: 0,
    };
    todo.push(0, start_node);
    best_so_far.insert(start_node, 0);

    // use manhattan distance from goal as the heuristic
    let heuristic = |node: Node| -> Cost { destination.distance_to(&node.location) * MOVE_COST };

    let result = loop {
        let node = match todo.pop() {
            Some((_, node)) => node,
            _ => {
                trace!("Routing failed!");
                break None;
            }
        };

        n_explored += 1;

        if done_fn(&node) {
            let path = build_path(came_from, node);
            break Some(path);
        }

        // insert returns false if value was already there
        if !done.insert(node) || node.time > max_time {
            continue;
        }

        // node must be in best_so_far because it was inserted when we put it in
        // the minheap
        let node_cost: Cost = best_so_far[&node];

        for (edge_cost, next) in next_fn(&node) {
            if done.contains(&next) {
                continue;
            }

            let mut next_cost = node_cost + edge_cost;

            match best_so_far.entry(next) {
                Occupied(entry) => {
                    let old_cost = *entry.get();
                    if next_cost < old_cost {
                        *entry.into_mut() = next_cost;
                        came_from.insert(next, node);
                    } else {
                        next_cost = old_cost;
                    }
                }
                Vacant(entry) => {
                    entry.insert(next_cost);
                    came_from.insert(next, node);
                }
            };

            let next_cost_est = next_cost + heuristic(next);
            todo.push(next_cost_est, next)
        }
    };

    trace!(
        "Routing droplet {id} from {src} to {dst}",
        id = droplet.id.id,
        src = droplet.location,
        dst = destination
    );
    let duration = start_time.elapsed();
    trace!(
        "I saw {} nodes in {}.{:03} sec",
        n_explored,
        duration.as_secs(),
        duration.subsec_nanos() / 1_000_000
    );

    result
}

#[cfg(test)]
mod tests {

    // use super::*;
    // use grid::gridview::tests::{c2id, parse_gridview};

    // TODO put back the tests

    // fn path(locs: &[(i32, i32)]) -> Path {
    //     locs.iter().map(|&(y, x)| Location { y, x }).collect()
    // }

    // fn get_droplet(gv: &mut GridView, ch: char) -> &mut Droplet {
    //     gv.snapshot_mut().droplets.get_mut(&c2id(ch)).unwrap()
    // }

    // #[test]
    // fn test_collide_at_end() {
    //     #[cfg_attr(rustfmt, rustfmt_skip)]
    //     let mut gv = parse_gridview(&[
    //         "a...b",
    //         "  .  ",
    //         "  .  "
    //     ]);

    //     let dest = Location { y: 2, x: 2 };
    //     get_droplet(&mut gv, 'a').destination = Some(dest);
    //     get_droplet(&mut gv, 'b').destination = Some(dest);

    //     // this should fail because the droplets aren't allow to collide
    //     assert!(gv.route().is_none());

    //     get_droplet(&mut gv, 'a').collision_group = 42;
    //     get_droplet(&mut gv, 'b').collision_group = 42;

    //     // this should work, as the droplets are allowed to collide now
    //     // but, we check to make sure that they collide at the end of the path
    //     let paths = gv.route().unwrap();

    //     assert_eq!(
    //         paths[&c2id('a')],
    //         path(&[(0, 0), (0, 1), (0, 2), (1, 2), (2, 2),])
    //     );

    //     assert_eq!(
    //         paths[&c2id('b')],
    //         path(&[
    //             (0, 4),
    //             (0, 4),
    //             (0, 4),
    //             (0, 4),
    //             (0, 4),
    //             (0, 3),
    //             (0, 2),
    //             (1, 2),
    //             (2, 2)
    //         ])
    //     );
    // }

    // #[test]
    // fn test_pinned() {
    //     #[cfg_attr(rustfmt, rustfmt_skip)]
    //     let mut gv = parse_gridview(&[
    //         "....b",
    //         "  a  ",
    //         "  .  "
    //     ]);

    //     // right now nothing is pinned, so it should work fine
    //     get_droplet(&mut gv, 'a').destination = None;
    //     get_droplet(&mut gv, 'b').destination = Some(Location { y: 0, x: 0 });

    //     // 'a' moved out of the way
    //     let paths = gv.route().unwrap();
    //     assert_eq!(
    //         paths[&c2id('a')],
    //         path(&[(1, 2), (2, 2), (2, 2), (2, 2), (2, 2), (1, 2)])
    //     );

    //     // once you pin 'a', 'b' no longer has a path
    //     get_droplet(&mut gv, 'a').pinned = true;

    //     assert!(gv.route().is_none());
    // }
}
