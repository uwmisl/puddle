use grid::Cell;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Mark {
    #[serde(rename = " ")]
    Empty,
    #[serde(rename = "a")]
    Auto,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
enum CellIndex {
    Marked(Mark),
    // TODO support manually specified pins
    // but it gets semi-complicated with the interaction of auto pins and
    // specified pins
    // Index(u32),
}

use self::CellIndex::*;
use self::Mark::*;

type ParsedGridVec = Vec<Vec<CellIndex>>;
type GridVec = Vec<Vec<Option<Cell>>>;

pub fn deserialize<'de, D>(d: D) -> Result<GridVec, D::Error>
where
    D: Deserializer<'de>,
{
    let pg_vec: ParsedGridVec = try!(Vec::deserialize(d));
    let mut next_pin = 0;

    let vec = pg_vec
        .iter()
        .map(|row| {
            row.iter()
                .map(|ci: &CellIndex| match ci {
                    &Marked(Empty) => None,
                    &Marked(Auto) => {
                        let pin = next_pin;
                        next_pin += 1;
                        Some(Cell { pin: pin })
                    }
                })
                .collect()
        })
        .collect();

    Ok(vec)
}

pub fn serialize<S>(gv: &GridVec, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let pg_vec: ParsedGridVec = gv.iter()
        .map(|row| {
            row.iter()
                .map(|opt: &Option<Cell>| match opt {
                    &None => Marked(Empty),
                    &Some(_) => Marked(Auto),
                })
                .collect()
        })
        .collect();

    pg_vec.serialize(s)
}

#[cfg(test)]
mod tests {

    use super::*;

    use glob::glob;
    use std::fs::File;

    use serde_json as sj;

    use grid::{Grid, Location};
    use std::collections::{HashMap, HashSet};

    fn parse_strings(rows: &[&str]) -> (Grid, HashMap<char, Blob>) {
        use grid::location::tests::connected_components;

        let mut droplet_map = HashMap::new();
        let mut cell_locs = HashSet::new();

        for (i, row) in rows.iter().enumerate() {
            for (j, ch) in row.chars().enumerate() {
                // we think this is a droplet is it's alphanumeric
                if ch.is_alphanumeric() {
                    // add this location to those for this character
                    let locs = droplet_map.entry(ch).or_insert_with(|| Vec::new());
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
                let labels = connected_components(locs.iter().cloned());
                assert!(labels.values().all(|v| *v == 0));
                (ch, Blob::from_locations(&locs).expect("not a blob!"))
            })
            .collect();

        let mut next_pin = 0;
        let to_cell = |loc: Location| {
            if cell_locs.contains(&loc) {
                let pin = next_pin;
                next_pin += 1;
                Some(Cell { pin })
            } else {
                None
            }
        };

        let height = cell_locs.iter().map(|l| l.y as usize).max().unwrap_or(0) + 1;
        let width = cell_locs.iter().map(|l| l.x as usize).max().unwrap_or(0) + 1;
        let grid = Grid::from_function(to_cell, height, width);

        (grid, blob_map)
    }

    struct Blob {
        location: Location,
        dimensions: Location,
    }

    impl Blob {
        fn from_locations(locs: &[Location]) -> Option<Blob> {
            let location = Location {
                y: locs.iter().map(|l| l.y).min().unwrap_or(0),
                x: locs.iter().map(|l| l.x).min().unwrap_or(0),
            };
            let far_corner = Location {
                y: locs.iter().map(|l| l.y).max().unwrap_or(0) + 1,
                x: locs.iter().map(|l| l.x).max().unwrap_or(0) + 1,
            };
            let dimensions = &far_corner - &location;

            let set1: HashSet<Location> = locs.iter().cloned().collect();
            let mut set2 = HashSet::new();

            // build set2 with all the locations the rectangle should have
            for i in 0..dimensions.y {
                for j in 0..dimensions.x {
                    set2.insert(Location {
                        y: location.y + i,
                        x: location.x + j,
                    });
                }
            }

            println!("{:?} =? {:?}", set1, set2);
            if set1 == set2 {
                Some(Blob {
                    location,
                    dimensions,
                })
            } else {
                None
            }
        }
    }

    #[test]
    fn test_simple_parse() {
        let _: ParsedGridVec = sj::from_str(
            "[[\"a\", \"a\", \"a\"],
              [\"a\", \"a\", \"a\"]]",
        ).expect("parse failed");
    }

    fn check_round_trip(grid: Grid) {
        let s = sj::to_string(&grid).expect("serialization failed");
        let grid2: Grid = sj::from_str(&s).expect("deserialization failed");
        assert_eq!(grid, grid2);
    }

    #[test]
    fn test_parse_files() {
        for entry in glob("../tests/arches/**/*.json").unwrap() {
            let path = entry.expect("glob failed");
            let reader = File::open(path).expect("file not found");
            let grid = sj::from_reader(reader).expect("parse failed");
            check_round_trip(grid);
        }
    }

    #[test]
    fn test_parse() {
        // test uneven string lengths with gaps
        let strs = vec![".....aa.....", "  ...aa...      ", ".bb.........  "];

        let (grid, blobs) = parse_strings(&strs);

        assert_eq!(blobs[&'a'].location, Location { y: 0, x: 5 });
        assert_eq!(blobs[&'a'].dimensions, Location { y: 2, x: 2 });

        assert_eq!(blobs[&'b'].location, Location { y: 2, x: 1 });
        assert_eq!(blobs[&'b'].dimensions, Location { y: 1, x: 2 });

        assert_eq!(grid.max_height(), 3);
        assert_eq!(grid.max_width(), 12);
    }
}
