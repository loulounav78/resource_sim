pub type Pos = (usize, usize);

#[derive(Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn delta(self) -> (i32, i32) {
        match self {
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum ResourceKind {
    Energy,
    Crystal,
}

#[derive(Clone)]
pub struct Resource {
    pub kind: ResourceKind,
    pub quantity: u32,
}
