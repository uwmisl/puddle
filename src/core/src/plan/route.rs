use std::collections::HashSet;
use std::time::Instant;

use grid::{Droplet, DropletId, Grid, GridView, Location};
use plan::minheap::MinHeap;

use util::collections::Entry::*;
use util::collections::{Map, Set};
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
    time: Time,
}

type Time = u32;
type Cost = u32;
const MOVE_COST: Cost = 100;
const STAY_COST: Cost = 1;

#[derive(Default)]
struct AvoidanceSet {
    max_time: Time,
    present: Map<(Location, Time), Node>,
    finals: Map<Location, Node>,
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
        self.present.get(&(node.location, node.time)).map(|n| {
            if n.collision_group == node.collision_group {
                SameGroup
            } else {
                DifferentGroup
            }
        })
    }

    fn collides_with_final(&self, node: &Node) -> bool {
        self.finals
            .get(&node.location)
            .filter(|n| n.collision_group != node.collision_group)
            .map_or(false, |&fin| node.time >= fin.time)
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
        });
        for node in node_path {
            self.avoid_node(grid, node, droplet);
        }

        // Add last element to finals
        let last = path.len() - 1;
        for loc in grid.neighbors_dimensions(&path[last], &droplet.dimensions) {
            let earliest_time = self.finals
                .get(&loc)
                .map_or(last as Time, |&prev| prev.time.min(last as Time));
            self.finals.insert(
                loc,
                Node {
                    time: earliest_time,
                    collision_group: droplet.collision_group,
                    location: loc,
                },
            );
        }

        self.max_time = self.max_time.max(last as Time)
    }

    fn avoid_node(&mut self, grid: &Grid, node: Node, droplet: &Droplet) {
        for loc in grid.neighbors_dimensions(&node.location, &droplet.dimensions) {
            for t in -1..2 {
                let time = (node.time as i32) + t;
                if time < 0 {
                    continue;
                }
                self.present.insert(
                    (loc, time as Time),
                    Node {
                        location: loc,
                        collision_group: droplet.collision_group,
                        time: time as Time,
                    },
                );
            }
        }
    }
}

impl Node {
    /// Returns a vector representing possible locations on the given `Grid` that can be the next
    /// location for this `Node`. This uses `neighbors4`, since droplets only move in the cardinal
    /// directions.
    fn expand(&self, grid: &Grid) -> Vec<(Cost, Node)> {
        let mut vec: Vec<(Cost, Node)> = grid.neighbors4(&self.location)
            .iter()
            .map(|&location| {
                (
                    MOVE_COST,
                    Node {
                        location,
                        collision_group: self.collision_group,
                        time: self.time + 1,
                    },
                )
            })
            .collect();

        vec.push((
            STAY_COST,
            Node {
                location: self.location,
                collision_group: self.collision_group,
                time: self.time + 1,
            },
        ));

        vec
    }
}

impl GridView {
    pub fn route(&self) -> Option<Map<DropletId, Path>> {
        let mut droplets = self.snapshot().droplets.iter().collect::<Vec<_>>();
        let mut rng = mk_rng();
        for i in 1..50 {
            rng.shuffle(&mut droplets);
            let result = route_many(&droplets, &self.grid, &self.bad_edges);
            if result.is_some() {
                return result;
            }
            trace!("route failed, trying iteration {}", i);
        }

        None
    }
}

fn route_many(
    droplets: &[(&DropletId, &Droplet)],
    grid: &Grid,
    bad_edges: &Set<(Location, Location)>,
) -> Option<Map<DropletId, Path>> {
    let mut av_set = AvoidanceSet::default();
    let num_cells = grid.locations().count();

    let mut paths = Map::new();
    let mut max_t = 0;

    for &(&id, droplet) in droplets.iter() {
        // route a single droplet
        let result = {
            let max_time = num_cells as Time + max_t;
            let next_fn = |node: &Node| {
                node.expand(grid)
                    .iter()
                    .filter(|(_cost, n)| {
                        let l1 = node.location;
                        let l2 = n.location;
                        !av_set.should_avoid(n) && !bad_edges.contains(&(l1, l2))
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            };
            let done_fn = |node: &Node| {
                node.location == match droplet.destination {
                    Some(x) => x,
                    None => droplet.location,
                } && !av_set.would_finally_collide(node)
            };

            route_one(&droplet, max_time, next_fn, done_fn)
        };
        let path = match result {
            None => return None,
            Some(path) => path,
        };

        max_t = max_t.max(path.len() as Time);

        // once we know this path works, add to our avoidance set
        av_set.avoid_path(&path, grid, &droplet);
        paths.insert(id, path);
    }

    Some(paths)
}

fn route_one<FNext, FDone>(
    droplet: &Droplet,
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
        time: 0,
    };
    todo.push(0, start_node);
    best_so_far.insert(start_node, 0);

    let dest = match droplet.destination {
        Some(x) => x,
        None => droplet.location,
    };

    // use manhattan distance from goal as the heuristic
    let heuristic = |node: Node| -> Cost { dest.distance_to(&node.location) * MOVE_COST };

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
        dst = droplet
            .destination
            .map_or("nowhere".into(), |dst| format!("{}", dst))
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

    use super::*;
    use grid::gridview::tests::{c2id, parse_gridview};

    // TODO make some tests

    fn path(locs: &[(i32, i32)]) -> Path {
        locs.iter().map(|&(y, x)| Location { y, x }).collect()
    }

    #[test]
    fn test_routing() {
        let mut gv = parse_gridview(&["a...b", "  .  ", "  .  "]);

        let dest = Location { y: 2, x: 2 };
        gv.snapshot_mut()
            .droplets
            .get_mut(&c2id('a'))
            .unwrap()
            .destination = Some(dest);
        gv.snapshot_mut()
            .droplets
            .get_mut(&c2id('b'))
            .unwrap()
            .destination = Some(dest);

        // this should fail because the droplets aren't allow to collide
        assert!(gv.route().is_none());

        gv.snapshot_mut()
            .droplets
            .get_mut(&c2id('a'))
            .unwrap()
            .collision_group = 42;
        gv.snapshot_mut()
            .droplets
            .get_mut(&c2id('b'))
            .unwrap()
            .collision_group = 42;

        // this should work, as the droplets are allowed to collide now
        // but, we check to make sure that they collide at the end of the path
        let paths = gv.route().unwrap();

        assert_eq!(
            paths[&c2id('a')],
            path(&[(0, 0), (0, 1), (0, 2), (1, 2), (2, 2),])
        );

        assert_eq!(
            paths[&c2id('b')],
            path(&[
                (0, 4),
                (0, 4),
                (0, 4),
                (0, 4),
                (0, 4),
                (0, 3),
                (0, 2),
                (1, 2),
                (2, 2)
            ])
        );
    }

}
