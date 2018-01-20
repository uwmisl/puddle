
use std::collections::{HashSet, HashMap};
use std::collections::hash_map::Entry::{Occupied, Vacant};

use minheap::MinHeap;
use arch::{Location, Grid, Droplet, Architecture};


type Path = Vec<Location>;

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
    time: u32,
}

type Cost = u32;

impl Node {
    fn expand(&self, grid: &Grid) -> Vec<(Cost, Node)> {
        let mut vec: Vec<(u32, Node)> = grid.neighbors(&self.location)
            .iter()
            .map(|&location| {
                (1,
                 Node {
                    location: location,
                    time: self.time + 1,
                })
            })
            .collect();

        vec.push((100,
                  Node {
            location: self.location,
            time: self.time + 1,
        }));

        // println!("Expanding vec: {:?}", vec);
        vec
    }
}


pub fn route_one(droplet: &Droplet, arch: &Architecture) -> Option<Path> {
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

    // use manhattan distance from goal as the heuristic
    let heuristic = |node: Node| -> u32 { droplet.destination.distance_to(&node.location) };

    while let Some((est_cost, node)) = todo.pop() {

        if node.location == droplet.destination {
            let path = build_path(came_from, node);
            return Some(path);
        }

        // insert returns false if value was already there
        if !done.insert(node) {
            continue;
        }

        println!("Popping: {:?}, {:?}", node, est_cost);

        // node must be in best_so_far because it was inserted when we put it in
        // the minheap
        let node_cost: Cost = *best_so_far.get(&node).unwrap();

        for (edge_cost, next) in node.expand(&arch.grid) {

            if done.contains(&next) {
                continue;
            }

            let mut next_cost = node_cost + edge_cost;

            match best_so_far.entry(next) {
                Occupied(entry) => {
                    // println!("FOUND IN MAP");
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
            println!("  Pushing: {:?}, {:?}, h{:?}",
                     next,
                     next_cost_est,
                     heuristic(next));
            todo.push(next_cost_est, next)
        }

    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn it_works() {
        let grid = Grid::rectangle(10, 10);
        let arch = Architecture {
            grid: grid,
            droplets: HashSet::new(),
        };

        let start = Location { y: 5, x: 0 };
        let end = Location { y: 9, x: 9 };

        let droplet = Droplet {
            location: start,
            destination: end,
        };

        let path = route_one(&droplet, &arch).unwrap();

        println!("{:?}", path);

        assert_eq!(droplet.location, path[0]);
        assert_eq!(droplet.destination, path[path.len() - 1]);

        for win in path.windows(2) {
            assert!(win[0].distance_to(&win[1]) == 1)
        }
    }
}
