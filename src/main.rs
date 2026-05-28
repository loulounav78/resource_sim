mod config;
mod levels;
mod map;
mod message;
mod pathfinding;
mod robot;
mod save;
mod simulation;
mod store;
mod types;
mod ui;

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        KeyboardEnhancementFlags, ModifierKeyCode, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use config::{Config, Priority};
use levels::LEVELS;
use map::Map;
use simulation::{RobotDisplay, RobotKind, SimState};
use types::Direction;

const MAP_WIDTH: usize = 120;
const MAP_HEIGHT: usize = 40;
const UI_TICK_MS: u64 = 80;
const RALLY_HOLD_GRACE_MS: u64 = 300;

// ── Simulation lifecycle ────────────────────────────────────────

struct SimHandle {
    running: Arc<AtomicBool>,
    handles: Vec<JoinHandle<()>>,
}

impl SimHandle {
    fn stop(self) {
        self.running.store(false, Ordering::Relaxed);
        for h in self.handles {
            let _ = h.join();
        }
    }
}

fn build_config(level_idx: usize, save: &save::SaveData) -> Config {
    let lvl = &LEVELS[level_idx];
    Config {
        num_scouts: save.num_scouts,
        num_collectors: save.num_collectors,
        priority: Priority::None,
        carry_capacity: save.carry_capacity,
        energy_min: lvl.energy_min + save.energy_bonus,
        energy_max: lvl.energy_max + save.energy_bonus,
        crystal_min: lvl.crystal_min + save.crystal_bonus,
        crystal_max: lvl.crystal_max + save.crystal_bonus,
        wall_break_power: save.wall_break_power,
    }
}

fn start_simulation(cfg: &Config) -> (Arc<Mutex<SimState>>, SimHandle) {
    let seed: u32 = rand::random();
    let (map, resources) = Map::generate(
        MAP_WIDTH,
        MAP_HEIGHT,
        seed,
        (cfg.energy_min, cfg.energy_max),
        (cfg.crystal_min, cfg.crystal_max),
    );
    let base_pos = map.base_pos();

    let mut initial = SimState::new(map, resources, cfg.wall_break_power);
    let total = cfg.num_scouts + cfg.num_collectors;
    for i in 0..total {
        initial.robots.push(RobotDisplay {
            x: base_pos.0,
            y: base_pos.1,
            kind: if i < cfg.num_scouts {
                RobotKind::Scout
            } else {
                RobotKind::Collector
            },
            carrying: false,
        });
    }

    let state = Arc::new(Mutex::new(initial));
    let running = Arc::new(AtomicBool::new(true));
    let (msg_tx, msg_rx) = mpsc::channel::<message::Message>();
    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    for i in 0..cfg.num_scouts {
        let tx = msg_tx.clone();
        let s = state.clone();
        let r = running.clone();
        handles.push(thread::spawn(move || {
            robot::run_scout(i, base_pos, tx, s, r)
        }));
    }

    let num_scouts = cfg.num_scouts;
    let carry_cap = cfg.carry_capacity;
    let prio = cfg.priority.clone();
    for i in 0..cfg.num_collectors {
        let tx = msg_tx.clone();
        let s = state.clone();
        let r = running.clone();
        let p = prio.clone();
        handles.push(thread::spawn(move || {
            robot::run_collector(num_scouts + i, base_pos, tx, s, r, carry_cap, p)
        }));
    }

    {
        let s = state.clone();
        let r = running.clone();
        drop(msg_tx);
        handles.push(thread::spawn(move || robot::base_processor(msg_rx, s, r)));
    }

    (state, SimHandle { running, handles })
}

// ── App screen state ────────────────────────────────────────────

enum Screen {
    MainMenu {
        selected: usize,
    },
    Store {
        selected_item: usize,
        feedback: Option<(&'static str, bool)>, // (message, is_success)
    },
    Running {
        state: Arc<Mutex<SimState>>,
        handle: SimHandle,
        start_time: Instant,
        rally_until: Option<Instant>,
        rally_shift_held: bool,
        #[allow(dead_code)]
        level: usize,
    },
    VictoryDialog {
        state: Arc<Mutex<SimState>>,
        elapsed_secs: u64,
        energy_earned: u32,
        crystals_earned: u32,
    },
}

fn stop_screen(screen: Screen) {
    if let Screen::Running { handle, .. } = screen {
        handle.stop();
    }
}

fn refresh_scout_rally(screen: &mut Screen) {
    if let Screen::Running {
        state,
        rally_until,
        rally_shift_held,
        ..
    } = screen
    {
        if let Some(shift_down) = shift_key_is_down() {
            *rally_shift_held = shift_down;
        }

        let timed_active = match *rally_until {
            Some(until) if Instant::now() <= until => true,
            Some(_) => {
                *rally_until = None;
                false
            }
            None => false,
        };
        state
            .lock()
            .unwrap()
            .set_scout_rally_active(*rally_shift_held || timed_active);
    }
}

fn pulse_scout_rally(state: &Arc<Mutex<SimState>>, rally_until: &mut Option<Instant>) {
    *rally_until = Some(Instant::now() + Duration::from_millis(RALLY_HOLD_GRACE_MS));
    state.lock().unwrap().set_scout_rally_active(true);
}

fn set_scout_rally_held(state: &Arc<Mutex<SimState>>, rally_shift_held: &mut bool, held: bool) {
    *rally_shift_held = held;
    if held {
        state.lock().unwrap().set_scout_rally_active(true);
    }
}

fn release_scout_rally(
    state: &Arc<Mutex<SimState>>,
    rally_until: &mut Option<Instant>,
    rally_shift_held: &mut bool,
) {
    *rally_shift_held = false;
    let timed_active = rally_until
        .map(|until| Instant::now() <= until)
        .unwrap_or(false);
    if !timed_active {
        *rally_until = None;
        state.lock().unwrap().set_scout_rally_active(false);
    }
}

fn direction_from_key(code: KeyCode) -> Option<Direction> {
    match code {
        KeyCode::Char(c) => match c.to_ascii_lowercase() {
            'z' => Some(Direction::Up),
            's' => Some(Direction::Down),
            'q' => Some(Direction::Left),
            'd' => Some(Direction::Right),
            _ => None,
        },
        KeyCode::Up => Some(Direction::Up),
        KeyCode::Down => Some(Direction::Down),
        KeyCode::Left => Some(Direction::Left),
        KeyCode::Right => Some(Direction::Right),
        _ => None,
    }
}

fn is_shift_key(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::Modifier(ModifierKeyCode::LeftShift | ModifierKeyCode::RightShift)
    )
}

#[cfg(windows)]
fn shift_key_is_down() -> Option<bool> {
    use winapi::um::winuser::{GetAsyncKeyState, VK_SHIFT};

    Some(unsafe { GetAsyncKeyState(VK_SHIFT) < 0 })
}

#[cfg(not(windows))]
fn shift_key_is_down() -> Option<bool> {
    None
}

// ── Main ────────────────────────────────────────────────────────

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let keyboard_enhancement_enabled = execute!(
        stdout,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES,
        )
    )
    .is_ok();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut save = save::load();
    let mut screen = Screen::MainMenu { selected: 0 };
    let tick = Duration::from_millis(UI_TICK_MS);
    let mut last_tick = Instant::now();

    'main: loop {
        refresh_scout_rally(&mut screen);

        // ── Render ────────────────────────────────────────────
        match &screen {
            Screen::MainMenu { selected } => {
                let sel = *selected;
                terminal.draw(|f| ui::draw_main_menu(f, sel, &save))?;
            }
            Screen::Store {
                selected_item,
                feedback,
            } => {
                let sel = *selected_item;
                let fb = *feedback;
                terminal.draw(|f| ui::draw_store(f, sel, &save, fb))?;
            }
            Screen::Running { state, .. } => {
                let s = state.lock().unwrap();
                terminal.draw(|f| ui::draw_simulation(f, &s))?;
            }
            Screen::VictoryDialog {
                state,
                elapsed_secs,
                energy_earned,
                crystals_earned,
            } => {
                let s = state.lock().unwrap();
                let secs = *elapsed_secs;
                let ee = *energy_earned;
                let ce = *crystals_earned;
                let te = save.total_energy;
                let tc = save.total_crystals;
                terminal.draw(|f| {
                    ui::draw_simulation(f, &s);
                    ui::draw_victory_dialog(f, secs, ee, ce, te, tc);
                })?;
            }
        }

        // ── Victory detection ─────────────────────────────────
        if let Screen::Running {
            state, start_time, ..
        } = &screen
        {
            if state.lock().unwrap().resources.is_empty() {
                let elapsed_secs = start_time.elapsed().as_secs();
                let old = std::mem::replace(&mut screen, Screen::MainMenu { selected: 0 });
                if let Screen::Running { state, handle, .. } = old {
                    let (energy_earned, crystals_earned) = {
                        let s = state.lock().unwrap();
                        (s.available_energy, s.collected_crystals)
                    };
                    save.total_energy += energy_earned;
                    save.total_crystals += crystals_earned;
                    save::persist(&save);
                    handle.stop();
                    screen = Screen::VictoryDialog {
                        state,
                        elapsed_secs,
                        energy_earned,
                        crystals_earned,
                    };
                }
                continue;
            }
        }

        // ── Input ─────────────────────────────────────────────
        let timeout = tick.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                let is_key_down = matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat);

                if let Screen::Running {
                    state,
                    rally_until,
                    rally_shift_held,
                    ..
                } = &mut screen
                {
                    if key.kind == KeyEventKind::Release && is_shift_key(key.code) {
                        release_scout_rally(state, rally_until, rally_shift_held);
                    } else if is_key_down && is_shift_key(key.code) {
                        set_scout_rally_held(state, rally_shift_held, true);
                    } else if is_key_down && key.modifiers.contains(KeyModifiers::SHIFT) {
                        pulse_scout_rally(state, rally_until);
                    }
                }

                if !is_key_down {
                    continue;
                }

                enum Action {
                    None,
                    Quit,
                    OpenStore,
                    CloseStore,
                    StartLevel(usize),
                    StoreBuy(usize),
                    QuitRunning,
                    VictoryReturn,
                }

                let action = match &mut screen {
                    Screen::MainMenu { selected } => match key.code {
                        KeyCode::Esc => Action::Quit,
                        KeyCode::Left => {
                            if *selected > 0 {
                                *selected -= 1;
                            }
                            Action::None
                        }
                        KeyCode::Right => {
                            if *selected < LEVELS.len() - 1 {
                                *selected += 1;
                            }
                            Action::None
                        }
                        KeyCode::Char('s') | KeyCode::Char('S') => Action::OpenStore,
                        KeyCode::Enter => Action::StartLevel(*selected),
                        _ => Action::None,
                    },
                    Screen::Store {
                        selected_item,
                        feedback,
                    } => match key.code {
                        KeyCode::Esc => Action::CloseStore,
                        KeyCode::Up => {
                            if *selected_item > 0 {
                                *selected_item -= 1;
                            }
                            *feedback = None;
                            Action::None
                        }
                        KeyCode::Down => {
                            if *selected_item < store::STORE_ITEMS.len() - 1 {
                                *selected_item += 1;
                            }
                            *feedback = None;
                            Action::None
                        }
                        KeyCode::Enter => Action::StoreBuy(*selected_item),
                        _ => Action::None,
                    },
                    Screen::Running { state, .. } => match key.code {
                        KeyCode::Esc => Action::QuitRunning,
                        _ => {
                            if let Some(direction) = direction_from_key(key.code) {
                                let mut s = state.lock().unwrap();
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    s.damage_wall_from_player(direction);
                                } else {
                                    s.move_player(direction);
                                }
                            }
                            Action::None
                        }
                    },
                    Screen::VictoryDialog { .. } => match key.code {
                        KeyCode::Enter | KeyCode::Esc => Action::VictoryReturn,
                        _ => Action::None,
                    },
                };

                match action {
                    Action::Quit => break 'main,
                    Action::OpenStore => {
                        screen = Screen::Store {
                            selected_item: 0,
                            feedback: None,
                        };
                    }
                    Action::CloseStore => {
                        screen = Screen::MainMenu { selected: 0 };
                    }
                    Action::StartLevel(level) => {
                        let cfg = build_config(level, &save);
                        let (state, handle) = start_simulation(&cfg);
                        screen = Screen::Running {
                            state,
                            handle,
                            start_time: Instant::now(),
                            rally_until: None,
                            rally_shift_held: false,
                            level,
                        };
                    }
                    Action::StoreBuy(idx) => {
                        let msg = match store::apply_upgrade(idx, &mut save) {
                            store::PurchaseResult::Bought => {
                                save::persist(&save);
                                ("Amelioration achetee !", true)
                            }
                            store::PurchaseResult::InsufficientResources => {
                                ("Ressources insuffisantes !", false)
                            }
                            store::PurchaseResult::MaxedOut => {
                                ("Amelioration deja au maximum !", false)
                            }
                        };
                        if let Screen::Store { feedback, .. } = &mut screen {
                            *feedback = Some(msg);
                        }
                    }
                    Action::QuitRunning => {
                        let old = std::mem::replace(&mut screen, Screen::MainMenu { selected: 0 });
                        if let Screen::Running { state, handle, .. } = old {
                            let s = state.lock().unwrap();
                            save.total_energy += s.available_energy;
                            save.total_crystals += s.collected_crystals;
                            drop(s);
                            save::persist(&save);
                            handle.stop();
                        }
                    }
                    Action::VictoryReturn => {
                        screen = Screen::MainMenu { selected: 0 };
                    }
                    Action::None => {}
                }
            }
        }

        if last_tick.elapsed() >= tick {
            last_tick = Instant::now();
        }
    }

    stop_screen(screen);

    disable_raw_mode()?;
    if keyboard_enhancement_enabled {
        execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags)?;
    }
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
