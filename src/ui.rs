use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::config::{Config, Priority, NUM_FIELDS};
use crate::map::Tile;
use crate::simulation::{RobotKind, SimState};
use crate::types::ResourceKind;

// ──────────────────────────────────────────────────────────────
//  Config screen
// ──────────────────────────────────────────────────────────────

pub fn draw_config(f: &mut Frame, config: &Config, selected: usize, has_prev_game: bool) {
    let area = f.area();

    let title = if has_prev_game {
        " Resource Simulation — Configuration  (partie en cours) "
    } else {
        " Resource Simulation — Configuration "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_alignment(Alignment::Center);
    f.render_widget(block, area);

    let inner = area.inner(Margin { horizontal: 2, vertical: 1 });

    // Header
    let header = Line::from(vec![
        Span::styled("  Parameter                  ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled("Value                     ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled("Controls", Style::default().add_modifier(Modifier::BOLD)),
    ]);

    let separator = Line::from(Span::styled(
        "─".repeat(inner.width as usize),
        Style::default().fg(Color::DarkGray),
    ));

    let mut lines: Vec<Line> = vec![header, separator, Line::raw("")];

    for i in 0..NUM_FIELDS {
        let is_selected = i == selected;
        let is_start_btn = i == NUM_FIELDS - 1;

        let arrow = if is_selected { "► " } else { "  " };

        let base_style = if is_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        if is_start_btn {
            lines.push(Line::raw(""));
            let btn_style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            };
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("  [ Start Simulation ]  ", btn_style),
                Span::styled("  (Enter)", Style::default().fg(Color::DarkGray)),
            ]));
        } else {
            let label = Config::field_label(i);
            let value = config.field_value(i);

            let controls = match i {
                2 => "← → cycle",
                _ => "← → adjust",
            };

            // Color-code values by category
            let value_color = match i {
                0 | 1 => Color::Cyan,
                2 => match config.priority {
                    Priority::Energy => Color::Green,
                    Priority::Crystal => Color::LightMagenta,
                    Priority::None => Color::Gray,
                },
                3 => Color::LightYellow,
                4 | 5 => Color::Green,
                6 | 7 => Color::LightMagenta,
                _ => Color::White,
            };

            lines.push(Line::from(vec![
                Span::styled(format!("{}{:<24}", arrow, label), base_style),
                Span::styled(format!("{:<26}", value), Style::default().fg(value_color)),
                Span::styled(controls, Style::default().fg(Color::DarkGray)),
            ]));
        }
    }

    lines.push(Line::raw(""));
    lines.push(Line::raw(""));

    let mut footer_spans = vec![
        Span::styled(" ↑ ↓ ", Style::default().fg(Color::Yellow)),
        Span::raw("Navigate   "),
        Span::styled(" ← → ", Style::default().fg(Color::Yellow)),
        Span::raw("Adjust   "),
        Span::styled(" Enter ", Style::default().fg(Color::Yellow)),
        Span::raw("Start   "),
        Span::styled(" Q ", Style::default().fg(Color::Yellow)),
        Span::raw("Quit"),
    ];
    if has_prev_game {
        footer_spans.push(Span::raw("   "));
        footer_spans.push(Span::styled(" Esc ", Style::default().fg(Color::Cyan)));
        footer_spans.push(Span::styled("Retour à la partie", Style::default().fg(Color::Cyan)));
    }
    lines.push(Line::from(footer_spans));

    f.render_widget(Paragraph::new(lines), inner);
}

// ──────────────────────────────────────────────────────────────
//  Simulation screen
// ──────────────────────────────────────────────────────────────

pub fn draw_simulation(f: &mut Frame, state: &SimState) {
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
        .title(" Resource Collection Simulation — any key → config  |  Q/Esc → quitter ");
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
                        if robot.carrying { ("@", Color::LightYellow) } else { ("o", Color::Magenta) }
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

            // Tiles
            spans.push(match state.map.tiles[y][x] {
                Tile::Obstacle => Span::styled("O", Style::default().fg(Color::LightCyan)),
                Tile::Base => Span::styled("#", Style::default().fg(Color::LightGreen)),
                Tile::Empty => Span::raw(" "),
            });
        }
        lines.push(Line::from(spans));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

pub fn draw_confirm_dialog(f: &mut Frame, selected: usize) {
    let area = f.area();

    let popup_w = 62u16;
    let popup_h = 11u16;
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect { x, y, width: popup_w.min(area.width), height: popup_h.min(area.height) };

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Stats modifiées ")
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let opt0_style = if selected == 0 {
        Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    let opt1_style = if selected == 1 {
        Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };

    let lines = vec![
        Line::raw(""),
        Line::from(Span::styled(
            "  Les statistiques ont été modifiées.",
            Style::default().fg(Color::Yellow),
        )),
        Line::raw(""),
        Line::from(vec![
            Span::raw(if selected == 0 { " ► " } else { "   " }),
            Span::styled("  Nouvelle partie avec les nouvelles stats  ", opt0_style),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::raw(if selected == 1 { " ► " } else { "   " }),
            Span::styled("  Retourner à la partie en cours (annuler)  ", opt1_style),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("  ↑ ↓ ", Style::default().fg(Color::Yellow)),
            Span::raw("Choisir   "),
            Span::styled(" Enter ", Style::default().fg(Color::Yellow)),
            Span::raw("Confirmer   "),
            Span::styled(" Esc ", Style::default().fg(Color::Yellow)),
            Span::raw("Retour config"),
        ]),
    ];

    f.render_widget(Paragraph::new(lines), inner);
}

pub fn draw_victory_dialog(f: &mut Frame, elapsed_secs: u64) {
    let area = f.area();

    let popup_w = 62u16;
    let popup_h = 12u16;
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect { x, y, width: popup_w.min(area.width), height: popup_h.min(area.height) };

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Partie terminee ! ")
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let mins = elapsed_secs / 60;
    let secs = elapsed_secs % 60;
    let time_str = if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    };

    let lines = vec![
        Line::raw(""),
        Line::from(Span::styled(
            "  Bravo ! Toutes les ressources ont ete recoltees !",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::from(vec![
            Span::raw("  Temps ecoule : "),
            Span::styled(time_str, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "  [ Nouvelle partie ]  ",
                Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("  Enter ", Style::default().fg(Color::Yellow)),
            Span::raw("Nouvelle partie   "),
            Span::styled(" Q/Esc ", Style::default().fg(Color::Yellow)),
            Span::raw("Quitter"),
        ]),
    ];

    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_stats(f: &mut Frame, state: &SimState, area: ratatui::layout::Rect) {
    let carrying = state.robots.iter().filter(|r| r.carrying).count();
    let known = state.knowledge.resources.len();
    let remaining = state.resources.len();

    let line = Line::from(vec![
        Span::styled(
            format!(" ⚡ Energy: {} ", state.collected_energy),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("💎 Crystals: {} ", state.collected_crystals),
            Style::default().fg(Color::LightMagenta).add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("Resources left: {} ", remaining),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled("│ ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("Known: {} ", known), Style::default().fg(Color::Cyan)),
        Span::styled("│ ", Style::default().fg(Color::DarkGray)),
        Span::raw(format!(
            "Scouts: {}  Collectors: {} ({} carrying)",
            state.robots.iter().filter(|r| r.kind == RobotKind::Scout).count(),
            state.robots.iter().filter(|r| r.kind == RobotKind::Collector).count(),
            carrying,
        )),
    ]);

    f.render_widget(
        Paragraph::new(line).block(Block::default().borders(Borders::ALL).title(" Stats ")),
        area,
    );
}
