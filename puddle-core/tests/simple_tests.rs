use std::collections::{HashMap, HashSet};
use std::env;

use matches::assert_matches;
use puddle_core::{grid::location::yx, prelude::*, process::ProcessHandle};

fn manager_from_str(s: &str) -> Manager {
    let _ = env_logger::builder().is_test(true).try_init();
    let grid = serde_yaml::from_reader(s.as_bytes()).unwrap();
    // reduce the step delay for testing
    env::set_var("PUDDLE_STEP_DELAY_MS", "1");

    let blocking = false;
    Manager::new(blocking, grid)
}

fn manager_from_rect(rows: usize, cols: usize) -> Manager {
    let _ = env_logger::builder().is_test(true).try_init();
    let grid = Grid::rectangle(rows, cols);

    // reduce the step delay for testing
    env::set_var("PUDDLE_STEP_DELAY_MS", "1");

    let blocking = false;
    Manager::new(blocking, grid)
}

fn info_dict(p: &ProcessHandle) -> HashMap<DropletId, DropletInfo> {
    p.flush().unwrap().into_iter().map(|d| (d.id, d)).collect()
}

fn float_epsilon_equal(float1: f64, float2: f64) -> bool {
    let epsilon = 0.00001f64;
    (float1 - float2).abs() < epsilon
}

#[test]
fn create_some_droplets() {
    let man = manager_from_rect(1, 4);
    let p = man.get_new_process("test");

    let loc = yx(0, 0);
    let id = p.create(Some(loc), 1.0, None).unwrap();

    let should_work = p.create(None, 1.0, None);
    // FIXME we don't do ahead of time checking
    // let should_not_work = p.create(None, 1.0, None);

    assert!(should_work.is_ok());
    // assert!(should_not_work.is_err());

    let droplets = info_dict(&p);

    assert_eq!(droplets.len(), 2);
    assert_eq!(droplets[&id].location, loc);
    assert!(float_epsilon_equal(droplets[&id].volume, 1.0));

    p.flush().unwrap();
}

#[test]
fn move_droplet() {
    let man = manager_from_rect(1, 4);
    let p = man.get_new_process("test");

    let loc1 = yx(0, 0);
    let loc2 = yx(0, 3);
    let id1 = p.create(Some(loc1), 1.0, None).unwrap();
    let id2 = p.move_droplet(id1, loc2).unwrap();

    let droplets = info_dict(&p);

    assert_eq!(droplets.len(), 1);
    assert_eq!(droplets[&id2].location, loc2);
    assert!(float_epsilon_equal(droplets[&id2].volume, 1.0));
    assert_eq!(p.ticks(), 6);
}

#[test]
fn mix2() {
    let man = manager_from_rect(20, 20);
    let p = man.get_new_process("test");

    let loc = yx(1, 1);
    let id1 = p.create(Some(loc), 1.0, None).unwrap();
    let id2 = p.create(None, 1.0, None).unwrap();

    let id12 = p.mix(id1, id2).unwrap();
    let droplets = info_dict(&p);

    assert_eq!(droplets.len(), 1);
    assert!(droplets.contains_key(&id12));
    assert!(float_epsilon_equal(droplets[&id12].volume, 2.0));
}

#[test]
fn mix3() {
    let man = manager_from_rect(20, 20);
    let p = man.get_new_process("test");

    let id1 = p.create(None, 1.0, None).unwrap();
    let id2 = p.create(None, 1.0, None).unwrap();
    let id3 = p.create(None, 1.0, None).unwrap();

    let id12 = p.mix(id1, id2).unwrap();
    let id123 = p.mix(id12, id3).unwrap();

    let droplets = info_dict(&p);

    assert_eq!(droplets.len(), 1);
    assert!(droplets.contains_key(&id123));
    assert!(float_epsilon_equal(droplets[&id123].volume, 3.0));
}

#[test]
fn mix_split() {
    let man = manager_from_rect(9, 9);
    let p = man.get_new_process("test");

    let id1 = p.create(None, 1.0, None).unwrap();
    let id2 = p.create(None, 1.0, None).unwrap();

    let id12 = p.mix(id1, id2).unwrap();

    let (id3, id4) = p.split(id12).unwrap();
    let (id5, id6) = p.split(id4).unwrap();

    let droplets = info_dict(&p);

    assert_eq!(
        droplets.keys().collect::<HashSet<_>>(),
        vec![id3, id5, id6].iter().collect()
    );

    assert!(float_epsilon_equal(droplets[&id3].volume, 1.0));
    assert!(float_epsilon_equal(droplets[&id5].volume, 0.5));
}

#[test]
fn process_isolation() {
    // Spawn 6 processes
    let num_processes = 6;

    use std::sync::Arc;

    let manager = Arc::new(manager_from_rect(9, 9));

    // for p in ps {
    for i in 0..num_processes {
        let man = Arc::clone(&manager);
        std::thread::spawn(move || {
            let p = man.get_new_process(format!("test-{}", i));
            let _drop_id = p.create(None, 1.0, None).unwrap();
            p.flush().unwrap();
        });
    }
}

#[test]
#[should_panic(expected = "PlaceError")]
fn create_does_not_fit() {
    let man = manager_from_rect(2, 2);
    let p = man.get_new_process("test");

    let _id1 = p.create(None, 1.0, None).unwrap();
    let id2 = p.create(None, 1.0, None);

    // should panic here
    let _droplets = info_dict(&p);

    assert_matches!(
        id2,
        Err(PuddleError::PlanError(puddle_core::plan::PlanError::PlaceError(_)))
    );
}

fn check_mix_dimensions(dim1: Location, dim2: Location, dim_result: Location) {
    let man = manager_from_rect(20, 20);
    let p = man.get_new_process("test");

    let id1 = p.create(None, 1.0, Some(dim1)).unwrap();
    let id2 = p.create(None, 1.0, Some(dim2)).unwrap();

    let id12 = p.mix(id1, id2).unwrap();

    let droplets = info_dict(&p);

    assert_eq!(droplets.len(), 1);
    assert_eq!(droplets[&id12].dimensions, dim_result);
}

#[test]
fn mix_dimensions_size() {
    check_mix_dimensions(yx(1, 1), yx(2, 1), yx(3, 1));
    check_mix_dimensions(yx(1, 2), yx(2, 1), yx(3, 2));
}

#[test]
#[should_panic(expected = "PlaceError")]
fn mix_dimensions_too_large_to_combine() {
    // recall, this is on 20x20 board

    // too tall to fit vertically
    check_mix_dimensions(yx(11, 3), yx(11, 3), yx(11, 6));

    // too wide to fit horizontally
    check_mix_dimensions(yx(3, 11), yx(3, 11), yx(6, 11));
}

#[test]
fn mix_dimensions_place_must_overlap() {
    // test that a placement can overlap with the input droplets without
    // requiring them to move
    let man = manager_from_rect(3, 3);
    let p = man.get_new_process("test");

    let id1 = p.create(None, 1.0, None).unwrap();
    let id2 = p.create(None, 1.0, None).unwrap();

    let _ = p.mix(id1, id2).unwrap();
}

fn check_split_dimensions(dim: Location, dim1: Location, dim2: Location) {
    let man = manager_from_rect(9, 9);
    let p = man.get_new_process("test");

    let id = p.create(None, 1.0, Some(dim)).unwrap();

    let (id1, id2) = p.split(id).unwrap();

    let droplets = info_dict(&p);

    assert_eq!(droplets.len(), 2);
    assert_eq!(droplets[&id1].dimensions, dim1);
    assert_eq!(droplets[&id2].dimensions, dim2);
}

#[test]
fn split_dimensions_size() {
    check_split_dimensions(yx(1, 1), yx(1, 1), yx(1, 1));
    check_split_dimensions(yx(1, 3), yx(1, 2), yx(1, 2));
}

#[test]
fn create_dimensions_failure_overlap() {
    let man = manager_from_rect(9, 9);
    let p = man.get_new_process("test");

    let dim1 = yx(1, 2);
    let dim2 = yx(1, 1);

    let loc1 = yx(0, 1);
    let loc2 = yx(1, 3);

    let id1 = p.create(Some(loc1), 1.0, Some(dim1)).unwrap();
    let _id2 = p.create(Some(loc2), 1.0, Some(dim2)).unwrap();

    // FIXME not only does create not check, but planning upon flush
    // will actually move one of the placements (the previously placed
    // droplets?). Placement should respect pinned locations in the
    // current flush, right?

    let droplets = dbg!(info_dict(&p));
    assert_eq!(droplets[&id1].location, loc1);
    // FIXME this fails for now
    // assert_eq!(droplets[&id2].location, loc2);
}

#[test]
fn create_dimension() {
    let man = manager_from_rect(9, 9);
    let p = man.get_new_process("test");

    let dim1 = yx(3, 2);

    let id1 = p.create(None, 1.0, Some(dim1)).unwrap();

    let droplets = info_dict(&p);

    assert_eq!(droplets.len(), 1);
    assert_eq!(droplets[&id1].dimensions, dim1);
}

#[test]
fn mix_larger_droplets() {
    let man = manager_from_rect(100, 100);
    let p = man.get_new_process("test");

    let dim1 = yx(4, 6);
    let dim2 = yx(8, 4);

    let id1 = p.create(None, 1.0, Some(dim1)).unwrap();
    let id2 = p.create(None, 1.0, Some(dim2)).unwrap();

    let _id12 = p.mix(id1, id2).unwrap();
}

#[test]
fn split_single_nonzero_dimensions() {
    let man = manager_from_rect(9, 9);
    let p = man.get_new_process("test");

    let dim = yx(1, 1);
    let id0 = p.create(None, 1.0, Some(dim)).unwrap();

    let (id1, id2) = p.split(id0).unwrap();

    let droplets = info_dict(&p);

    assert_eq!(droplets.len(), 2);
    assert_eq!(droplets[&id1].dimensions, dim);
    assert_eq!(droplets[&id2].dimensions, dim);
}

#[test]
fn heat_droplet() {
    let board_str = r#"
        board: [
          [  0,  1,  2,  3,  4 ],
          [  5,  6,  7,  8,  9 ],
          [  _,  _, 10,  _,  _ ],
          [  _,  _, 11,  _,  _ ],
        ]
        peripherals:
          - location: {y: 3, x: 2}
            type: Heater
            pwm_channel: 0
            spi_channel: 0
    "#;

    let man = manager_from_str(board_str);
    let p = man.get_new_process("test");

    let dim = yx(1, 1);
    let id0 = p.create(None, 1.0, Some(dim)).unwrap();
    let temp = 60.0;
    let id1 = p.heat(id0, temp, 1.0).unwrap();

    let droplets = info_dict(&p);

    // we expect it to be just above the heater, since the runtime will move it
    // off the heater when placing the flush command
    let expected_loc = yx(2, 2);

    assert_eq!(droplets.len(), 1);
    assert_eq!(droplets[&id1].location, expected_loc);
}

#[test]
#[ignore = "We don't support combine into yet"]
fn combine_into() {
    let man = manager_from_rect(10, 10);
    let p = man.get_new_process("test");

    let loc_a = yx(2, 0);
    let a = p.create(Some(loc_a), 1.0, None).unwrap();
    let loc_b = yx(2, 9);
    let b = p.create(Some(loc_b), 1.0, None).unwrap();

    let loc_c = yx(9, 0);
    let c = p.create(Some(loc_c), 1.0, None).unwrap();
    let loc_d = yx(9, 9);
    let d = p.create(Some(loc_d), 1.0, None).unwrap();

    let ab = p.combine_into(a, b).unwrap();
    let cd = p.combine_into(d, c).unwrap();

    let droplets = info_dict(&p);
    let y1 = yx(1, 0);
    assert_eq!(droplets[&ab].location, loc_a - y1);
    assert_eq!(droplets[&cd].location, loc_d - y1);
}
