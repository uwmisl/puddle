use serde_yaml;

use std::io::Read;

#[derive(Debug)]
pub enum GridParseError {
    ParseError(serde_yaml::Error),
    DataError(String),
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mark {
    #[serde(rename = "_")]
    Empty,
    #[serde(rename = "a")]
    Auto,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CellIndex {
    Marked(Mark),
    // TODO support manually specified pins
    // but it gets semi-complicated with the interaction of auto pins and
    // specified pins
    // Index(u32),
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedGrid {
    pub board: Vec<Vec<CellIndex>>,
}


impl ParsedGrid {
    fn from_yaml_reader<R: Read>(reader: R) -> Result<Self, GridParseError> {
        serde_yaml::from_reader(reader).map_err(GridParseError::ParseError)
    }
    fn from_yaml_string(str: &str) -> Result<Self, GridParseError> {
        serde_yaml::from_str(str).map_err(GridParseError::ParseError)
    }
    fn to_yaml_string(&self) -> Result<String, GridParseError> {
        serde_yaml::to_string(self).map_err(GridParseError::ParseError)
    }

}

#[cfg(test)]
mod tests {

    use super::*;
    use super::CellIndex::*;
    use super::Mark::*;

    use std::fs::File;
    use glob::glob;

    #[test]
    fn test_parse() {
        let grid = ParsedGrid::from_yaml_string("board: [[a, a, a], [a, a, a]]");

        println!("{:?}", grid);
    }

    #[test]
    fn test_print() {

        let grid = ParsedGrid {
            board: vec![
                vec![ Marked(Auto), Marked(Auto), Marked(Auto) ],
                vec![ Marked(Auto), Marked(Auto), Marked(Auto) ],
            ],
        };

        println!("{}", ParsedGrid::to_yaml_string(&grid).unwrap());
    }

    fn check_round_trip(grid: ParsedGrid) {
        let s = grid.to_yaml_string().unwrap();
        let grid2 = ParsedGrid::from_yaml_string(&s).unwrap();
        assert_eq!(grid, grid2);
    }

    #[test]
    fn test_parse_files() {
        for entry in glob("../tests/arches/**/*.yaml").unwrap() {
            let path = entry.unwrap();
            println!("Parsing file {}", path.to_str().unwrap());
            let reader = File::open(path).expect("file not found");
            let grid = ParsedGrid::from_yaml_reader(reader).unwrap();
            check_round_trip(grid);
        }
    }
}
