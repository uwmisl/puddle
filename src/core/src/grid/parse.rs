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

use self::Mark::*;
use self::CellIndex::*;

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
    use grid::Grid;

    use std::fs::File;
    use glob::glob;

    use serde_json as sj;

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
}
