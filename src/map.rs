use noise::{NoiseFn, Perlin};
use rand::Rng;
use std::collections::HashMap;

use crate::types::{Pos, Resource, ResourceKind};

#[derive(Clone, PartialEq)]
pub enum Tile {
    Empty,
    Obstacle,
    Base,
}

pub struct Map {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Vec<Tile>>,
}

impl Map {
    pub fn generate(width: usize, height: usize, seed: u32) -> (Self, HashMap<Pos, Resource>) {
        let perlin = Perlin::new(seed);
        let mut tiles = vec![vec![Tile::Empty; width]; height];

        for y in 0..height {
            for x in 0..width {
                let nx = x as f64 / width as f64 * 6.0;
                let ny = y as f64 / height as f64 * 6.0;
                if perlin.get([nx, ny]) > 0.22 {
                    tiles[y][x] = Tile::Obstacle;
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

        let mut rng = rand::thread_rng();
        let mut resources: HashMap<Pos, Resource> = HashMap::new();
        let target = (width * height) / 35;
        let mut attempts = 0;

        while resources.len() < target && attempts < 100_000 {
            let x = rng.gen_range(0..width);
            let y = rng.gen_range(0..height);
            if tiles[y][x] == Tile::Empty && !resources.contains_key(&(x, y)) {
                let kind = if rng.gen_bool(0.5) {
                    ResourceKind::Energy
                } else {
                    ResourceKind::Crystal
                };
                let quantity = rng.gen_range(50u32..=200);
                resources.insert((x, y), Resource { kind, quantity });
            }
            attempts += 1;
        }

        (Map { width, height, tiles }, resources)
    }

    pub fn is_passable(&self, x: usize, y: usize) -> bool {
        x < self.width && y < self.height && self.tiles[y][x] != Tile::Obstacle
    }

    pub fn is_base(&self, x: usize, y: usize) -> bool {
        x < self.width && y < self.height && self.tiles[y][x] == Tile::Base
    }

    pub fn base_pos(&self) -> Pos {
        (self.width / 2, self.height / 2)
    }
}
