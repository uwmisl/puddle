use std::collections::HashMap;

extern crate puddle_core;

extern crate env_logger;

use puddle_core::{DropletId, DropletInfo, Grid, Location, Manager, ProcessHandle};

fn manager_from_rect<'a>(rows: usize, cols: usize) -> Manager {
    let grid = Grid::rectangle(rows, cols);
    let man = Manager::new(false, grid);
    let _ = env_logger::try_init();
    man
}

fn info_dict(p: &ProcessHandle) -> HashMap<DropletId, DropletInfo> {
    p.droplet_info()
        .unwrap()
        .into_iter()
        .map(|d| (d.id, d))
        .collect()
}

#[test]
fn input_some_droplets() {
    let mut man = manager_from_rect(1, 4);
    let p = man.get_new_process("test");

    let loc = Location { y: 0, x: 0 };
    let id = p.input(Some(loc)).unwrap();

    let should_work = p.input(None);
    let should_not_work = p.input(None);

    assert!(should_work.is_ok());
    assert!(should_not_work.is_err());

    let droplets = info_dict(&p);

    assert_eq!(droplets.len(), 2);
    assert_eq!(droplets[&id].location, loc);

    p.flush().unwrap();
}

#[test]
fn move_droplet() {
    let mut man = manager_from_rect(1, 4);
    let p = man.get_new_process("test");

    let loc1 = Location { y: 0, x: 0 };
    let loc2 = Location { y: 0, x: 3 };
    let id1 = p.input(Some(loc1)).unwrap();
    let id2 = p.move_droplet(id1, loc2).unwrap();

    let droplets = info_dict(&p);

    assert_eq!(droplets.len(), 1);
    assert_eq!(droplets[&id2].location, loc2);
}

#[test]
fn mix3() {
    let mut man = manager_from_rect(9, 9);
    let p = man.get_new_process("test");

    let id1 = p.input(None).unwrap();
    let id2 = p.input(None).unwrap();
    let id3 = p.input(None).unwrap();

    let id12 = p.mix(id1, id2).unwrap();
    let id123 = p.mix(id12, id3).unwrap();

    let droplets = info_dict(&p);

    assert_eq!(droplets.len(), 1);
    assert!(droplets.contains_key(&id123));
}

#[test]
fn mix_split() {
    let mut man = manager_from_rect(9, 9);
    let p = man.get_new_process("test");

    let id1 = p.input(None).unwrap();
    let id2 = p.input(None).unwrap();

    let id12 = p.mix(id1, id2).unwrap();

    let (id3, id4) = p.split(id12).unwrap();

    let droplets = info_dict(&p);

    assert_eq!(droplets.len(), 2);
    assert!(droplets.contains_key(&id3));
    assert!(droplets.contains_key(&id4));
}
