#[derive(Clone, PartialEq)]
#[allow(dead_code)]
pub enum Priority {
    None,
    Energy,
    Crystal,
}

impl Priority {
    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            Priority::None => "Nearest",
            Priority::Energy => "Energy first",
            Priority::Crystal => "Crystal first",
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Config {
    pub num_scouts: usize,
    pub num_collectors: usize,
    pub priority: Priority,
    pub carry_capacity: u32,
    pub energy_min: u32,
    pub energy_max: u32,
    pub crystal_min: u32,
    pub crystal_max: u32,
}
