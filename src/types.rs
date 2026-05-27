pub type Pos = (usize, usize);

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
