use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::map::Tile;
use crate::simulation::{RobotKind, SimState};
use crate::types::ResourceKind;

pub fn draw(f: &mut Frame, state: &SimState) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    draw_map(f, state, chunks[0]);
    draw_stats(f, state, chunks[1]);
}

fn draw_map(f: &mut Frame, state: &SimState, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Resource Collection Simulation — Press any key to exit ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let vis_w = inner.width as usize;
    let vis_h = inner.height as usize;
    let map_w = state.map.width.min(vis_w);
    let map_h = state.map.height.min(vis_h);

    let mut lines: Vec<Line> = Vec::with_capacity(map_h);

    for y in 0..map_h {
        let mut spans: Vec<Span> = Vec::with_capacity(map_w);

        for x in 0..map_w {
            // Robots have highest draw priority
            if let Some(robot) = state.robots.iter().find(|r| r.x == x && r.y == y) {
                let (ch, color) = match robot.kind {
                    RobotKind::Scout => ("x", Color::Red),
                    RobotKind::Collector => {
                        if robot.carrying {
                            ("@", Color::LightMagenta)
                        } else {
                            ("o", Color::Magenta)
                        }
                    }
                };
                spans.push(Span::styled(ch, Style::default().fg(color)));
                continue;
            }

            // Resources
            if let Some(res) = state.resources.get(&(x, y)) {
                let (ch, color) = match res.kind {
                    ResourceKind::Energy => ("E", Color::Green),
                    ResourceKind::Crystal => ("C", Color::LightMagenta),
                };
                spans.push(Span::styled(ch, Style::default().fg(color)));
                continue;
            }

            // Tile background
            let span = match state.map.tiles[y][x] {
                Tile::Obstacle => Span::styled("O", Style::default().fg(Color::LightCyan)),
                Tile::Base => Span::styled("#", Style::default().fg(Color::LightGreen)),
                Tile::Empty => Span::raw(" "),
            };
            spans.push(span);
        }

        lines.push(Line::from(spans));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_stats(f: &mut Frame, state: &SimState, area: ratatui::layout::Rect) {
    let scouts = state
        .robots
        .iter()
        .filter(|r| r.kind == RobotKind::Scout)
        .count();
    let collectors = state
        .robots
        .iter()
        .filter(|r| r.kind == RobotKind::Collector)
        .count();
    let carrying = state
        .robots
        .iter()
        .filter(|r| r.kind == RobotKind::Collector && r.carrying)
        .count();

    let known = state.knowledge.resources.len();
    let remaining = state.resources.len();

    let line = Line::from(vec![
        Span::styled(
            format!(" Energy: {} ", state.collected_energy),
            Style::default().fg(Color::Green),
        ),
        Span::raw("| "),
        Span::styled(
            format!("Crystals: {} ", state.collected_crystals),
            Style::default().fg(Color::LightMagenta),
        ),
        Span::raw("| "),
        Span::styled(
            format!("Resources left: {} ", remaining),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw("| "),
        Span::styled(
            format!("Known: {} ", known),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw("| "),
        Span::raw(format!(
            "Scouts: {}  Collectors: {} ({} carrying) ",
            scouts, collectors, carrying
        )),
    ]);

    let block = Block::default().borders(Borders::ALL).title(" Stats ");
    f.render_widget(Paragraph::new(line).block(block), area);
}
