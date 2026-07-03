use std::collections::{HashMap, HashSet};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use rand::Rng;

use crate::config::Priority;
use crate::message::Message;
use crate::pathfinding;
use crate::simulation::SimState;
use crate::types::{Pos, ResourceKind};

const SCOUT_TICK_MS: u64 = 160;
const COLLECTOR_TICK_MS: u64 = 120;
const VISION_RANGE: i32 = 4;

/// Processes messages from robots and updates the shared base knowledge and stats.
/// Exits naturally once all robot senders are dropped (robots have stopped).
pub fn base_processor(
    rx: mpsc::Receiver<Message>,
    state: Arc<Mutex<SimState>>,
    _running: Arc<AtomicBool>,
) {
    for msg in rx {
        let mut s = state.lock().unwrap();
        match msg {
            Message::ResourceDiscovered {
                pos,
                kind,
                quantity,
            } => {
                s.knowledge.resources.entry(pos).or_insert((kind, quantity));
            }
            Message::ResourceCollected { pos, kind, amount } => {
                match kind {
                    ResourceKind::Energy => {
                        s.collected_energy += amount;
                        s.available_energy += amount;
                    }
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

/// Scout: explores the map with a visit-count biased random walk.
/// Among passable free neighbors, those visited least often are strongly preferred;
/// ties are broken randomly. This preserves the "random exploration" character
/// while avoiding the O(n²) revisit problem of a pure random walk.
pub fn run_scout(
    robot_idx: usize,
    start: Pos,
    tx: mpsc::Sender<Message>,
    state: Arc<Mutex<SimState>>,
    running: Arc<AtomicBool>,
) {
    let mut pos = start;
    let mut rng = rand::thread_rng();
    // Local visit counts — private to this scout, no shared state needed.
    let mut visit_counts: HashMap<Pos, u32> = HashMap::new();
    *visit_counts.entry(start).or_insert(0) += 1;

    while running.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(SCOUT_TICK_MS));
        if !running.load(Ordering::Relaxed) {
            break;
        }

        let mut discoveries: Vec<Message> = Vec::new();

        {
            let mut s = state.lock().unwrap();

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

            let occupied = occupied_by_others(&s, robot_idx);

            if let Some(rally_pos) = s.scout_rally_pos {
                if pos != rally_pos {
                    let mut path = pathfinding::bfs(&s.map, pos, rally_pos).unwrap_or_default();
                    if let Some(next) = path.first().copied() {
                        if !occupied.contains(&next) {
                            pos = next;
                        } else {
                            step_aside(&s.map, &mut pos, &mut path, rally_pos, &occupied, &mut rng);
                        }
                    }
                }
            } else {
                let candidates: Vec<Pos> = pathfinding::passable_neighbors(&s.map, pos)
                    .into_iter()
                    .filter(|p| !occupied.contains(p))
                    .collect();

                pos = if candidates.is_empty() {
                    pos
                } else {
                    // Pick the candidate(s) with the fewest prior visits, then choose randomly among ties.
                    let min_visits = candidates
                        .iter()
                        .map(|p| *visit_counts.get(p).unwrap_or(&0))
                        .min()
                        .unwrap();
                    let least_visited: Vec<Pos> = candidates
                        .into_iter()
                        .filter(|p| *visit_counts.get(p).unwrap_or(&0) == min_visits)
                        .collect();
                    least_visited[rng.gen_range(0..least_visited.len())]
                };
            }

            *visit_counts.entry(pos).or_insert(0) += 1;

            if robot_idx < s.robots.len() {
                s.robots[robot_idx].x = pos.0;
                s.robots[robot_idx].y = pos.1;
            }
        }

        for msg in discoveries {
            let _ = tx.send(msg);
        }
    }
}

/// Collector: navigates to known resources, collects up to `carry_capacity` units,
/// then returns to base to unload. Respects `priority` when choosing targets.
pub fn run_collector(
    robot_idx: usize,
    start: Pos,
    tx: mpsc::Sender<Message>,
    state: Arc<Mutex<SimState>>,
    running: Arc<AtomicBool>,
    carry_capacity: u32,
    priority: Priority,
) {
    let mut pos = start;
    let mut target: Option<Pos> = None;
    let mut path: Vec<Pos> = Vec::new();

    // Collecting phase state
    let mut at_source = false;
    let mut returning = false;
    let mut carry_source: Pos = start;
    let mut carry_kind = ResourceKind::Energy;
    let mut carry_amount: u32 = 0;
    let mut my_claimed: Option<Pos> = None;

    let mut rng = rand::thread_rng();

    while running.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(COLLECTOR_TICK_MS));
        if !running.load(Ordering::Relaxed) {
            break;
        }

        let mut pending_msgs: Vec<Message> = Vec::new();

        {
            let mut s = state.lock().unwrap();
            let base_pos = s.map.base_pos();

            if returning {
                //── RETURN TO BASE ──────────────────────────────────────
                if s.map.is_base(pos.0, pos.1) {
                    pending_msgs.push(Message::ResourceCollected {
                        pos: carry_source,
                        kind: carry_kind.clone(),
                        amount: carry_amount,
                    });
                    returning = false;
                    at_source = false;
                    carry_amount = 0;
                    s.robots[robot_idx].carrying = false;
                    path.clear();
                    target = None;
                } else {
                    if path.is_empty() {
                        path = pathfinding::bfs(&s.map, pos, base_pos).unwrap_or_default();
                    }
                    if let Some(next) = path.first().copied() {
                        let occupied = occupied_by_others(&s, robot_idx);
                        if !occupied.contains(&next) {
                            path.remove(0);
                            pos = next;
                        } else {
                            step_aside(&s.map, &mut pos, &mut path, base_pos, &occupied, &mut rng);
                        }
                    }
                }
            } else if at_source {
                //── COLLECT AT CURRENT TILE ─────────────────────────────
                // Collect one unit (or detect that the resource is gone).
                let resource_depleted = if let Some(res) = s.resources.get_mut(&carry_source) {
                    res.quantity -= 1;
                    carry_amount += 1;
                    res.quantity == 0
                } else {
                    true // resource already gone (taken by another collector)
                };

                if resource_depleted {
                    s.resources.remove(&carry_source); // no-op if already absent
                    pending_msgs.push(Message::ResourceDepleted { pos: carry_source });
                }

                let should_return = resource_depleted || carry_amount >= carry_capacity;

                if should_return {
                    at_source = false;
                    path.clear();
                    if let Some(p) = my_claimed.take() {
                        s.reserved.remove(&p);
                    }
                    if carry_amount > 0 {
                        returning = true;
                        s.robots[robot_idx].carrying = true;
                    }
                }
                // else: stay at the tile, collect more next tick
            } else {
                //── SEEK A RESOURCE ─────────────────────────────────────
                if s.resources.contains_key(&pos)
                    && !s.map.is_base(pos.0, pos.1)
                    && (my_claimed == Some(pos) || !s.reserved.contains(&pos))
                {
                    // Start collecting here
                    carry_source = pos;
                    carry_kind = s.resources[&pos].kind.clone();
                    carry_amount = 0;
                    at_source = true;
                    target = None;
                    path.clear();
                    // Claim carry_source if not already claimed (wanderer case)
                    if my_claimed != Some(pos) {
                        if let Some(p) = my_claimed.take() {
                            s.reserved.remove(&p);
                        }
                        s.reserved.insert(pos);
                        my_claimed = Some(pos);
                    }
                } else if let Some(t) = target {
                    if pos == t {
                        target = None;
                        path.clear();
                    } else {
                        if path.is_empty() {
                            match pathfinding::bfs(&s.map, pos, t) {
                                Some(p) => path = p,
                                None => {
                                    if let Some(p) = my_claimed.take() {
                                        s.reserved.remove(&p);
                                    }
                                    target = None;
                                }
                            }
                        }
                        if let Some(next) = path.first().copied() {
                            let occupied = occupied_by_others(&s, robot_idx);
                            if !occupied.contains(&next) {
                                path.remove(0);
                                pos = next;
                            } else {
                                step_aside(&s.map, &mut pos, &mut path, t, &occupied, &mut rng);
                            }
                        }
                    }
                } else {
                    // Unclaim any stale claim (arrived at target, resource was already gone)
                    if let Some(p) = my_claimed.take() {
                        s.reserved.remove(&p);
                    }
                    // Find nearest known resource (respecting priority)
                    let best = best_target(&s, pos, &priority);
                    if let Some(t) = best {
                        s.reserved.insert(t);
                        my_claimed = Some(t);
                        target = Some(t);
                        path = pathfinding::bfs(&s.map, pos, t).unwrap_or_default();
                        if let Some(next) = path.first().copied() {
                            let occupied = occupied_by_others(&s, robot_idx);
                            if !occupied.contains(&next) {
                                path.remove(0);
                                pos = next;
                            } else {
                                step_aside(&s.map, &mut pos, &mut path, t, &occupied, &mut rng);
                            }
                        }
                    } else {
                        // Wander until scouts discover something
                        let occupied = occupied_by_others(&s, robot_idx);
                        let neighbors = pathfinding::passable_neighbors(&s.map, pos);
                        let free: Vec<Pos> = neighbors
                            .iter()
                            .copied()
                            .filter(|p| !occupied.contains(p))
                            .collect();
                        if !free.is_empty() {
                            pos = free[rng.gen_range(0..free.len())];
                        }
                    }
                }
            }

            s.robots[robot_idx].x = pos.0;
            s.robots[robot_idx].y = pos.1;
        }

        for msg in pending_msgs {
            let _ = tx.send(msg);
        }
    }
}

/// Returns positions of all robots except `self_idx`.
fn manhattan(a: Pos, b: Pos) -> i32 {
    (a.0 as i32 - b.0 as i32).abs() + (a.1 as i32 - b.1 as i32).abs()
}

/// When the next path step is occupied by another robot, moves to the free passable
/// neighbor (excluding the blocked cell) that is closest to `goal`, then clears the
/// path so it is replanned next tick from the new position.
/// Returns true if a side-step was taken, false if the robot is truly stuck.
fn step_aside(
    map: &crate::map::Map,
    pos: &mut Pos,
    path: &mut Vec<Pos>,
    goal: Pos,
    occupied: &HashSet<Pos>,
    rng: &mut impl rand::Rng,
) -> bool {
    let blocked = match path.first() {
        Some(&n) => n,
        None => return false,
    };
    let alternatives: Vec<Pos> = pathfinding::passable_neighbors(map, *pos)
        .into_iter()
        .filter(|p| *p != blocked && !occupied.contains(p))
        .collect();
    if alternatives.is_empty() {
        return false;
    }
    let min_d = alternatives
        .iter()
        .map(|p| manhattan(*p, goal))
        .min()
        .unwrap();
    let tied: Vec<Pos> = alternatives
        .into_iter()
        .filter(|p| manhattan(*p, goal) == min_d)
        .collect();
    path.clear();
    *pos = tied[rng.gen_range(0..tied.len())];
    true
}

fn occupied_by_others(s: &SimState, self_idx: usize) -> HashSet<Pos> {
    let mut occupied: HashSet<Pos> = s
        .robots
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != self_idx)
        .map(|(_, r)| (r.x, r.y))
        .collect();
    occupied.insert(s.player_pos());
    occupied
}

fn best_target(s: &SimState, pos: Pos, priority: &Priority) -> Option<Pos> {
    let dist =
        |p: &Pos| -> i32 { (pos.0 as i32 - p.0 as i32).abs() + (pos.1 as i32 - p.1 as i32).abs() };
    let available = |p: &&Pos| s.resources.contains_key(*p) && !s.reserved.contains(*p);

    match priority {
        Priority::Energy => s
            .knowledge
            .resources
            .iter()
            .filter(|(p, (k, _))| {
                s.resources.contains_key(*p)
                    && !s.reserved.contains(*p)
                    && *k == ResourceKind::Energy
            })
            .min_by_key(|(p, _)| dist(p))
            .map(|(p, _)| *p)
            .or_else(|| {
                s.knowledge
                    .resources
                    .keys()
                    .filter(available)
                    .min_by_key(|p| dist(p))
                    .copied()
            }),
        Priority::Crystal => s
            .knowledge
            .resources
            .iter()
            .filter(|(p, (k, _))| {
                s.resources.contains_key(*p)
                    && !s.reserved.contains(*p)
                    && *k == ResourceKind::Crystal
            })
            .min_by_key(|(p, _)| dist(p))
            .map(|(p, _)| *p)
            .or_else(|| {
                s.knowledge
                    .resources
                    .keys()
                    .filter(available)
                    .min_by_key(|p| dist(p))
                    .copied()
            }),
        Priority::None => s
            .knowledge
            .resources
            .keys()
            .filter(available)
            .min_by_key(|p| dist(p))
            .copied(),
    }
}
