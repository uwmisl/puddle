extern crate pathfinding;

use pathfinding::kuhn_munkres::*;
use pathfinding::matrix::*;

use super::{Droplet, DropletId};
use grid::droplet::Blob;

use std::collections::HashMap;

/*
 * Takes a map of droplet ids to droplets (as in that
 * of the planner/executor view) and a vector of blobs
 * (as in that of the chip view) and returns a matching
 * of droplet ids to closest matching blobs.
 *
 * Can currently only handle where both views contain
 * the same number of 'droplets'
 */
#[allow(dead_code)]
pub fn match_views(
    exec_view: HashMap<DropletId, Droplet>,
    chip_view: Vec<Blob>,
) -> HashMap<DropletId, Blob> {
    // Ensure lengths are the same
    if exec_view.len() != chip_view.len() {
        panic!("Expected and actual droplets are of different lengths");
    }
    let mut result = HashMap::new(); // to be returned
    let mut ids = vec![]; // store corresponding ids to indeces
    let mut matches = vec![]; // store similarity between blobs/droplets
    let n = chip_view.len();

    // store the id of each droplet in its corresponding
    // index in 'ids', then store the similarity of each
    // droplet to each blob in 'matches'
    for (id, droplet) in &exec_view {
        ids.push(id);
        for blob in chip_view.clone().into_iter() {
            matches.push(get_similarity(&blob, droplet));
        }
    }

    // convert the matches vector to a matrix
    // input should be [1,2,3,4], where the output
    // matrix is [[1,2],[3,4]]
    let m: Matrix<i32> = Matrix::from_vec(n, n, matches);

    // km is a vector of size n where the value at each index
    // corresponds to the index of a blob
    let (_c, km) = kuhn_munkres_min(&m);

    for i in 0..n {
        result.insert(*ids[i], chip_view[km[i]].clone());
    }
    result
}

pub fn get_similarity(blob: &Blob, droplet: &Droplet) -> i32 {
    blob.location.distance_to(&droplet.location) as i32
        + blob.dimensions.distance_to(&droplet.dimensions) as i32
        + ((blob.volume - droplet.volume) as i32).abs()
}

#[cfg(test)]
pub mod tests {
    use super::super::parse;
    use super::*;

    /*
     * Converts a parsed map of characters to blobs into
     * a map of sequentially assigned droplet ids to droplets
     * with the same structure as their corresponding blob
     */
    fn blob_map_to_droplet_map(
        blobs: HashMap<char, Blob>,
    ) -> (HashMap<DropletId, Droplet>, Vec<char>) {
        let mut droplet_vec: HashMap<DropletId, Droplet> = HashMap::new();
        let mut char_to_id: Vec<char> = vec![];

        for c in blobs.keys() {
            let droplet_id = DropletId {
                id: char_to_id.len(),
                process_id: 0,
            };
            let blob = blobs.get(c).unwrap();
            droplet_vec.insert(
                droplet_id,
                Droplet::new(droplet_id, blob.volume, blob.location, blob.dimensions),
            );
            char_to_id.push(c.clone());
        }
        (droplet_vec, char_to_id)
    }

    #[test]
    fn test_no_diff() {
        let strs = vec!["aa..........c", ".....bb......", "............."];

        // parse the string representation of droplet locations/dimensions
        let (_, exec_blobs) = parse::tests::parse_strings(&strs);

        // chip/exec blobs are the same when testing things work with
        // no difference
        let (_, chip_blobs) = parse::tests::parse_strings(&strs);

        // convert the exec_blobs to a droplet id/droplet map
        let (exec_view, char_to_id) = blob_map_to_droplet_map(exec_blobs);

        // create the expected map by mapping the ids in the exec view
        // to the associated blob which corresponds to the character
        // at a given id
        let mut expected: HashMap<DropletId, Blob> = HashMap::new();
        for id in exec_view.keys() {
            expected.insert(*id, chip_blobs[&char_to_id[id.id]].clone());
        }

        let result: HashMap<DropletId, Blob> = super::match_views(
            exec_view,
            chip_blobs.into_iter().map(|(_, blob)| blob).collect(),
        );
        for id in expected.keys() {
            assert_eq!(result.get(id), expected.get(id));
        }
    }

    #[test]
    fn test_location_diff() {
        let exec_strs = vec!["aa..........c", ".....bb......", "............."];

        let chip_strs = vec!["aa...........", "............c", ".....bb......"];

        let (_, exec_blobs) = parse::tests::parse_strings(&exec_strs);
        let (_, chip_blobs) = parse::tests::parse_strings(&chip_strs);

        let (exec_view, char_to_id) = blob_map_to_droplet_map(exec_blobs);

        let mut expected: HashMap<DropletId, Blob> = HashMap::new();
        for id in exec_view.keys() {
            expected.insert(*id, chip_blobs[&char_to_id[id.id]].clone());
        }

        let result: HashMap<DropletId, Blob> = super::match_views(
            exec_view,
            chip_blobs.into_iter().map(|(_, blob)| blob).collect(),
        );
        for id in expected.keys() {
            assert_eq!(result.get(id), expected.get(id));
        }
    }

    #[test]
    fn test_dimension_diff() {
        let exec_strs = vec!["aa..........c", ".....bb......", "............."];
        let chip_strs = vec!["aa.........cc", ".....b.......", ".....b......."];

        let (_, exec_blobs) = parse::tests::parse_strings(&exec_strs);
        let (_, chip_blobs) = parse::tests::parse_strings(&chip_strs);

        let (exec_view, char_to_id) = blob_map_to_droplet_map(exec_blobs);

        let mut expected: HashMap<DropletId, Blob> = HashMap::new();
        for id in exec_view.keys() {
            expected.insert(*id, chip_blobs[&char_to_id[id.id]].clone());
        }

        let result: HashMap<DropletId, Blob> = super::match_views(
            exec_view,
            chip_blobs.into_iter().map(|(_, blob)| blob).collect(),
        );
        for id in expected.keys() {
            assert_eq!(result.get(id), expected.get(id));
        }
    }

    #[test]
    #[should_panic(expected = "Expected and actual droplets are of different lengths")]
    fn test_mix_split_diff() {
        let exec_strs = vec!["aa...........", ".....bb..c...", "............."];
        let chip_strs = vec!["aa...........", ".....bbb.....", "............."];

        let (_, exec_blobs) = parse::tests::parse_strings(&exec_strs);
        let (_, chip_blobs) = parse::tests::parse_strings(&chip_strs);

        let (exec_view, _char_to_id) = blob_map_to_droplet_map(exec_blobs);

        let _result: HashMap<DropletId, Blob> = super::match_views(
            exec_view,
            chip_blobs.into_iter().map(|(_, blob)| blob).collect(),
        );
    }
}
