use std::collections::{HashMap, VecDeque};

use crate::map::Map;
use crate::types::Pos;

const DIRS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

pub fn bfs(map: &Map, start: Pos, goal: Pos) -> Option<Vec<Pos>> {
    if start == goal {
        return Some(vec![]);
    }

    let mut visited: HashMap<Pos, Pos> = HashMap::new();
    let mut queue = VecDeque::new();

    visited.insert(start, start);
    queue.push_back(start);

    while let Some(cur) = queue.pop_front() {
        if cur == goal {
            let mut path = Vec::new();
            let mut node = cur;
            while node != start {
                path.push(node);
                node = visited[&node];
            }
            path.reverse();
            return Some(path);
        }

        for (dx, dy) in DIRS {
            let nx = cur.0 as i32 + dx;
            let ny = cur.1 as i32 + dy;
            if nx < 0 || ny < 0 {
                continue;
            }
            let next = (nx as usize, ny as usize);
            if !visited.contains_key(&next) && (map.is_passable(next.0, next.1) || next == goal) {
                visited.insert(next, cur);
                queue.push_back(next);
            }
        }
    }

    None
}

pub fn passable_neighbors(map: &Map, pos: Pos) -> Vec<Pos> {
    let mut result = Vec::new();
    for (dx, dy) in DIRS {
        let nx = pos.0 as i32 + dx;
        let ny = pos.1 as i32 + dy;
        if nx >= 0 && ny >= 0 {
            let next = (nx as usize, ny as usize);
            if map.is_passable(next.0, next.1) {
                result.push(next);
            }
        }
    }
    result
}
