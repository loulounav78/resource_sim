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
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
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

const MAP_WIDTH: usize = 120;
const MAP_HEIGHT: usize = 40;
const UI_TICK_MS: u64 = 80;

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
    }
}

fn start_simulation(cfg: &Config) -> (Arc<Mutex<SimState>>, SimHandle) {
    let seed: u32 = rand::random();
    let (map, resources) = Map::generate(
        MAP_WIDTH, MAP_HEIGHT, seed,
        (cfg.energy_min, cfg.energy_max),
        (cfg.crystal_min, cfg.crystal_max),
    );
    let base_pos = map.base_pos();

    let mut initial = SimState::new(map, resources);
    let total = cfg.num_scouts + cfg.num_collectors;
    for i in 0..total {
        initial.robots.push(RobotDisplay {
            x: base_pos.0,
            y: base_pos.1,
            kind: if i < cfg.num_scouts { RobotKind::Scout } else { RobotKind::Collector },
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
        handles.push(thread::spawn(move || robot::run_scout(i, base_pos, tx, s, r)));
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

// ── Main ────────────────────────────────────────────────────────

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut save = save::load();
    let mut screen = Screen::MainMenu { selected: 0 };
    let tick = Duration::from_millis(UI_TICK_MS);
    let mut last_tick = Instant::now();

    'main: loop {
        // ── Render ────────────────────────────────────────────
        match &screen {
            Screen::MainMenu { selected } => {
                let sel = *selected;
                terminal.draw(|f| ui::draw_main_menu(f, sel, &save))?;
            }
            Screen::Store { selected_item, feedback } => {
                let sel = *selected_item;
                let fb = *feedback;
                terminal.draw(|f| ui::draw_store(f, sel, &save, fb))?;
            }
            Screen::Running { state, .. } => {
                let s = state.lock().unwrap();
                terminal.draw(|f| ui::draw_simulation(f, &s))?;
            }
            Screen::VictoryDialog { state, elapsed_secs, energy_earned, crystals_earned } => {
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
        if let Screen::Running { state, start_time, .. } = &screen {
            if state.lock().unwrap().resources.is_empty() {
                let elapsed_secs = start_time.elapsed().as_secs();
                let old = std::mem::replace(&mut screen, Screen::MainMenu { selected: 0 });
                if let Screen::Running { state, handle, .. } = old {
                    let (energy_earned, crystals_earned) = {
                        let s = state.lock().unwrap();
                        (s.collected_energy, s.collected_crystals)
                    };
                    save.total_energy += energy_earned;
                    save.total_crystals += crystals_earned;
                    save::persist(&save);
                    handle.stop();
                    screen = Screen::VictoryDialog { state, elapsed_secs, energy_earned, crystals_earned };
                }
                continue;
            }
        }

        // ── Input ─────────────────────────────────────────────
        let timeout = tick.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press { continue; }

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
                        KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
                        KeyCode::Left  => { if *selected > 0 { *selected -= 1; } Action::None }
                        KeyCode::Right => { if *selected < LEVELS.len() - 1 { *selected += 1; } Action::None }
                        KeyCode::Char('s') | KeyCode::Char('S') => Action::OpenStore,
                        KeyCode::Enter => Action::StartLevel(*selected),
                        _ => Action::None,
                    },
                    Screen::Store { selected_item, feedback } => match key.code {
                        KeyCode::Esc => Action::CloseStore,
                        KeyCode::Up => {
                            if *selected_item > 0 { *selected_item -= 1; }
                            *feedback = None;
                            Action::None
                        }
                        KeyCode::Down => {
                            if *selected_item < store::STORE_ITEMS.len() - 1 { *selected_item += 1; }
                            *feedback = None;
                            Action::None
                        }
                        KeyCode::Enter => Action::StoreBuy(*selected_item),
                        _ => Action::None,
                    },
                    Screen::Running { .. } => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => Action::QuitRunning,
                        _ => Action::None,
                    },
                    Screen::VictoryDialog { .. } => match key.code {
                        KeyCode::Enter | KeyCode::Char('q') | KeyCode::Esc => Action::VictoryReturn,
                        _ => Action::None,
                    },
                };

                match action {
                    Action::Quit => break 'main,
                    Action::OpenStore => {
                        screen = Screen::Store { selected_item: 0, feedback: None };
                    }
                    Action::CloseStore => {
                        screen = Screen::MainMenu { selected: 0 };
                    }
                    Action::StartLevel(level) => {
                        let cfg = build_config(level, &save);
                        let (state, handle) = start_simulation(&cfg);
                        screen = Screen::Running { state, handle, start_time: Instant::now(), level };
                    }
                    Action::StoreBuy(idx) => {
                        let msg = if store::apply_upgrade(idx, &mut save) {
                            save::persist(&save);
                            ("Amelioration achetee !", true)
                        } else {
                            ("Cristaux insuffisants !", false)
                        };
                        if let Screen::Store { feedback, .. } = &mut screen {
                            *feedback = Some(msg);
                        }
                    }
                    Action::QuitRunning => {
                        let old = std::mem::replace(&mut screen, Screen::MainMenu { selected: 0 });
                        if let Screen::Running { state, handle, .. } = old {
                            let s = state.lock().unwrap();
                            save.total_energy += s.collected_energy;
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
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}
