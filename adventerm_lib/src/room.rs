use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoomId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DoorId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TileKind {
    Wall,
    Floor,
    Door(DoorId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Room {
    pub id: RoomId,
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<TileKind>,
}

impl Room {
    pub fn new_filled(id: RoomId, width: usize, height: usize, fill: TileKind) -> Self {
        Self {
            id,
            width,
            height,
            tiles: vec![fill; width * height],
        }
    }

    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    pub fn in_bounds(&self, x: isize, y: isize) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height
    }

    pub fn kind_at(&self, x: usize, y: usize) -> Option<TileKind> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(self.tiles[self.idx(x, y)])
    }

    pub fn set(&mut self, x: usize, y: usize, kind: TileKind) {
        let i = self.idx(x, y);
        self.tiles[i] = kind;
    }

    pub fn is_walkable(&self, x: usize, y: usize) -> bool {
        match self.kind_at(x, y) {
            Some(TileKind::Floor) | Some(TileKind::Door(_)) => true,
            _ => false,
        }
    }

    pub fn doors(&self) -> impl Iterator<Item = (usize, usize, DoorId)> + '_ {
        self.tiles.iter().enumerate().filter_map(|(i, t)| {
            if let TileKind::Door(d) = t {
                Some((i % self.width, i / self.width, *d))
            } else {
                None
            }
        })
    }

    pub fn find_door(&self, id: DoorId) -> Option<(usize, usize)> {
        self.doors()
            .find(|(_, _, d)| *d == id)
            .map(|(x, y, _)| (x, y))
    }

    pub fn first_floor(&self) -> Option<(usize, usize)> {
        for y in 0..self.height {
            for x in 0..self.width {
                if matches!(self.kind_at(x, y), Some(TileKind::Floor)) {
                    return Some((x, y));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walkable_includes_doors() {
        let mut r = Room::new_filled(RoomId(0), 3, 3, TileKind::Wall);
        r.set(1, 1, TileKind::Floor);
        r.set(2, 1, TileKind::Door(DoorId(7)));
        assert!(r.is_walkable(1, 1));
        assert!(r.is_walkable(2, 1));
        assert!(!r.is_walkable(0, 0));
    }

    #[test]
    fn find_door_returns_position() {
        let mut r = Room::new_filled(RoomId(0), 4, 4, TileKind::Floor);
        r.set(3, 2, TileKind::Door(DoorId(9)));
        assert_eq!(r.find_door(DoorId(9)), Some((3, 2)));
        assert_eq!(r.find_door(DoorId(0)), None);
    }
}
