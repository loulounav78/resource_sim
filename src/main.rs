mod config;
mod map;
mod message;
mod pathfinding;
mod robot;
mod simulation;
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

use config::{Config, NUM_FIELDS};
use map::Map;
use simulation::{RobotDisplay, RobotKind, SimState};

const MAP_WIDTH: usize = 120;
const MAP_HEIGHT: usize = 40;
const UI_TICK_MS: u64 = 80;

// ── Simulation lifecycle ────────────────────────────────────────

struct SimHandle {
    running: Arc<AtomicBool>,
    /// Robot threads first, base_processor last (so it exits after robots drop their senders).
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

fn start_simulation(cfg: &Config) -> (Arc<Mutex<SimState>>, SimHandle) {
    let seed: u32 = rand::random();
    let (map, resources) =
        Map::generate(MAP_WIDTH, MAP_HEIGHT, seed, (cfg.energy_min, cfg.energy_max), (cfg.crystal_min, cfg.crystal_max));
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
    Config {
        selected: usize,
        prev_game: Option<(Arc<Mutex<SimState>>, SimHandle)>,
    },
    Running {
        state: Arc<Mutex<SimState>>,
        handle: SimHandle,
        start_time: Instant,
    },
    ConfirmDialog {
        option: usize, // 0 = nouvelle partie, 1 = retour à la partie
        prev_game: (Arc<Mutex<SimState>>, SimHandle),
    },
    VictoryDialog {
        state: Arc<Mutex<SimState>>,
        elapsed_secs: u64,
    },
}

fn stop_screen(screen: Screen) {
    match screen {
        Screen::Running { handle, .. } => handle.stop(),
        Screen::Config { prev_game: Some((_, handle)), .. } => handle.stop(),
        Screen::ConfirmDialog { prev_game: (_, handle), .. } => handle.stop(),
        _ => {}
    }
}

// ── Main ────────────────────────────────────────────────────────

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut cfg = Config::default();
    let mut cfg_snapshot: Option<Config> = None;
    let mut screen = Screen::Config { selected: 0, prev_game: None };
    let tick = Duration::from_millis(UI_TICK_MS);
    let mut last_tick = Instant::now();

    'main: loop {
        // ── Render ────────────────────────────────────────────
        match &screen {
            Screen::Config { selected, prev_game } => {
                let sel = *selected;
                let has_prev = prev_game.is_some();
                terminal.draw(|f| ui::draw_config(f, &cfg, sel, has_prev))?;
            }
            Screen::Running { state, .. } => {
                let s = state.lock().unwrap();
                terminal.draw(|f| ui::draw_simulation(f, &s))?;
            }
            Screen::ConfirmDialog { option, prev_game } => {
                let opt = *option;
                let s = prev_game.0.lock().unwrap();
                terminal.draw(|f| {
                    ui::draw_simulation(f, &s);
                    ui::draw_confirm_dialog(f, opt);
                })?;
            }
            Screen::VictoryDialog { state, elapsed_secs } => {
                let s = state.lock().unwrap();
                let secs = *elapsed_secs;
                terminal.draw(|f| {
                    ui::draw_simulation(f, &s);
                    ui::draw_victory_dialog(f, secs);
                })?;
            }
        }

        // ── Victory detection ─────────────────────────────────
        if let Screen::Running { state, start_time, .. } = &screen {
            if state.lock().unwrap().resources.is_empty() {
                let elapsed_secs = start_time.elapsed().as_secs();
                let old = std::mem::replace(&mut screen, Screen::Config { selected: 0, prev_game: None });
                if let Screen::Running { state, handle, .. } = old {
                    handle.stop();
                    screen = Screen::VictoryDialog { state, elapsed_secs };
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
                    StartNew,
                    OpenConfig,
                    TryBack,
                    ConfirmRestart,
                    ConfirmReturn,
                    BackToConfig,
                }

                let action = match &mut screen {
                    Screen::Config { selected, prev_game } => match key.code {
                        KeyCode::Char('q') => Action::Quit,
                        KeyCode::Esc => {
                            if prev_game.is_some() { Action::TryBack } else { Action::Quit }
                        }
                        KeyCode::Up => {
                            if *selected > 0 { *selected -= 1; }
                            Action::None
                        }
                        KeyCode::Down => {
                            if *selected < NUM_FIELDS - 1 { *selected += 1; }
                            Action::None
                        }
                        KeyCode::Left => { cfg.adjust(*selected, -1); Action::None }
                        KeyCode::Right => { cfg.adjust(*selected, 1); Action::None }
                        KeyCode::Enter => Action::StartNew,
                        _ => Action::None,
                    },
                    Screen::Running { .. } => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
                        _ => Action::OpenConfig,
                    },
                    Screen::ConfirmDialog { option, .. } => match key.code {
                        KeyCode::Esc => Action::BackToConfig,
                        KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
                            *option = 1 - *option;
                            Action::None
                        }
                        KeyCode::Enter => {
                            if *option == 0 { Action::ConfirmRestart } else { Action::ConfirmReturn }
                        }
                        _ => Action::None,
                    },
                    Screen::VictoryDialog { .. } => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
                        KeyCode::Enter => Action::StartNew,
                        _ => Action::None,
                    },
                };

                match action {
                    Action::Quit => {
                        let old = std::mem::replace(&mut screen, Screen::Config { selected: 0, prev_game: None });
                        stop_screen(old);
                        break 'main;
                    }
                    Action::StartNew => {
                        let old = std::mem::replace(&mut screen, Screen::Config { selected: 0, prev_game: None });
                        stop_screen(old);
                        let (state, handle) = start_simulation(&cfg);
                        screen = Screen::Running { state, handle, start_time: Instant::now() };
                        cfg_snapshot = None;
                    }
                    Action::OpenConfig => {
                        cfg_snapshot = Some(cfg.clone());
                        let old = std::mem::replace(&mut screen, Screen::Config { selected: 0, prev_game: None });
                        if let Screen::Running { state, handle, .. } = old {
                            screen = Screen::Config { selected: 0, prev_game: Some((state, handle)) };
                        }
                    }
                    Action::TryBack => {
                        let modified = cfg_snapshot.as_ref().map_or(false, |snap| snap != &cfg);
                        if modified {
                            let old = std::mem::replace(&mut screen, Screen::Config { selected: 0, prev_game: None });
                            if let Screen::Config { prev_game: Some(pg), .. } = old {
                                // Default to option 1 (retour à la partie) — choix le plus sûr
                                screen = Screen::ConfirmDialog { option: 1, prev_game: pg };
                            }
                        } else {
                            let old = std::mem::replace(&mut screen, Screen::Config { selected: 0, prev_game: None });
                            if let Screen::Config { prev_game: Some((state, handle)), .. } = old {
                                screen = Screen::Running { state, handle, start_time: Instant::now() };
                                cfg_snapshot = None;
                            }
                        }
                    }
                    Action::ConfirmRestart => {
                        let old = std::mem::replace(&mut screen, Screen::Config { selected: 0, prev_game: None });
                        stop_screen(old);
                        let (state, handle) = start_simulation(&cfg);
                        screen = Screen::Running { state, handle, start_time: Instant::now() };
                        cfg_snapshot = None;
                    }
                    Action::ConfirmReturn => {
                        if let Some(snap) = cfg_snapshot.take() {
                            cfg = snap;
                        }
                        let old = std::mem::replace(&mut screen, Screen::Config { selected: 0, prev_game: None });
                        if let Screen::ConfirmDialog { prev_game: (state, handle), .. } = old {
                            screen = Screen::Running { state, handle, start_time: Instant::now() };
                        }
                    }
                    Action::BackToConfig => {
                        let old = std::mem::replace(&mut screen, Screen::Config { selected: 0, prev_game: None });
                        if let Screen::ConfirmDialog { prev_game, .. } = old {
                            screen = Screen::Config { selected: 0, prev_game: Some(prev_game) };
                        }
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
