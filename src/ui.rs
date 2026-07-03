use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::levels::LEVELS;
use crate::map::Tile;
use crate::save::SaveData;
use crate::simulation::{
    wall_break_energy_cost, RobotKind, SimState, SCOUT_RALLY_ENERGY_COST_PER_SCOUT,
};
use crate::store::{can_afford, is_maxed, item_cost, StoreCurrency, STORE_ITEMS};
use crate::types::ResourceKind;

// ──────────────────────────────────────────────────────────────
//  Menu principal
// ──────────────────────────────────────────────────────────────

pub fn draw_main_menu(
    f: &mut Frame,
    selected: usize,
    save: &SaveData,
    seed_input_focused: bool,
    seed_input: &str,
    favorites_focused: bool,
    fav_selected: usize,
) {
    let area = f.area();

    let outer = Block::default()
        .borders(Borders::ALL)
        .title(" RESOURCE SIMULATION ")
        .title_alignment(Alignment::Center);
    f.render_widget(outer, area);

    let inner = area.inner(Margin {
        horizontal: 2,
        vertical: 1,
    });

    let fav_visible_lines = save.favorite_maps.len().min(6).max(1) as u16;
    let fav_block_height = fav_visible_lines + 2; // bordures

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),            // barre ressources
            Constraint::Length(2),            // label "Selectionnez"
            Constraint::Length(10),           // cartes de niveau
            Constraint::Length(5),            // bouton STORE
            Constraint::Length(3),            // saisie d'une seed
            Constraint::Length(fav_block_height), // liste de cartes favorites
            Constraint::Min(1),               // espace flexible
            Constraint::Length(1),            // footer
        ])
        .split(inner);

    // Barre de ressources
    let res_line = Line::from(vec![
        Span::raw("  "),
        Span::styled("Energie : ", Style::default().fg(Color::Green)),
        Span::styled(
            save.total_energy.to_string(),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "      Cristaux : ",
            Style::default().fg(Color::LightMagenta),
        ),
        Span::styled(
            save.total_crystals.to_string(),
            Style::default()
                .fg(Color::LightMagenta)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    f.render_widget(
        Paragraph::new(res_line)
            .block(Block::default().borders(Borders::ALL).title(" Ressources ")),
        chunks[0],
    );

    // Label
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "  Selectionnez un niveau :",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ))),
        chunks[1],
    );

    // Cartes de niveau — 5 colonnes égales
    let card_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ])
        .split(chunks[2]);

    for (i, lvl) in LEVELS.iter().enumerate() {
        let is_sel = i == selected;
        let card_area = card_cols[i].inner(Margin {
            horizontal: 1,
            vertical: 0,
        });

        let border_style = if is_sel {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let card_block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(if is_sel {
                format!(" Niv.{} ", i + 1)
            } else {
                format!(" Niv.{} ", i + 1)
            })
            .title_alignment(Alignment::Center)
            .title_style(if is_sel {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            });

        let card_inner = card_block.inner(card_area);
        f.render_widget(card_block, card_area);

        let subtitle_style = if is_sel {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let e_range = format!("{}-{}", lvl.energy_min, lvl.energy_max);
        let c_range = format!("{}-{}", lvl.crystal_min, lvl.crystal_max);

        let lines = vec![
            Line::from(Span::styled(lvl.subtitle, subtitle_style)),
            Line::raw(""),
            Line::from(vec![
                Span::styled("E: ", Style::default().fg(Color::Green)),
                Span::styled(&e_range, Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::styled("C: ", Style::default().fg(Color::LightMagenta)),
                Span::styled(&c_range, Style::default().fg(Color::LightMagenta)),
            ]),
            Line::raw(""),
            if is_sel {
                Line::from(Span::styled(
                    "[ Enter ]",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::raw("")
            },
        ];

        f.render_widget(
            Paragraph::new(lines).alignment(Alignment::Center),
            card_inner,
        );
    }

    // Bouton STORE
    let store_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title_alignment(Alignment::Center);
    let store_inner = store_block.inner(chunks[3]);
    f.render_widget(store_block, chunks[3]);

    let store_lines = vec![
        Line::raw(""),
        Line::from(Span::styled(
            "  [ S T O R E ]  — ameliorez vos bots et le vaisseau",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  Appuyez sur  S  pour entrer",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    f.render_widget(
        Paragraph::new(store_lines).alignment(Alignment::Center),
        store_inner,
    );

    // Saisie d'une seed a lancer directement
    let seed_border_style = if seed_input_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let seed_block = Block::default()
        .borders(Borders::ALL)
        .border_style(seed_border_style)
        .title(" Lancer une seed ")
        .title_alignment(Alignment::Center);
    let seed_inner = seed_block.inner(chunks[4]);
    f.render_widget(seed_block, chunks[4]);

    let cursor = if seed_input_focused { "_" } else { "" };
    let seed_display = if seed_input.is_empty() {
        format!("  Seed : {}", cursor)
    } else {
        format!("  Seed : {}{}", seed_input, cursor)
    };
    let mut seed_spans = vec![Span::styled(
        seed_display,
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )];
    if seed_input.parse::<u32>().is_ok() {
        seed_spans.push(Span::styled(
            "   [Enter] Lancer",
            Style::default().fg(Color::Green),
        ));
    } else if !seed_input_focused {
        seed_spans.push(Span::styled(
            "   (selectionnez puis tapez des chiffres)",
            Style::default().fg(Color::DarkGray),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(seed_spans)), seed_inner);

    // Liste des cartes favorites
    let fav_border_style = if favorites_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let fav_block = Block::default()
        .borders(Borders::ALL)
        .border_style(fav_border_style)
        .title(" Cartes favorites ")
        .title_alignment(Alignment::Center);
    let fav_inner = fav_block.inner(chunks[5]);
    f.render_widget(fav_block, chunks[5]);

    if save.favorite_maps.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  Aucune carte favorite. Appuyez sur  F  en jeu pour en ajouter une.",
                Style::default().fg(Color::DarkGray),
            ))),
            fav_inner,
        );
    } else {
        let mut fav_lines: Vec<Line> = Vec::new();
        for (i, fav) in save.favorite_maps.iter().take(6).enumerate() {
            let is_sel = favorites_focused && i == fav_selected;
            let lvl_name = LEVELS
                .get(fav.level)
                .map(|l| l.subtitle)
                .unwrap_or("?");
            let prefix = if is_sel { "> " } else { "  " };
            let style = if is_sel {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            fav_lines.push(Line::from(Span::styled(
                format!(
                    "{}Niv.{} ({})  —  Seed {}",
                    prefix,
                    fav.level + 1,
                    lvl_name,
                    fav.seed
                ),
                style,
            )));
        }
        if save.favorite_maps.len() > 6 {
            fav_lines.push(Line::from(Span::styled(
                format!("  ... et {} de plus", save.favorite_maps.len() - 6),
                Style::default().fg(Color::DarkGray),
            )));
        }
        f.render_widget(Paragraph::new(fav_lines), fav_inner);
    }

    // Footer
    let footer = Line::from(vec![
        Span::styled(" <-/-> ", Style::default().fg(Color::Yellow)),
        Span::raw("Niveau   "),
        Span::styled(" ^/v ", Style::default().fg(Color::Yellow)),
        Span::raw("Favoris   "),
        Span::styled(" Enter ", Style::default().fg(Color::Yellow)),
        Span::raw("Jouer   "),
        Span::styled(" Suppr ", Style::default().fg(Color::Yellow)),
        Span::raw("Retirer favori   "),
        Span::styled(" S ", Style::default().fg(Color::Cyan)),
        Span::raw("Store   "),
        Span::styled(" Esc ", Style::default().fg(Color::Yellow)),
        Span::raw("Quitter"),
    ]);
    f.render_widget(Paragraph::new(footer), chunks[7]);
}

// ──────────────────────────────────────────────────────────────
//  Store
// ──────────────────────────────────────────────────────────────

pub fn draw_store(
    f: &mut Frame,
    selected_item: usize,
    save: &SaveData,
    feedback: Option<(&str, bool)>,
) {
    let area = f.area();

    let outer = Block::default()
        .borders(Borders::ALL)
        .title(" STORE — Depensez vos cristaux et votre energie ")
        .title_alignment(Alignment::Center);
    f.render_widget(outer, area);

    let inner = area.inner(Margin {
        horizontal: 2,
        vertical: 1,
    });

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // ressources
            Constraint::Length(1),  // espace
            Constraint::Length(13), // items (6 × 2 lignes + 1 intro)
            Constraint::Length(1),  // feedback
            Constraint::Length(5),  // stats actuelles
            Constraint::Min(0),     // espace flexible
            Constraint::Length(1),  // footer
        ])
        .split(inner);

    // Barre de ressources
    let res_line = Line::from(vec![
        Span::raw("  "),
        Span::styled("Energie : ", Style::default().fg(Color::Green)),
        Span::styled(
            save.total_energy.to_string(),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "      Cristaux disponibles : ",
            Style::default().fg(Color::LightMagenta),
        ),
        Span::styled(
            save.total_crystals.to_string(),
            Style::default()
                .fg(Color::LightMagenta)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    f.render_widget(
        Paragraph::new(res_line)
            .block(Block::default().borders(Borders::ALL).title(" Ressources ")),
        chunks[0],
    );

    // Liste des items
    let mut item_lines: Vec<Line> = vec![Line::raw("")];
    for (i, item) in STORE_ITEMS.iter().enumerate() {
        let is_sel = i == selected_item;
        let cost = item_cost(i, save);
        let maxed = is_maxed(i, save);
        let affordable = can_afford(i, save) && !maxed;
        let (currency_label, currency_color) = match item.currency {
            StoreCurrency::Crystals => ("cristaux", Color::LightMagenta),
            StoreCurrency::Energy => ("energie", Color::Green),
        };

        let arrow = if is_sel { "> " } else { "  " };

        let name_style = if is_sel {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let cost_style = if maxed {
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD)
        } else if affordable {
            Style::default()
                .fg(currency_color)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Red)
        };
        let cost_text = if maxed {
            "MAX".to_string()
        } else {
            format!("{} {}", cost, currency_label)
        };

        item_lines.push(Line::from(vec![
            Span::styled(format!("{}{:<22}", arrow, item.name), name_style),
            Span::styled(
                format!("{:<38}", item.description),
                Style::default().fg(Color::Gray),
            ),
            Span::styled("Cout: ", Style::default().fg(Color::DarkGray)),
            Span::styled(cost_text, cost_style),
        ]));
        item_lines.push(Line::raw(""));
    }
    f.render_widget(Paragraph::new(item_lines), chunks[2]);

    // Feedback
    if let Some((msg, is_ok)) = feedback {
        let color = if is_ok { Color::Green } else { Color::Red };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("  {}", msg),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ))),
            chunks[3],
        );
    }

    // Stats actuelles
    let stats_lines = vec![
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Scouts: ", Style::default().fg(Color::Red)),
            Span::styled(
                save.num_scouts.to_string(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::styled("Collectors: ", Style::default().fg(Color::Magenta)),
            Span::styled(
                save.num_collectors.to_string(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::styled("Cargo: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                save.carry_capacity.to_string(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Bonus Energie:  ", Style::default().fg(Color::Green)),
            Span::styled(
                format!("+{}", save.energy_bonus),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::styled("Bonus Cristaux: ", Style::default().fg(Color::LightMagenta)),
            Span::styled(
                format!("+{}", save.crystal_bonus),
                Style::default()
                    .fg(Color::LightMagenta)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Cassage murs: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{}/3", save.wall_break_power),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::styled(
                format!(
                    "Cout action: -{}E",
                    wall_break_energy_cost(save.wall_break_power)
                ),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
    ];
    f.render_widget(
        Paragraph::new(stats_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Configuration actuelle "),
        ),
        chunks[4],
    );

    // Footer
    let footer = Line::from(vec![
        Span::styled(" Up Down ", Style::default().fg(Color::Yellow)),
        Span::raw("Naviguer   "),
        Span::styled(" Enter ", Style::default().fg(Color::Yellow)),
        Span::raw("Acheter   "),
        Span::styled(" Esc ", Style::default().fg(Color::Yellow)),
        Span::raw("Retour au menu"),
    ]);
    f.render_widget(Paragraph::new(footer), chunks[6]);
}

// ──────────────────────────────────────────────────────────────
//  Simulation screen
// ──────────────────────────────────────────────────────────────

pub fn draw_simulation(f: &mut Frame, state: &SimState, is_favorite: bool) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(4)])
        .split(area);

    draw_map(f, state, chunks[0]);
    draw_stats(f, state, chunks[1], is_favorite);
}

fn draw_map(f: &mut Frame, state: &SimState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Resource Collection Simulation  —  Esc retour au menu ");
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
            if state.player.x == x && state.player.y == y {
                spans.push(Span::styled(
                    "V",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ));
                continue;
            }

            if let Some(robot) = state.robots.iter().find(|r| r.x == x && r.y == y) {
                let (ch, color) = match robot.kind {
                    RobotKind::Scout => ("x", Color::Red),
                    RobotKind::Collector => {
                        if robot.carrying {
                            ("@", Color::LightYellow)
                        } else {
                            ("o", Color::Magenta)
                        }
                    }
                };
                spans.push(Span::styled(ch, Style::default().fg(color)));
                continue;
            }

            if let Some(res) = state.resources.get(&(x, y)) {
                let (ch, color) = match res.kind {
                    ResourceKind::Energy => ("E", Color::Green),
                    ResourceKind::Crystal => ("C", Color::LightMagenta),
                };
                spans.push(Span::styled(ch, Style::default().fg(color)));
                continue;
            }

            spans.push(match state.map.tiles[y][x] {
                Tile::Obstacle(3) => Span::styled("O", Style::default().fg(Color::LightCyan)),
                Tile::Obstacle(2) => Span::styled("Ø", Style::default().fg(Color::Cyan)),
                Tile::Obstacle(_) => Span::styled("¤", Style::default().fg(Color::Blue)),
                Tile::Base => Span::styled("#", Style::default().fg(Color::LightGreen)),
                Tile::Empty => Span::raw(" "),
            });
        }
        lines.push(Line::from(spans));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

// ──────────────────────────────────────────────────────────────
//  Victory dialog
// ──────────────────────────────────────────────────────────────

pub fn draw_victory_dialog(
    f: &mut Frame,
    elapsed_secs: u64,
    energy_earned: u32,
    crystals_earned: u32,
    total_energy: u32,
    total_crystals: u32,
    is_favorite: bool,
) {
    let area = f.area();

    let popup_w = 76u16;
    let popup_h = 18u16;
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect {
        x,
        y,
        width: popup_w.min(area.width),
        height: popup_h.min(area.height),
    };

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
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::from(vec![
            Span::raw("  Temps ecoule : "),
            Span::styled(
                time_str,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::raw(""),
        Line::from(Span::styled(
            "  Recolte cette partie :",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(vec![
            Span::raw("    "),
            Span::styled(
                format!("Energie +{}  ", energy_earned),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("Cristaux +{}", crystals_earned),
                Style::default()
                    .fg(Color::LightMagenta)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::raw(""),
        Line::from(Span::styled(
            "  Total accumule :",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(vec![
            Span::raw("    "),
            Span::styled(
                format!("Energie {}  ", total_energy),
                Style::default().fg(Color::Green),
            ),
            Span::styled(
                format!("Cristaux {}", total_crystals),
                Style::default().fg(Color::LightMagenta),
            ),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "  [ Retour au menu ]  ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("  Enter / Esc ", Style::default().fg(Color::Yellow)),
            Span::raw("Retour au menu   "),
            Span::styled("F ", Style::default().fg(Color::Yellow)),
            Span::raw(if is_favorite {
                "Retirer des favoris "
            } else {
                "Ajouter aux favoris "
            }),
            Span::styled(
                if is_favorite { "[*]" } else { "[ ]" },
                Style::default().fg(if is_favorite {
                    Color::Yellow
                } else {
                    Color::DarkGray
                }),
            ),
        ]),
    ];

    f.render_widget(Paragraph::new(lines), inner);
}

// ──────────────────────────────────────────────────────────────
//  Stats bar
// ──────────────────────────────────────────────────────────────

fn draw_stats(f: &mut Frame, state: &SimState, area: Rect, is_favorite: bool) {
    let carrying = state.robots.iter().filter(|r| r.carrying).count();
    let known = state.knowledge.resources.len();
    let remaining = state.resources.len();
    let rally_status = if state.scout_rally_pos.is_some() {
        "ON"
    } else {
        "OFF"
    };

    let line = Line::from(vec![
        Span::styled(
            format!(" Energie recoltee: {} ", state.available_energy),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("| ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("Cristaux: {} ", state.collected_crystals),
            Style::default()
                .fg(Color::LightMagenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("| ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("Restantes: {} ", remaining),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled("| ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("Connues: {} ", known),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled("| ", Style::default().fg(Color::DarkGray)),
        Span::raw(format!(
            "Scouts: {}  Collectors: {} ({} portent)",
            state
                .robots
                .iter()
                .filter(|r| r.kind == RobotKind::Scout)
                .count(),
            state
                .robots
                .iter()
                .filter(|r| r.kind == RobotKind::Collector)
                .count(),
            carrying,
        )),
    ]);
    let controls = Line::from(vec![
        Span::styled(
            " Vaisseau: ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("ZQSD bouger   "),
        Span::styled("Ctrl+ZQSD ", Style::default().fg(Color::Cyan)),
        Span::raw(format!(
            "casser murs ({}/3, -{}E)   ",
            state.wall_break_power,
            state.wall_break_energy_cost()
        )),
        Span::styled("Shift ", Style::default().fg(Color::Red)),
        Span::raw(format!(
            "all scouts come to me (-{}E/scout, prochain -{}E) ",
            SCOUT_RALLY_ENERGY_COST_PER_SCOUT,
            state.scout_rally_energy_cost()
        )),
        Span::styled(
            format!("[{}]", rally_status),
            Style::default().fg(if state.scout_rally_pos.is_some() {
                Color::Green
            } else {
                Color::DarkGray
            }),
        ),
        Span::raw("   "),
        Span::styled("F ", Style::default().fg(Color::Yellow)),
        Span::raw(if is_favorite {
            "Retirer des favoris "
        } else {
            "Ajouter aux favoris "
        }),
        Span::styled(
            if is_favorite { "[*]" } else { "[ ]" },
            Style::default().fg(if is_favorite {
                Color::Yellow
            } else {
                Color::DarkGray
            }),
        ),
    ]);

    f.render_widget(
        Paragraph::new(vec![line, controls])
            .block(Block::default().borders(Borders::ALL).title(" Stats ")),
        area,
    );
}
