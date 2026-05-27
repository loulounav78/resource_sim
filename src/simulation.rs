use std::collections::{HashMap, HashSet};

use crate::map::Map;
use crate::types::{Pos, Resource, ResourceKind};

#[derive(Clone, PartialEq)]
pub enum RobotKind {
    Scout,
    Collector,
}

#[derive(Clone)]
pub struct RobotDisplay {
    pub x: usize,
    pub y: usize,
    pub kind: RobotKind,
    pub carrying: bool,
}

pub struct BaseKnowledge {
    /// Resources known to the base (pos → (kind, last known quantity))
    pub resources: HashMap<Pos, (ResourceKind, u32)>,
}

impl BaseKnowledge {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }
}

pub struct SimState {
    pub map: Map,
    pub resources: HashMap<Pos, Resource>,
    pub knowledge: BaseKnowledge,
    pub robots: Vec<RobotDisplay>,
    pub collected_energy: u32,
    pub collected_crystals: u32,
    pub reserved: HashSet<Pos>,
}

impl SimState {
    pub fn new(map: Map, resources: HashMap<Pos, Resource>) -> Self {
        Self {
            map,
            resources,
            knowledge: BaseKnowledge::new(),
            robots: Vec::new(),
            collected_energy: 0,
            collected_crystals: 0,
            reserved: HashSet::new(),
        }
    }
}
