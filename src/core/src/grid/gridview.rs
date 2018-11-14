use grid::{Location, Droplet, DropletId, Grid, Electrode,
           Blob,
};
use util::collections::{Map, Set};
use plan::place::Placement;

#[derive(Default)]
pub struct GridView {
    pub grid: Grid,
    pub droplets: Map<DropletId, Droplet>,
}

impl GridView {
    pub fn new(grid: Grid) -> GridView {
        GridView {
            grid,
            ..GridView::default()
        }
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

    pub fn subview<'a>(
        &'a mut self,
        ids: impl IntoIterator<Item = &'a DropletId>,
        placement: &'a Placement
    ) -> GridSubView<'a> {
        GridSubView {
            backing_gridview: self,
            placement,
            ids: ids.into_iter().cloned().collect(),
        }
    }
}

pub struct GridSubView<'a> {
    backing_gridview: &'a mut GridView,
    placement: &'a Placement,
    ids: Set<DropletId>,
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
        assert!(self.ids.contains(&id));
        &self.backing_gridview.droplets[id]
    }

    fn get_mut(&mut self, id: &DropletId) -> &mut Droplet {
        assert!(self.ids.contains(&id));
        self.backing_gridview
            .droplets
            .get_mut(id)
            .unwrap()
    }

    pub fn insert(&mut self, mut droplet: Droplet) {
        let new_loc = self.placement.mapping.get(&droplet.location);
        trace!("Inserting {:#?} at {:?}", droplet, new_loc);
        droplet.location = *new_loc.unwrap();
        let was_not_there = self.ids.insert(droplet.id);
        assert!(was_not_there);
        let droplets = &mut self.backing_gridview.droplets;
        let was_there = droplets.insert(droplet.id, droplet);
        assert!(was_there.is_none());
    }

    pub fn remove(&mut self, id: &DropletId) -> Droplet {
        let was_there = self.ids.remove(id);
        assert!(was_there);
        let droplets = &mut self.backing_gridview.droplets;
        let mut droplet = droplets.remove(id).unwrap();
        // TODO this is pretty slow
        let (unmapped_loc, _) = self
            .placement
            .mapping
            .iter()
            .find(|(_, &v)| v == droplet.location)
            .unwrap();
        droplet.location = *unmapped_loc;
        droplet
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
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use grid::parse::tests::parse_strings;

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
        let ids = &[c2id('b')];
        let mut sub = gv.subview(ids, &placement);

        sub.move_north(c2id('b'));
        let _c = sub.get(&c2id('b'));

        // TODO this is a weak test because we don't really do anything
    }

    #[test]
    #[should_panic]
    fn test_subview_check() {
        let mut gv = parse_gridview(&["a.b.c"]);

        // placement only contains b
        let placement = placement_rect((0, 1), (0, 2));
        let ids = &[c2id('b')];
        let mut sub = gv.subview(ids, &placement);

        sub.get(&c2id('a'));
    }

}
