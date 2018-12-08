use crate::grid::{Droplet, DropletId, DropletInfo, Electrode, Grid, Location};
use crate::plan::place::Placement;
use crate::process::ProcessId;
use crate::util::collections::{Map, Set};

#[derive(Default, Clone)]
pub struct GridView {
    pub grid: Grid,
    pub droplets: Map<DropletId, Droplet>,
}

use std::fmt;
impl fmt::Debug for GridView {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("GridView")
            .field("grid", &"...hiding grid...")
            .field("droplets", &self.droplets)
            .finish()
    }
}


impl GridView {
    pub fn new(grid: Grid) -> GridView {
        GridView {
            grid,
            ..GridView::default()
        }
    }

    pub fn droplet_info(&self, pid_option: Option<ProcessId>) -> Vec<DropletInfo> {
        self.droplets
            .values()
            .filter(|&d| pid_option.map_or(true, |pid| d.id.process_id == pid))
            .map(|d| d.info())
            .collect()
    }

    /// Returns an invalid droplet, if any.
    fn get_collision(&self) -> Option<(i32, Droplet, Droplet)> {
        for (id1, droplet1) in &self.droplets {
            for (id2, droplet2) in &self.droplets {
                if id1 == id2 {
                    continue;
                }
                if droplet1.collision_group == droplet2.collision_group {
                    continue;
                }
                let distance = droplet1.collision_distance(droplet2);
                if distance <= 0 {
                    return Some((distance, droplet1.clone(), droplet2.clone()));
                }
            }
        }
        None
    }

    pub fn check_no_collision(&self) {
        if let Some((_distance, d1, d2)) = self.get_collision() {
            panic!("Collision!!!!! between {:#?} and {:#?}", d1, d2)
        }
    }

    pub fn subview<'a>(&'a mut self, placement: &'a Placement) -> GridSubView<'a> {
        GridSubView {
            backing_gridview: self,
            placement,
        }
    }
}

pub struct GridSubView<'a> {
    backing_gridview: &'a mut GridView,
    placement: &'a Placement,
}

impl<'a> GridSubView<'a> {
    // #[cfg(feature = "pi")]
    // pub fn with_pi<T>(&mut self, f: impl FnOnce(&mut RaspberryPi) -> T) -> Option<T> {
    //     self.backing_gridview.pi.as_mut().map(f)
    // }

    pub fn get_electrode(&self, loc: &Location) -> Option<&Electrode> {
        let actual_loc = self.placement.mapping.get(loc)?;
        self.backing_gridview.grid.get_cell(&actual_loc)
    }

    // TODO: translate or somehow hide the untranslated location of this
    pub fn get(&self, id: &DropletId) -> &Droplet {
        // assert!(self.ids.contains(&id));
        // TODO we can at least assert that this thing is in the placed locations
        &self.backing_gridview.droplets[id]
    }

    fn get_mut(&mut self, id: &DropletId) -> &mut Droplet {
        // assert!(self.ids.contains(&id));
        self.backing_gridview.droplets.get_mut(id).unwrap()
    }

    pub fn insert(&mut self, mut droplet: Droplet) {
        let id = droplet.id;
        let new_loc = self.placement.mapping.get(&droplet.location);
        trace!("Inserting {:#?} at {:?}", droplet, new_loc);
        droplet.location = *new_loc.unwrap();
        // let was_not_there = self.ids.insert(droplet.id);
        // assert!(was_not_there);
        let droplets = &mut self.backing_gridview.droplets;
        let was_there = droplets.insert(id, droplet);
        if let Some(was_there) = was_there {
            let droplet = &droplets[&id];
            panic!(
                "Something was here! While inserting {:#?}, I found {:#?}",
                droplet, was_there
            );
        }
    }

    pub fn remove(&mut self, id: &DropletId) -> Droplet {
        // let was_there = self.ids.remove(id);
        // assert!(was_there);
        let droplets = &mut self.backing_gridview.droplets;
        let mut droplet = droplets.remove(id).unwrap();
        // TODO this is pretty slow
        let find_unmapped = self
            .placement
            .mapping
            .iter()
            .find(|(_, &v)| v == droplet.location);

        if let Some((unmapped_loc, _)) = find_unmapped {
            droplet.location = *unmapped_loc;
            droplet
        } else {
            panic!(
                "Droplet {:?} was not in mapping. Location: {}, Mapping: {:#?}",
                id, droplet.location, self.placement
            );
        }
    }

    fn check_droplet(&self, id: &DropletId) {
        // TODO will this have translated or real location??
        let droplet = self.get(id);
        let mapped_to: Set<_> = self.placement.mapping.values().collect();
        // TODO this is pretty slow
        for i in 0..droplet.dimensions.y {
            for j in 0..droplet.dimensions.x {
                let loc = Location {
                    y: droplet.location.y + i,
                    x: droplet.location.x + j,
                };
                if !mapped_to.contains(&loc) {
                    panic!("{} was unmapped!, placement: {:#?}", loc, self.placement);
                }
            }
        }
    }

    fn update(&mut self, id: &DropletId, func: impl FnOnce(&mut Droplet)) {
        func(self.get_mut(id));
        self.check_droplet(id);
    }

    pub fn move_west(&mut self, id: DropletId) {
        trace!("Moving droplet {:?} west", id);
        self.update(&id, |droplet| {
            droplet.location = droplet.location.west();
        })
    }

    pub fn move_east(&mut self, id: DropletId) {
        trace!("Moving droplet {:?} east", id);
        self.update(&id, |droplet| {
            droplet.location = droplet.location.east();
        })
    }

    pub fn move_north(&mut self, id: DropletId) {
        trace!("Moving droplet {:?} north", id);
        self.update(&id, |droplet| {
            droplet.location = droplet.location.north();
        })
    }

    pub fn move_south(&mut self, id: DropletId) {
        trace!("Moving droplet {:?} south", id);
        self.update(&id, |droplet| {
            droplet.location = droplet.location.south();
        })
    }

    pub fn droplet_info(&self, pid_option: Option<ProcessId>) -> Vec<DropletInfo> {
        self.backing_gridview.droplet_info(pid_option)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::grid::droplet::Blob;
    use crate::grid::parse::tests::parse_strings;

    pub fn id2c(id: &DropletId) -> char {
        assert!(id.id < 255);
        (id.id as u8) as char
    }

    pub fn c2id(c: char) -> DropletId {
        for u in 0x00u8..0xff {
            let c2 = u as char;
            if c == c2 {
                return DropletId {
                    id: u as usize,
                    process_id: 0,
                };
            }
        }
        panic!("Can't make {} a u8", c);
    }

    pub fn parse_gridview(strs: &[&str]) -> GridView {
        // this commonly used function will start logging
        let _ = env_logger::try_init();

        // same chars are guaranteed to have the same ids

        let (grid, blobs) = parse_strings(&strs);
        let mut gv = GridView::new(grid);

        for (ch, blob) in blobs.iter() {
            let id = c2id(*ch);
            gv.droplets.insert(id, blob.to_droplet(id));
        }

        gv
    }

    fn placement_rect(offset: impl Into<Location>, size: impl Into<Location>) -> Placement {
        let mut mapping = Map::new();
        let offset = offset.into();
        let size = size.into();
        for y in 0..size.y {
            for x in 0..size.x {
                let loc = Location {
                    y: offset.y + y,
                    x: offset.x + x,
                };
                mapping.insert(loc, loc);
            }
        }
        Placement { mapping }
    }

    #[test]
    fn test_subview_ok() {
        let mut gv = parse_gridview(&[
            "aa..........c",
            ".....bb......",
            ".............",
            ".............",
        ]);

        let placement = placement_rect((0, 2), (4, 7));
        // let ids: Vec<_> = gv.droplets.keys().cloned().collect();
        let mut sub = gv.subview(&placement);

        sub.move_north(c2id('b'));
        let _c = sub.get(&c2id('b'));

        // TODO this is a weak test because we don't really do anything
    }

    #[test]
    #[should_panic]
    fn test_subview_check() {
        let mut gv = parse_gridview(&["a...b...c"]);

        // placement only contains b
        let placement = placement_rect((0, 4), (0, 2));
        let mut sub = gv.subview(&placement);

        // try to move b to an invalid location outside the placement
        sub.update(&c2id('b'), |b| b.location = (0, 2).into())
    }

}
