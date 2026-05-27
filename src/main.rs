mod map;
mod message;
mod pathfinding;
mod robot;
mod simulation;
mod types;
mod ui;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use map::Map;
use simulation::{RobotDisplay, RobotKind, SimState};

const MAP_WIDTH: usize = 120;
const MAP_HEIGHT: usize = 40;
const NUM_SCOUTS: usize = 3;
const NUM_COLLECTORS: usize = 3;
const UI_TICK_MS: u64 = 80;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let seed: u32 = rand::random();
    let (map, resources) = Map::generate(MAP_WIDTH, MAP_HEIGHT, seed);
    let base_pos = map.base_pos();

    let mut initial_state = SimState::new(map, resources);

    // Initialise robot display slots (all starting at base)
    for i in 0..(NUM_SCOUTS + NUM_COLLECTORS) {
        initial_state.robots.push(RobotDisplay {
            x: base_pos.0,
            y: base_pos.1,
            kind: if i < NUM_SCOUTS {
                RobotKind::Scout
            } else {
                RobotKind::Collector
            },
            carrying: false,
        });
    }

    let state: Arc<Mutex<SimState>> = Arc::new(Mutex::new(initial_state));

    // Channel for robot → base communication
    let (msg_tx, msg_rx) = mpsc::channel::<message::Message>();

    // Base processor thread
    {
        let s = state.clone();
        thread::spawn(move || robot::base_processor(msg_rx, s));
    }

    // Scout threads
    for i in 0..NUM_SCOUTS {
        let tx = msg_tx.clone();
        let s = state.clone();
        thread::spawn(move || robot::run_scout(i, base_pos, tx, s));
    }

    // Collector threads
    for i in 0..NUM_COLLECTORS {
        let tx = msg_tx.clone();
        let s = state.clone();
        thread::spawn(move || robot::run_collector(NUM_SCOUTS + i, base_pos, tx, s));
    }

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick = Duration::from_millis(UI_TICK_MS);
    let mut last_tick = Instant::now();

    loop {
        {
            let s = state.lock().unwrap();
            terminal.draw(|f| ui::draw(f, &s))?;
        }

        let timeout = tick.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(_) = event::read()? {
                break;
            }
        }

        if last_tick.elapsed() >= tick {
            last_tick = Instant::now();
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
