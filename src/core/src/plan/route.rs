use std::collections::HashSet;

use plan::minheap::MinHeap;
use grid::{Droplet, DropletId, Grid, GridView, Location};
use exec::Action;

use util::collections::{Map, Set};
use util::collections::Entry::*;

use rand::{thread_rng, Rng};

type Path = Vec<Location>;

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

pub fn paths_to_actions(paths: Map<DropletId, Path>) -> Vec<Action> {
    let max_len = paths.values().map(|p| p.len()).max().unwrap_or(0);
    let mut acts = Vec::new();
    for i in 0..max_len {
        for (id, path) in paths.iter() {
            if i < path.len() {
                acts.push(Action::MoveDroplet {
                    id: *id,
                    location: path[i],
                });
            }
        }
        acts.push(Action::Tick);
    }
    acts
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
struct Node {
    location: Location,
    time: Time,
}

type Time = u32;
type Cost = u32;
type NextVec = Vec<(Cost, Node)>;

#[derive(Default)]
struct AvoidanceSet {
    max_time: Time,
    present: Set<Node>,
    finals: Map<Location, Time>,
}

impl AvoidanceSet {
    fn filter(&self, vec: NextVec) -> NextVec {
        vec.into_iter()
            .filter(|&(_cost, node)|
                    // make sure that it's either not in the map
                    !self.collides(&node)
                    && !self.collides_with_final(&node))
            .collect()
    }

    fn collides(&self, node: &Node) -> bool {
        // if not present, no collision
        self.present.get(&node).is_some()
    }

    fn collides_with_final(&self, node: &Node) -> bool {
        self.finals
            .get(&node.location)
            .map_or(false, |&final_t| node.time >= final_t)
    }

    fn would_finally_collide(&self, node: &Node) -> bool {
        (node.time..self.max_time)
            .map(|t| Node {
                time: t,
                location: node.location,
            })
            .any(|future_node| self.collides(&future_node))
    }

    fn avoid_path(&mut self, path: &Path, grid: &Grid, droplet_dimensions: &Location) {
        let node_path = path.clone().into_iter().enumerate().map(|(i, loc)| Node {
            time: i as Time,
            location: loc,
        });
        for node in node_path {
            self.avoid_node(grid, node, droplet_dimensions);
        }

        // Add last element to finals
        let last = path.len() - 1;
        for loc in grid.neighbors_dimensions(&path[last], droplet_dimensions) {
            self.finals.insert(loc, last as Time);
        }

        self.max_time = self.max_time.max(last as Time)
    }

    fn avoid_node(&mut self, grid: &Grid, node: Node, dimensions: &Location) {
        for loc in grid.neighbors_dimensions(&node.location, dimensions) {
            for t in -1..2 {
                let time = (node.time as i32) + t;
                if time < 0 {
                    continue;
                }
                self.present.insert(Node {
                    location: loc,
                    time: time as Time,
                });
            }
        }
    }
}

impl Node {
    /// Returns a vector representing possible locations on the given `Grid` that can be the next
    /// location for this `Node`. This uses `neighbors4`, since droplets only move in the cardinal
    /// directions.
    fn expand(&self, grid: &Grid) -> NextVec {
        let mut vec: Vec<(Cost, Node)> = grid.neighbors4(&self.location)
            .iter()
            .map(|&location| {
                (
                    100,
                    Node {
                        location,
                        time: self.time + 1,
                    },
                )
            })
            .collect();

        vec.push((
            1,
            Node {
                location: self.location,
                time: self.time + 1,
            },
        ));

        vec
    }
}

impl GridView {
    pub fn route(&self) -> Option<Map<DropletId, Path>> {
        let mut droplets = self.droplets.iter().collect::<Vec<_>>();
        let mut rng = thread_rng();
        for i in 1..50 {
            rng.shuffle(&mut droplets);
            let result = route_many(&droplets, &self.grid);
            if result.is_some() {
                return result;
            }
            trace!("route failed, trying iteration {}", i);
        }

        None
    }
}

fn route_many(droplets: &[(&DropletId, &Droplet)], grid: &Grid) -> Option<Map<DropletId, Path>> {
    let mut av_set = AvoidanceSet::default();
    let num_cells = grid.locations().count();

    let mut paths = Map::new();
    let mut max_t = 0;

    for &(&id, droplet) in droplets.iter() {
        // route a single droplet
        let result = route_one(
            &droplet,
            num_cells as Time + max_t,
            |node| av_set.filter(node.expand(grid)),
            |node| {
                node.location == match droplet.destination {
                    Some(x) => x,
                    None => droplet.location,
                } && !av_set.would_finally_collide(node)
            },
        );
        let path = match result {
            None => return None,
            Some(path) => path,
        };

        max_t = max_t.max(path.len() as Time);

        // once we know this path works, add to our avoidance set
        av_set.avoid_path(&path, grid, &droplet.dimensions);
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
    FNext: FnMut(&Node) -> NextVec,
    FDone: FnMut(&Node) -> bool,
{
    trace!(
        "Routing droplet {} from {} to {}",
        droplet.id.id,
        droplet.location,
        droplet
            .destination
            .map_or("nowhere".into(), |dst| format!("{}", dst))
    );

    let mut todo: MinHeap<Cost, Node> = MinHeap::new();
    let mut best_so_far: Map<Node, Cost> = Map::new();
    let mut came_from: Map<Node, Node> = Map::new();
    // TODO remove done in favor of came_from
    let mut done: HashSet<Node> = HashSet::new();

    let start_node = Node {
        location: droplet.location,
        time: 0,
    };
    todo.push(0, start_node);
    best_so_far.insert(start_node, 0);

    let dest = match droplet.destination {
        Some(x) => x,
        None => droplet.location,
    };

    // use manhattan distance from goal as the heuristic
    let heuristic = |node: Node| -> Cost { dest.distance_to(&node.location) };

    while let Some((_, node)) = todo.pop() {
        if done_fn(&node) {
            let path = build_path(came_from, node);
            return Some(path);
        }

        // insert returns false if value was already there
        if !done.insert(node) || node.time > max_time {
            continue;
        }

        // node must be in best_so_far because it was inserted when we put it in
        // the minheap
        let node_cost: Cost = *best_so_far.get(&node).unwrap();

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
    }

    None
}

#[cfg(test)]
pub mod tests {

    // TODO make some tests

}
