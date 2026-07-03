use std::collections::{HashMap, HashSet};

use crate::map::Map;
use crate::types::{Direction, Pos, Resource, ResourceKind};

pub const SCOUT_RALLY_ENERGY_COST_PER_SCOUT: u32 = 20;

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

#[derive(Clone)]
pub struct PlayerDisplay {
    pub x: usize,
    pub y: usize,
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
    pub player: PlayerDisplay,
    pub scout_rally_pos: Option<Pos>,
    pub wall_break_power: u8,
    pub available_energy: u32,
    pub collected_energy: u32,
    pub collected_crystals: u32,
    pub reserved: HashSet<Pos>,
}

impl SimState {
    pub fn new(map: Map, resources: HashMap<Pos, Resource>, wall_break_power: u8) -> Self {
        let player_pos = player_spawn_pos(&map);
        let wall_break_power = wall_break_power.clamp(1, 3);
        Self {
            map,
            resources,
            knowledge: BaseKnowledge::new(),
            robots: Vec::new(),
            player: PlayerDisplay {
                x: player_pos.0,
                y: player_pos.1,
            },
            scout_rally_pos: None,
            wall_break_power,
            available_energy: 0,
            collected_energy: 0,
            collected_crystals: 0,
            reserved: HashSet::new(),
        }
    }

    pub fn player_pos(&self) -> Pos {
        (self.player.x, self.player.y)
    }

    pub fn robot_at(&self, pos: Pos) -> bool {
        self.robots.iter().any(|r| (r.x, r.y) == pos)
    }

    pub fn set_scout_rally_active(&mut self, active: bool) {
        if !active {
            self.scout_rally_pos = None;
            return;
        }

        let player_pos = self.player_pos();
        if self.scout_rally_pos.is_none() {
            let cost = self.scout_rally_energy_cost();
            if self.available_energy < cost {
                self.scout_rally_pos = None;
                return;
            }
            self.available_energy -= cost;
        }

        self.scout_rally_pos = Some(player_pos);
    }

    pub fn move_player(&mut self, direction: Direction) -> bool {
        let Some((nx, ny)) = self.offset_from(self.player_pos(), direction) else {
            return false;
        };

        if !self.map.is_passable(nx, ny) {
            return false;
        }

        if self.robot_at((nx, ny)) {
            return false;
        }

        self.player.x = nx;
        self.player.y = ny;
        if self.scout_rally_pos.is_some() {
            self.scout_rally_pos = Some((nx, ny));
        }
        true
    }

    pub fn damage_wall_from_player(&mut self, direction: Direction) -> bool {
        let Some((tx, ty)) = self.offset_from(self.player_pos(), direction) else {
            return false;
        };

        if self.wall_break_power == 0 {
            return false;
        }

        let cost = self.wall_break_energy_cost();
        if self.available_energy < cost {
            return false;
        }

        if self.map.damage_wall(tx, ty, self.wall_break_power) {
            self.available_energy -= cost;
            true
        } else {
            false
        }
    }

    pub fn wall_break_energy_cost(&self) -> u32 {
        wall_break_energy_cost(self.wall_break_power)
    }

    pub fn scout_rally_energy_cost(&self) -> u32 {
        SCOUT_RALLY_ENERGY_COST_PER_SCOUT * self.attractable_scout_count()
    }

    fn attractable_scout_count(&self) -> u32 {
        let player_pos = self.player_pos();
        self.robots
            .iter()
            .filter(|r| r.kind == RobotKind::Scout && (r.x, r.y) != player_pos)
            .count() as u32
    }

    fn offset_from(&self, pos: Pos, direction: Direction) -> Option<Pos> {
        let (dx, dy) = direction.delta();
        let nx = pos.0 as i32 + dx;
        let ny = pos.1 as i32 + dy;
        if nx < 0 || ny < 0 || nx >= self.map.width as i32 || ny >= self.map.height as i32 {
            None
        } else {
            Some((nx as usize, ny as usize))
        }
    }
}

pub fn wall_break_energy_cost(level: u8) -> u32 {
    match level {
        0 => 0,
        1 => 50,
        2 => 150,
        _ => 350,
    }
}

fn player_spawn_pos(map: &Map) -> Pos {
    let base = map.base_pos();
    let offsets = [
        (0, -2),
        (2, 0),
        (0, 2),
        (-2, 0),
        (1, -2),
        (2, -1),
        (2, 1),
        (1, 2),
        (-1, 2),
        (-2, 1),
        (-2, -1),
        (-1, -2),
    ];

    offsets
        .iter()
        .filter_map(|(dx, dy)| {
            let x = base.0 as i32 + dx;
            let y = base.1 as i32 + dy;
            if x < 0 || y < 0 {
                return None;
            }
            let pos = (x as usize, y as usize);
            map.is_passable(pos.0, pos.1).then_some(pos)
        })
        .next()
        .unwrap_or(base)
}
