use noise::{NoiseFn, Perlin};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashMap;

use crate::types::{Pos, Resource, ResourceKind};

pub const WALL_MAX_DURABILITY: u8 = 3;

/// Disperse une seed (SplitMix64) avant de l'utiliser : la table de
/// permutation de `noise::Perlin` melange mal les petites valeurs brutes
/// (1, 2, 3...), ce qui produisait des terrains quasi identiques entre
/// seeds voisines. Ce mixage garantit un terrain bien distinct meme pour
/// une seed saisie a la main sur un seul chiffre.
fn mix_seed(seed: u32) -> u64 {
    let mut z = (seed as u64).wrapping_add(0x9E3779B97F4A7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

#[derive(Clone, Copy, PartialEq)]
pub enum Tile {
    Empty,
    Obstacle(u8),
    Base,
}

impl Tile {
    pub fn is_obstacle(&self) -> bool {
        matches!(self, Tile::Obstacle(_))
    }
}

pub struct Map {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Vec<Tile>>,
}

impl Map {
    pub fn generate(
        width: usize,
        height: usize,
        seed: u32,
        energy_range: (u32, u32),
        crystal_range: (u32, u32),
    ) -> (Self, HashMap<Pos, Resource>) {
        let mixed = mix_seed(seed);
        let perlin = Perlin::new(mixed as u32);
        let mut tiles = vec![vec![Tile::Empty; width]; height];

        for y in 0..height {
            for x in 0..width {
                let nx = x as f64 / width as f64 * 6.0;
                let ny = y as f64 / height as f64 * 6.0;
                if perlin.get([nx, ny]) > 0.22 {
                    tiles[y][x] = Tile::Obstacle(WALL_MAX_DURABILITY);
                }
            }
        }

        // Clear a 5x5 area around base center
        let bx = width / 2;
        let by = height / 2;
        for dy in -2i32..=2 {
            for dx in -2i32..=2 {
                let nx = bx as i32 + dx;
                let ny = by as i32 + dy;
                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                    tiles[ny as usize][nx as usize] = Tile::Base;
                }
            }
        }

        // RNG des ressources derivee de la seed pour qu'une carte favorite
        // (terrain + ressources) soit reproductible a l'identique.
        let mut rng = StdRng::seed_from_u64(mixed ^ 0xA5A5_A5A5_A5A5_A5A5);
        let mut resources: HashMap<Pos, Resource> = HashMap::new();
        let target = (width * height) / 35;
        let mut attempts = 0;

        while resources.len() < target && attempts < 100_000 {
            let x = rng.gen_range(0..width);
            let y = rng.gen_range(0..height);
            if tiles[y][x] == Tile::Empty && !resources.contains_key(&(x, y)) {
                let (kind, quantity) = if rng.gen_bool(0.5) {
                    (
                        ResourceKind::Energy,
                        rng.gen_range(energy_range.0..=energy_range.1),
                    )
                } else {
                    (
                        ResourceKind::Crystal,
                        rng.gen_range(crystal_range.0..=crystal_range.1),
                    )
                };
                resources.insert((x, y), Resource { kind, quantity });
            }
            attempts += 1;
        }

        (
            Map {
                width,
                height,
                tiles,
            },
            resources,
        )
    }

    pub fn is_passable(&self, x: usize, y: usize) -> bool {
        x < self.width && y < self.height && !self.tiles[y][x].is_obstacle()
    }

    pub fn is_base(&self, x: usize, y: usize) -> bool {
        x < self.width && y < self.height && self.tiles[y][x] == Tile::Base
    }

    pub fn damage_wall(&mut self, x: usize, y: usize, damage: u8) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }

        match &mut self.tiles[y][x] {
            Tile::Obstacle(durability) => {
                if damage >= *durability {
                    self.tiles[y][x] = Tile::Empty;
                } else {
                    *durability -= damage;
                }
                true
            }
            _ => false,
        }
    }

    pub fn base_pos(&self) -> Pos {
        (self.width / 2, self.height / 2)
    }
}
