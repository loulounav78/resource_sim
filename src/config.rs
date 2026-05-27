pub const NUM_FIELDS: usize = 9;

#[derive(Clone, PartialEq)]
pub enum Priority {
    None,
    Energy,
    Crystal,
}

impl Priority {
    pub fn next(&self) -> Self {
        match self {
            Priority::None => Priority::Energy,
            Priority::Energy => Priority::Crystal,
            Priority::Crystal => Priority::None,
        }
    }
    pub fn prev(&self) -> Self {
        match self {
            Priority::None => Priority::Crystal,
            Priority::Energy => Priority::None,
            Priority::Crystal => Priority::Energy,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Priority::None => "Nearest (no preference)",
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

impl Default for Config {
    fn default() -> Self {
        Config {
            num_scouts: 3,
            num_collectors: 3,
            priority: Priority::None,
            carry_capacity: 1,
            energy_min: 50,
            energy_max: 200,
            crystal_min: 50,
            crystal_max: 200,
        }
    }
}

impl Config {
    pub fn field_label(idx: usize) -> &'static str {
        match idx {
            0 => "Scouts",
            1 => "Collectors",
            2 => "Collector priority",
            3 => "Carry capacity",
            4 => "Energy   min",
            5 => "Energy   max",
            6 => "Crystal  min",
            7 => "Crystal  max",
            8 => "[ Start Simulation ]",
            _ => "",
        }
    }

    pub fn field_value(&self, idx: usize) -> String {
        match idx {
            0 => format!("{:>3}", self.num_scouts),
            1 => format!("{:>3}", self.num_collectors),
            2 => self.priority.label().to_string(),
            3 => format!("{:>3} units", self.carry_capacity),
            4 => format!("{:>3} units", self.energy_min),
            5 => format!("{:>3} units", self.energy_max),
            6 => format!("{:>3} units", self.crystal_min),
            7 => format!("{:>3} units", self.crystal_max),
            8 => String::new(),
            _ => String::new(),
        }
    }

    pub fn adjust(&mut self, idx: usize, delta: i32) {
        match idx {
            0 => self.num_scouts = clamp(self.num_scouts as i32 + delta, 1, 10) as usize,
            1 => self.num_collectors = clamp(self.num_collectors as i32 + delta, 1, 10) as usize,
            2 => {
                self.priority = if delta > 0 {
                    self.priority.next()
                } else {
                    self.priority.prev()
                }
            }
            3 => self.carry_capacity = clamp(self.carry_capacity as i32 + delta, 1, 20) as u32,
            4 => {
                let max = self.energy_max as i32 - 10;
                self.energy_min = clamp(self.energy_min as i32 + delta * 10, 10, max) as u32;
            }
            5 => {
                let min = self.energy_min as i32 + 10;
                self.energy_max = clamp(self.energy_max as i32 + delta * 10, min, 500) as u32;
            }
            6 => {
                let max = self.crystal_max as i32 - 10;
                self.crystal_min = clamp(self.crystal_min as i32 + delta * 10, 10, max) as u32;
            }
            7 => {
                let min = self.crystal_min as i32 + 10;
                self.crystal_max = clamp(self.crystal_max as i32 + delta * 10, min, 500) as u32;
            }
            _ => {}
        }
    }
}

fn clamp(v: i32, lo: i32, hi: i32) -> i32 {
    v.max(lo).min(hi)
}
