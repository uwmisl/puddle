
use std::collections::{HashSet, HashMap};
use std::collections::hash_map::Entry::{Occupied, Vacant};

use plan::minheap::MinHeap;
use arch::{Location, Droplet, DropletId, Architecture};
use arch::grid::Grid;


pub type Path = Vec<Location>;

fn build_path(mut came_from: HashMap<Node, Node>, end_node: Node) -> Path {
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
    location: Location,
    time: Time,
}

type CollisionGroup = usize;
type Time = u32;
type Cost = u32;
type NextVec = Vec<(Cost, Node)>;

#[derive(Default)]
struct AvoidanceSet {
    max_time: Time,
    present: HashMap<Node, CollisionGroup>,
    finals: HashMap<Location, (Time, CollisionGroup)>,
}

impl AvoidanceSet {
    fn filter(&self, vec: NextVec, cg: CollisionGroup) -> NextVec {
        vec.into_iter()
            .filter(|&(_cost, node)|
                    // make sure that it's either not in the map, or it's the
                    // same as the collision group that's there
                    !self.collides(&node, cg)
                    && !self.collides_with_final(&node, cg))
            .collect()
    }

    fn collides(&self, node: &Node, cg: CollisionGroup) -> bool {
        // if not present, no collision
        // if present, collision iff cg not the same
        self.present.get(&node).map_or(false, |cg2| *cg2 != cg)
    }

    fn collides_with_final(&self, node: &Node, cg: CollisionGroup) -> bool {
        self.finals.get(&node.location).map_or(false, |&(final_t,
           final_cg)| {
            node.time >= final_t && final_cg != cg
        })
    }

    fn would_finally_collide(&self, node: &Node, cg: CollisionGroup) -> bool {
        (node.time..self.max_time)
            .map(|t| {
                Node {
                    time: t,
                    location: node.location,
                }
            })
            .any(|future_node| self.collides(&future_node, cg))
    }

    fn avoid_path(&mut self, path: &Path, cg: CollisionGroup, grid: &Grid) {
        let node_path = path.clone().into_iter().enumerate().map(|(i, loc)| {
            Node {
                time: i as Time,
                location: loc,
            }
        });
        for node in node_path {
            self.avoid_node(grid, node, cg);
        }

        let last = path.len() - 1;
        for loc in grid.neighbors9(&path[last]) {
            self.finals.insert(loc, (last as Time, cg));
        }

        self.max_time = self.max_time.max(last as Time)
    }

    fn avoid_node(&mut self, grid: &Grid, node: Node, cg: CollisionGroup) {
        for loc in grid.neighbors9(&node.location) {
            for t in -1..2 {
                let time = (node.time as i32) + t;
                if time < 0 {
                    continue;
                }
                self.present.insert(
                    Node {
                        location: loc,
                        time: time as Time,
                    },
                    cg,
                );
            }
        }
    }
}

impl Node {
    fn expand(&self, grid: &Grid) -> NextVec {
        let mut vec: Vec<(Cost, Node)> = grid.neighbors4(&self.location)
            .iter()
            .map(|&location| {
                (
                    1,
                    Node {
                        location: location,
                        time: self.time + 1,
                    },
                )
            })
            .collect();

        vec.push((
            100,
            Node {
                location: self.location,
                time: self.time + 1,
            },
        ));

        vec
    }
}

impl Architecture {
    pub fn route(&self) -> Option<HashMap<DropletId, Path>> {
        let mut av_set = AvoidanceSet::default();
        let grid = &self.grid;
        let num_cells = self.grid.locations().count();

        let mut paths = HashMap::new();
        let mut max_t = 0;

        for (&id, droplet) in self.droplets.iter() {
            let cg = droplet.collision_group;
            let result = route_one(
                droplet,
                num_cells as Time + max_t,
                |node| av_set.filter(node.expand(grid), cg),
                |node| node.location == match droplet.destination {
                        Some(x) => x,
                        None => droplet.location
                    }
                    && !av_set.would_finally_collide(node, cg)
            );
            let path = match result {
                None => return None,
                Some(path) => path,
            };

            max_t = max_t.max(path.len() as Time);

            av_set.avoid_path(&path, cg, grid);
            paths.insert(id, path);
        }

        Some(paths)
    }
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
    let mut todo: MinHeap<Cost, Node> = MinHeap::new();
    let mut best_so_far: HashMap<Node, Cost> = HashMap::new();
    let mut came_from: HashMap<Node, Node> = HashMap::new();
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
    use super::*;

    use proptest::prelude::*;

    use arch::tests::*;
    use arch::grid::tests::*;

    pub fn check_path_on_grid(droplet: &Droplet, path: &Path, grid: &Grid) {
        assert_eq!(droplet.location, path[0]);
        let dest = match droplet.destination {
            Some(x) => x,
            None => droplet.location,
        };
        assert_eq!(dest, path[path.len() - 1]);
        for win in path.windows(2) {
            assert!(grid.get_cell(&win[0]).is_some());
            assert!(grid.get_cell(&win[1]).is_some());
            assert!(win[0].distance_to(&win[1]) <= 1);
        }
    }

    fn uncrowded_arch_from_grid(grid: Grid) -> BoxedStrategy<Architecture> {

        let height = grid.vec.len();
        let width = grid.vec.iter().map(|row| row.len()).min().unwrap();
        let max_dim = height.min(width) / 2;
        let max_droplets = (max_dim as usize).max(1);
        arb_arch_from_grid(grid, 0..max_droplets)
    }

    proptest! {

        #[test]
        fn route_one_connected(
            ref arch in arb_grid((5..10), (5..10), 0.95)
                .prop_filter("not connected", |ref g| g.is_connected())
                .prop_flat_map(move |g| arb_arch_from_grid(g, 1..2)))
        {
            let droplet = arch.droplets.values().next().unwrap();
            let num_cells = arch.grid.locations().count();

            let path = route_one(
                &droplet,
                num_cells as Time,
                |node| node.expand(&arch.grid),
                |node| node.location == match droplet.destination {
                        Some(x) => x,
                        None => droplet.location
                    }
            )
                .unwrap();
            check_path_on_grid(&droplet, &path, &arch.grid)
        }

        #[test]
        fn route_connected(
            ref rarch in arb_grid((5..10), (5..10), 0.95)
                .prop_filter("not connected", |ref g| g.is_connected())
                .prop_flat_map(uncrowded_arch_from_grid)
                .prop_filter("starting collision",
                             |ref a| a.get_collision().is_none())
                .prop_filter("ending collision",
                             |ref a| a.get_destination_collision().is_none())
        )
        {
            let mut arch = rarch.clone();
            prop_assume!(arch.route().is_some());
            let paths = arch.route().unwrap();
            prop_assert_eq!(paths.len(), arch.droplets.len());
            // FIXME!!
            // arch.take_paths(paths, || {})
        }
    }
}
