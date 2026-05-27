use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use rand::Rng;

use crate::message::Message;
use crate::pathfinding;
use crate::simulation::SimState;
use crate::types::{Pos, ResourceKind};

const SCOUT_TICK_MS: u64 = 160;
const COLLECTOR_TICK_MS: u64 = 120;
const VISION_RANGE: i32 = 4;

/// Processes messages from robots and updates the shared base knowledge and stats.
pub fn base_processor(rx: mpsc::Receiver<Message>, state: Arc<Mutex<SimState>>) {
    for msg in rx {
        let mut s = state.lock().unwrap();
        match msg {
            Message::ResourceDiscovered { pos, kind, quantity } => {
                s.knowledge.resources.entry(pos).or_insert((kind, quantity));
            }
            Message::ResourceCollected { pos, kind, amount } => {
                match kind {
                    ResourceKind::Energy => s.collected_energy += amount,
                    ResourceKind::Crystal => s.collected_crystals += amount,
                }
                if let Some(entry) = s.knowledge.resources.get_mut(&pos) {
                    if entry.1 <= amount {
                        s.knowledge.resources.remove(&pos);
                    } else {
                        entry.1 -= amount;
                    }
                }
            }
            Message::ResourceDepleted { pos } => {
                s.knowledge.resources.remove(&pos);
            }
        }
    }
}

/// Scout robot: explores randomly, senses nearby resources and broadcasts discoveries.
pub fn run_scout(
    robot_idx: usize,
    start: Pos,
    tx: mpsc::Sender<Message>,
    state: Arc<Mutex<SimState>>,
) {
    let mut pos = start;
    let mut rng = rand::thread_rng();

    loop {
        thread::sleep(Duration::from_millis(SCOUT_TICK_MS));

        let mut discoveries: Vec<Message> = Vec::new();
        let next_pos;

        {
            let s = state.lock().unwrap();

            // Sense nearby tiles and report unknown resources
            for dy in -VISION_RANGE..=VISION_RANGE {
                for dx in -VISION_RANGE..=VISION_RANGE {
                    let nx = pos.0 as i32 + dx;
                    let ny = pos.1 as i32 + dy;
                    if nx < 0 || ny < 0 {
                        continue;
                    }
                    let np = (nx as usize, ny as usize);
                    if let Some(res) = s.resources.get(&np) {
                        if !s.knowledge.resources.contains_key(&np) {
                            discoveries.push(Message::ResourceDiscovered {
                                pos: np,
                                kind: res.kind.clone(),
                                quantity: res.quantity,
                            });
                        }
                    }
                }
            }

            // Choose a random passable neighbor to move to
            let neighbors = pathfinding::passable_neighbors(&s.map, pos);
            next_pos = if neighbors.is_empty() {
                pos
            } else {
                neighbors[rng.gen_range(0..neighbors.len())]
            };
        }

        // Broadcast discoveries outside the lock
        for msg in discoveries {
            let _ = tx.send(msg);
        }

        pos = next_pos;

        // Update display position
        {
            let mut s = state.lock().unwrap();
            if robot_idx < s.robots.len() {
                s.robots[robot_idx].x = pos.0;
                s.robots[robot_idx].y = pos.1;
            }
        }
    }
}

/// Collector robot: navigates to known resources, collects one unit, returns to base, unloads.
pub fn run_collector(
    robot_idx: usize,
    start: Pos,
    tx: mpsc::Sender<Message>,
    state: Arc<Mutex<SimState>>,
) {
    let mut pos = start;
    let mut carrying: Option<(Pos, ResourceKind)> = None;
    let mut target: Option<Pos> = None;
    let mut path: Vec<Pos> = Vec::new();
    let mut rng = rand::thread_rng();

    loop {
        thread::sleep(Duration::from_millis(COLLECTOR_TICK_MS));

        let mut pending_msgs: Vec<Message> = Vec::new();

        {
            let mut s = state.lock().unwrap();
            let base_pos = s.map.base_pos();

            if carrying.is_some() {
                // --- RETURN TO BASE ---
                if s.map.is_base(pos.0, pos.1) {
                    // Unload at base
                    let (rpos, kind) = carrying.take().unwrap();
                    pending_msgs.push(Message::ResourceCollected {
                        pos: rpos,
                        kind,
                        amount: 1,
                    });
                    s.robots[robot_idx].carrying = false;
                    path.clear();
                    target = None;
                } else {
                    // Navigate toward base
                    if path.is_empty() {
                        path = pathfinding::bfs(&s.map, pos, base_pos).unwrap_or_default();
                    }
                    if let Some(next) = path.first().copied() {
                        path.remove(0);
                        pos = next;
                    }
                }
            } else {
                // --- COLLECT OR NAVIGATE ---

                // Check if standing on a resource
                let on_resource = s.resources.contains_key(&pos);

                if on_resource {
                    // Collect one unit
                    let (kind, remaining) = {
                        let res = s.resources.get_mut(&pos).unwrap();
                        res.quantity -= 1;
                        (res.kind.clone(), res.quantity)
                    };
                    if remaining == 0 {
                        s.resources.remove(&pos);
                        pending_msgs.push(Message::ResourceDepleted { pos });
                    }
                    carrying = Some((pos, kind));
                    s.robots[robot_idx].carrying = true;
                    path.clear();
                    target = None;
                } else if let Some(t) = target {
                    // Navigate to current target
                    if pos == t {
                        // Arrived but resource is gone — retarget
                        target = None;
                        path.clear();
                    } else {
                        if path.is_empty() {
                            match pathfinding::bfs(&s.map, pos, t) {
                                Some(p) => path = p,
                                None => {
                                    target = None;
                                }
                            }
                        }
                        if let Some(next) = path.first().copied() {
                            path.remove(0);
                            pos = next;
                        }
                    }
                } else {
                    // Find the nearest known resource that still exists on the map
                    let best = s
                        .knowledge
                        .resources
                        .keys()
                        .filter(|&&p| s.resources.contains_key(&p))
                        .min_by_key(|&&p| {
                            let dx = pos.0 as i32 - p.0 as i32;
                            let dy = pos.1 as i32 - p.1 as i32;
                            dx.abs() + dy.abs()
                        })
                        .copied();

                    if let Some(t) = best {
                        target = Some(t);
                        path = pathfinding::bfs(&s.map, pos, t).unwrap_or_default();
                    } else {
                        // No known resources — wander like a scout
                        let neighbors = pathfinding::passable_neighbors(&s.map, pos);
                        if !neighbors.is_empty() {
                            pos = neighbors[rng.gen_range(0..neighbors.len())];
                        }
                    }
                }
            }

            // Update display
            if robot_idx < s.robots.len() {
                s.robots[robot_idx].x = pos.0;
                s.robots[robot_idx].y = pos.1;
            }
        }

        // Send messages outside the lock
        for msg in pending_msgs {
            let _ = tx.send(msg);
        }
    }
}
