use std::collections::{HashMap, HashSet};

extern crate puddle_core;

extern crate crossbeam;

extern crate env_logger;

use puddle_core::*;

fn manager_from_rect<'a>(rows: usize, cols: usize) -> Manager {
    manager_from_rect_with_error(rows, cols, 0.0)
}

fn manager_from_rect_with_error<'a>(rows: usize, cols: usize, split_err: f64) -> Manager {
    let grid = Grid::rectangle(rows, cols);
    let err_opts = ErrorOptions {
        split_error_stdev: split_err,
    };
    let man = Manager::new(false, grid, err_opts);
    let _ = env_logger::try_init();
    man
}

fn info_dict(p: &ProcessHandle) -> HashMap<DropletId, DropletInfo> {
    p.flush().unwrap().into_iter().map(|d| (d.id, d)).collect()
}

fn float_epsilon_equal(float1: f64, float2: f64) -> bool {
    let epsilon = 0.00001f64;
    (float1 - float2).abs() < epsilon
}

#[test]
fn input_some_droplets() {
    let man = manager_from_rect(1, 4);
    let p = man.get_new_process("test");

    let loc = Location { y: 0, x: 0 };
    let id = p.input(Some(loc), 1.0).unwrap();

    let should_work = p.input(None, 1.0);
    let should_not_work = p.input(None, 1.0);

    assert!(should_work.is_ok());
    assert!(should_not_work.is_err());

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

    let loc1 = Location { y: 0, x: 0 };
    let loc2 = Location { y: 0, x: 3 };
    let id1 = p.input(Some(loc1), 1.0).unwrap();
    let id2 = p.move_droplet(id1, loc2).unwrap();

    let droplets = info_dict(&p);

    assert_eq!(droplets.len(), 1);
    assert_eq!(droplets[&id2].location, loc2);
    assert!(float_epsilon_equal(droplets[&id2].volume, 1.0));
}

#[test]
fn mix3() {
    let man = manager_from_rect(9, 9);
    let p = man.get_new_process("test");

    let id1 = p.input(None, 1.0).unwrap();
    let id2 = p.input(None, 1.0).unwrap();
    let id3 = p.input(None, 1.0).unwrap();

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

    let id1 = p.input(None, 1.0).unwrap();
    let id2 = p.input(None, 1.0).unwrap();

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
fn split_with_error() {
    let man = manager_from_rect_with_error(5, 5, 0.1);
    let p = man.get_new_process("test");

    let id0 = p.input(None, 1.0).unwrap();
    let (id1, id2) = p.split(id0).unwrap();

    let droplets = info_dict(&p);

    // there is basically 0 chance that an error did not occur
    assert_ne!(droplets[&id1].volume, droplets[&id2].volume);
}

#[test]
fn process_isolation() {
    // Spawn 6 processes
    let num_processes = 6;

    let manager = manager_from_rect(9, 9);
    let ps = (0..num_processes).map(|i| manager.get_new_process(format!("test-{}", i)));

    crossbeam::scope(|scope| {
        for p in ps {
            scope.spawn(move || {
                let _drop_id = p.input(None, 1.0).unwrap();
                p.flush().unwrap();
            });
        }
    });
}

#[test]
#[should_panic(expected = "PlanError(PlaceError)")]
fn grid_size_3() {
    let man = manager_from_rect(3, 3);
    let p = man.get_new_process("test");

    let _id1 = p.input(None, 1.0).unwrap();
    let _id2 = p.input(None, 1.0).unwrap();
}
