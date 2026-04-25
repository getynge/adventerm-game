#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Ground,
    Player,
}

#[derive(Debug, Clone)]
pub struct World {
    width: usize,
    height: usize,
    player: (usize, usize),
}

impl World {
    pub const DEFAULT_WIDTH: usize = 11;
    pub const DEFAULT_HEIGHT: usize = 7;

    pub fn new() -> Self {
        Self::with_size(Self::DEFAULT_WIDTH, Self::DEFAULT_HEIGHT)
    }

    pub fn with_size(width: usize, height: usize) -> Self {
        let player = (width / 2, height / 2);
        Self {
            width,
            height,
            player,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn player(&self) -> (usize, usize) {
        self.player
    }

    pub fn tile_at(&self, x: usize, y: usize) -> Tile {
        if (x, y) == self.player {
            Tile::Player
        } else {
            Tile::Ground
        }
    }

    pub fn move_player(&mut self, direction: Direction) {
        let (x, y) = self.player;
        let (nx, ny) = match direction {
            Direction::Up => (x as isize, y as isize - 1),
            Direction::Down => (x as isize, y as isize + 1),
            Direction::Left => (x as isize - 1, y as isize),
            Direction::Right => (x as isize + 1, y as isize),
        };
        if nx >= 0 && ny >= 0 && (nx as usize) < self.width && (ny as usize) < self.height {
            self.player = (nx as usize, ny as usize);
        }
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_starts_centered() {
        let world = World::new();
        assert_eq!(
            world.player(),
            (World::DEFAULT_WIDTH / 2, World::DEFAULT_HEIGHT / 2)
        );
        assert_eq!(
            world.tile_at(World::DEFAULT_WIDTH / 2, World::DEFAULT_HEIGHT / 2),
            Tile::Player
        );
    }

    #[test]
    fn movement_updates_position() {
        let mut world = World::new();
        let (x, y) = world.player();
        world.move_player(Direction::Up);
        assert_eq!(world.player(), (x, y - 1));
    }

    #[test]
    fn movement_blocked_at_bounds() {
        let mut world = World::with_size(3, 3);
        for _ in 0..5 {
            world.move_player(Direction::Up);
        }
        assert_eq!(world.player().1, 0);
        for _ in 0..5 {
            world.move_player(Direction::Left);
        }
        assert_eq!(world.player(), (0, 0));
    }
}
