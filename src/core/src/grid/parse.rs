use serde::{Deserialize, Serialize};
use serde_aux::field_attributes::deserialize_number_from_string;
use serde_json;
use std::io::Read;

use crate::grid::grid::*;
use crate::util::HashMap;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mark {
    #[serde(rename = " ")]
    Empty,
    #[serde(rename = "a")]
    Auto,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParsedElectrode {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    Index(u32),
    Marked(Mark),
}

use self::Mark::*;
use self::ParsedElectrode::*;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PiConfig {
    pub polarity: PolarityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolarityConfig {
    pub frequency: f64,
    pub duty_cycle: f64,
}

impl Default for PolarityConfig {
    fn default() -> PolarityConfig {
        PolarityConfig {
            frequency: 500.0,
            duty_cycle: 0.5,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParsedGrid {
    #[serde(default)]
    pub pi_config: PiConfig,
    pub board: Vec<Vec<ParsedElectrode>>,
    #[serde(default)]
    pub peripherals: HashMap<String, Peripheral>,
}

impl ParsedGrid {
    pub fn from_reader<R: Read>(reader: R) -> Result<ParsedGrid, serde_json::Error> {
        serde_json::from_reader(reader)
    }

    pub fn to_grid(&self) -> Grid {
        // find a pin that higher than anything listed
        let mut next_auto_pin = self
            .board
            .iter()
            .flat_map(|row| row.iter())
            .filter_map(|ci| match ci {
                Index(n) => Some(n + 1),
                _ => None,
            })
            .max()
            .unwrap_or(0);

        let mut f = |pe: &ParsedElectrode| match pe {
            Marked(Empty) => None,
            Marked(Auto) => {
                let pin = next_auto_pin;
                next_auto_pin += 1;
                Some(Electrode {
                    pin: pin,
                    peripheral: None,
                })
            }
            Index(n) => Some(Electrode {
                pin: *n,
                peripheral: None,
            }),
        };

        let mut grid = Grid {
            vec: self
                .board
                .iter()
                .map(|row| row.iter().map(&mut f).collect())
                .collect(),
        };

        for (location, periph) in self.peripherals.iter() {
            let loc = location.parse().unwrap();
            let electrode = grid.get_cell_mut(loc).unwrap();
            assert_eq!(electrode.peripheral, None);
            electrode.peripheral = Some(periph.clone());
        }

        grid
    }
}

#[cfg(test)]
pub mod tests {

    use super::*;
    use crate::tests::project_path;

    use glob::glob;
    use std::fs::File;

    use serde_json as sj;

    use crate::grid::{droplet::SimpleBlob, Grid, Location};
    use crate::util::{HashMap, HashSet};
    use std::env;

    /// Returns the number of connected components where diagonal
    /// counts as connected
    fn connected_components(locs: &[Location]) -> usize {
        use petgraph::{graphmap::GraphMap, Undirected};
        let mut graph = GraphMap::<Location, (), Undirected>::default();
        for loc in locs {
            graph.add_node(*loc);
        }
        for &loc in locs {
            for y in -1..=1 {
                for x in -1..=1 {
                    let other = loc + Location { y, x };
                    if graph.contains_node(other) && loc != other {
                        graph.add_edge(loc, other, ());
                    }
                }
            }
        }

        petgraph::algo::connected_components(&graph)
    }

    pub fn parse_strings(rows: &[&str]) -> (Grid, HashMap<char, SimpleBlob>) {
        let mut droplet_map = HashMap::default();
        let mut cell_locs = HashSet::default();

        for (i, row) in rows.iter().enumerate() {
            for (j, ch) in row.chars().enumerate() {
                // we think this is a droplet is it's alphanumeric
                if ch.is_alphanumeric() {
                    // add this location to those for this character
                    let locs = droplet_map.entry(ch).or_insert_with(Vec::new);
                    locs.push(Location {
                        y: i as i32,
                        x: j as i32,
                    })
                } else if ch == ' ' {
                    continue;
                } else {
                    // this function should only be used in testing, so for now we just panic if the input isn't well formed.
                    assert_eq!(ch, '.');
                }

                cell_locs.insert(Location {
                    y: i as i32,
                    x: j as i32,
                });
            }
        }

        let blob_map: HashMap<_, _> = droplet_map
            .iter()
            .map(|(&ch, locs)| {
                // make sure it only has one connected component
                assert_eq!(connected_components(locs), 1);
                (ch, SimpleBlob::from_locations(&locs).expect("not a blob!"))
            })
            .collect();

        let mut next_pin = 0;
        let to_cell = |loc: Location| {
            if cell_locs.contains(&loc) {
                let pin = next_pin;
                next_pin += 1;
                Some(Electrode {
                    pin,
                    peripheral: None,
                })
            } else {
                None
            }
        };

        let height = cell_locs.iter().map(|l| l.y as usize).max().unwrap_or(0) + 1;
        let width = cell_locs.iter().map(|l| l.x as usize).max().unwrap_or(0) + 1;
        let grid = Grid::from_function(to_cell, height, width);

        (grid, blob_map)
    }

    #[test]
    fn test_simple_parse() {
        let _: ParsedGrid = sj::from_str(
            r#"
            {
                "board": [["a", "a", "a"],
                          ["a", "a", "a"]]
            }
        "#,
        )
        .expect("parse failed");
    }

    #[test]
    fn test_parse_number_or_string() {
        let _: ParsedGrid =
            sj::from_str(r#" { "board": [[1, "2", "3", " "]] } "#).expect("parse failed");
    }

    fn check_round_trip(grid: Grid, desc: &str) {
        let s = sj::to_string(&grid).expect("serialization failed");
        let grid2: Grid = sj::from_str(&s).expect("deserialization failed");
        if grid != grid2 {
            error!("Failed on {}", desc);
            assert_eq!(grid, grid2);
        }
    }

    #[test]
    fn test_parse_files() {
        let mut successes = 0;
        let _ = env_logger::try_init();

        if let Ok(s) = env::var("RUNNING_IN_CROSS") {
            if s == "1" {
                // we have to skip because the test files are out-of-tree
                info!("Skipping test because we're running in qemu");
                return;
            }
        }

        debug!("{}", project_path("/tests/arches/*.json"));
        for entry in glob(&project_path("/tests/arches/*.json")).unwrap() {
            trace!("Testing {:?}", entry);
            let path = entry.expect("glob failed");
            let reader = File::open(path.clone()).expect("file not found");
            let grid = sj::from_reader(reader).expect("parse failed");
            check_round_trip(grid, path.to_str().unwrap());
            successes += 1;
        }
        debug!("Tested {} parsing round trips", successes);
        assert!(successes >= 3);
    }

    #[test]
    fn test_parse() {
        // test uneven string lengths with gaps
        let strs = vec![
            ".....aa.....",
            "  ...aa...      ",
            ".bb.........  ",
            "  ......  ",
        ];

        let (grid, blobs) = parse_strings(&strs);

        assert_eq!(blobs[&'a'].location, Location { y: 0, x: 5 });
        assert_eq!(blobs[&'a'].dimensions, Location { y: 2, x: 2 });

        assert_eq!(blobs[&'b'].location, Location { y: 2, x: 1 });
        assert_eq!(blobs[&'b'].dimensions, Location { y: 1, x: 2 });

        assert_eq!(grid.max_height(), 4);
        assert_eq!(grid.max_width(), 12);
    }
}
