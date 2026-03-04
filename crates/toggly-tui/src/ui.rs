use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::app::{App, Dialog, Pane, Screen};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(0),   // main content
            Constraint::Length(3), // status bar
        ])
        .split(f.area());

    draw_header(f, chunks[0], app);
    draw_main(f, chunks[1], app);
    draw_status_bar(f, chunks[2], app);

    // Draw dialog overlay if present
    if let Some(dialog) = &app.dialog {
        draw_dialog(f, dialog);
    }
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let title = match &app.screen {
        Screen::ProjectList => " Toggly — Projects ".to_string(),
        Screen::ProjectDetail { project } => format!(" Toggly — {} ", project.name),
        Screen::FlagDetail {
            project,
            flag_with_states,
        } => {
            format!(
                " Toggly — {} — {} ",
                project.name, flag_with_states.flag.key
            )
        }
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(block, area);
}

fn draw_main(f: &mut Frame, area: Rect, app: &App) {
    match &app.screen {
        Screen::ProjectList => draw_project_list(f, area, app),
        Screen::ProjectDetail { .. } => draw_project_detail(f, area, app),
        Screen::FlagDetail {
            flag_with_states, ..
        } => draw_flag_detail(f, area, flag_with_states, app),
    }
}

fn draw_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let help = match &app.screen {
        Screen::ProjectList => "q: quit | j/k: navigate | Enter: select | c: create | d: delete | r: refresh",
        Screen::ProjectDetail { .. } => "Esc: back | Tab: switch pane | j/k: navigate | Enter: open flag | c: create | d: delete | r: refresh",
        Screen::FlagDetail { .. } => "Esc: back | j/k: navigate | t: toggle | r: refresh",
    };

    let status_text = if app.status_message.is_empty() {
        help.to_string()
    } else {
        format!("{} | {}", app.status_message, help)
    };

    let paragraph = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(paragraph, area);
}

// ---------------------------------------------------------------------------
// Project List
// ---------------------------------------------------------------------------

fn draw_project_list(f: &mut Frame, area: Rect, app: &App) {
    if app.projects.is_empty() {
        let msg = Paragraph::new("No projects yet. Press 'c' to create one.")
            .block(Block::default().borders(Borders::ALL).title(" Projects "));
        f.render_widget(msg, area);
        return;
    }

    let items: Vec<ListItem> = app
        .projects
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let style = if i == app.project_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let content = Line::from(vec![
                Span::styled(&p.name, style),
                Span::styled(
                    format!("  {}", p.description),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Projects "))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    f.render_widget(list, area);
}

// ---------------------------------------------------------------------------
// Project Detail
// ---------------------------------------------------------------------------

fn draw_project_detail(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    // Environments pane
    let env_border_style = if app.active_pane == Pane::Environments {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    if app.environments.is_empty() {
        let msg = Paragraph::new("No environments. Press 'c' to create.").block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Environments ")
                .border_style(env_border_style),
        );
        f.render_widget(msg, chunks[0]);
    } else {
        let items: Vec<ListItem> = app
            .environments
            .iter()
            .enumerate()
            .map(|(i, env)| {
                let style = if i == app.env_index && app.active_pane == Pane::Environments {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let content = Line::from(vec![
                    Span::styled(&env.name, style),
                    Span::styled(
                        format!("  ({})", &env.sdk_key[..12.min(env.sdk_key.len())]),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]);
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Environments ")
                .border_style(env_border_style),
        );
        f.render_widget(list, chunks[0]);
    }

    // Flags pane
    let flag_border_style = if app.active_pane == Pane::Flags {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    if app.flags.is_empty() {
        let msg = Paragraph::new("No flags. Press 'c' to create.").block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Flags ")
                .border_style(flag_border_style),
        );
        f.render_widget(msg, chunks[1]);
    } else {
        let rows: Vec<Row> = app
            .flags
            .iter()
            .enumerate()
            .map(|(i, flag)| {
                let style = if i == app.flag_index && app.active_pane == Pane::Flags {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                Row::new(vec![
                    flag.key.clone(),
                    flag.name.clone(),
                    flag.flag_type.as_str().to_string(),
                ])
                .style(style)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(35),
                Constraint::Percentage(45),
                Constraint::Percentage(20),
            ],
        )
        .header(
            Row::new(vec!["Key", "Name", "Type"])
                .style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Flags ")
                .border_style(flag_border_style),
        );
        f.render_widget(table, chunks[1]);
    }
}

// ---------------------------------------------------------------------------
// Flag Detail
// ---------------------------------------------------------------------------

fn draw_flag_detail(
    f: &mut Frame,
    area: Rect,
    fws: &toggly_core::models::FlagWithStates,
    app: &App,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(area);

    // Flag info
    let info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Key: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&fws.flag.key),
            Span::raw("  "),
            Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&fws.flag.name),
        ]),
        Line::from(vec![
            Span::styled("Type: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(fws.flag.flag_type.as_str()),
            Span::raw("  "),
            Span::styled("Description: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&fws.flag.description),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).title(" Flag Info "))
    .wrap(Wrap { trim: true });
    f.render_widget(info, chunks[0]);

    // Environment states
    if fws.environments.is_empty() {
        let msg = Paragraph::new("No environment states. Create environments first.")
            .block(Block::default().borders(Borders::ALL).title(" Environment States "));
        f.render_widget(msg, chunks[1]);
    } else {
        let rows: Vec<Row> = fws
            .environments
            .iter()
            .enumerate()
            .map(|(i, state)| {
                let style = if i == app.flag_env_index {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                Row::new(vec![
                    state.environment_name.clone(),
                    if state.enabled { "● ON".into() } else { "○ OFF".into() },
                    state.default_value.to_string(),
                    state.off_value.to_string(),
                ])
                .style(style)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(25),
                Constraint::Percentage(15),
                Constraint::Percentage(30),
                Constraint::Percentage(30),
            ],
        )
        .header(
            Row::new(vec!["Environment", "Status", "Default Value", "Off Value"])
                .style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Environment States — press 't' to toggle "),
        );
        f.render_widget(table, chunks[1]);
    }
}

// ---------------------------------------------------------------------------
// Dialog overlay
// ---------------------------------------------------------------------------

fn draw_dialog(f: &mut Frame, dialog: &Dialog) {
    let area = centered_rect(50, 30, f.area());
    f.render_widget(Clear, area);

    match dialog {
        Dialog::CreateProject { name } => {
            let block = Block::default()
                .title(" Create Project ")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Cyan));
            let inner = block.inner(area);
            f.render_widget(block, area);

            let text = Paragraph::new(vec![
                Line::from("Project name:"),
                Line::from(Span::styled(
                    format!("{}_", name),
                    Style::default().fg(Color::Yellow),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Enter: confirm | Esc: cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ]);
            f.render_widget(text, inner);
        }

        Dialog::CreateEnvironment { name, .. } => {
            let block = Block::default()
                .title(" Create Environment ")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Cyan));
            let inner = block.inner(area);
            f.render_widget(block, area);

            let text = Paragraph::new(vec![
                Line::from("Environment name:"),
                Line::from(Span::styled(
                    format!("{}_", name),
                    Style::default().fg(Color::Yellow),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Enter: confirm | Esc: cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ]);
            f.render_widget(text, inner);
        }

        Dialog::CreateFlag {
            key,
            name,
            active_field,
            ..
        } => {
            let block = Block::default()
                .title(" Create Flag ")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Cyan));
            let inner = block.inner(area);
            f.render_widget(block, area);

            let key_style = if *active_field == 0 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            let name_style = if *active_field == 1 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            let text = Paragraph::new(vec![
                Line::from("Flag key:"),
                Line::from(Span::styled(
                    format!("{}{}", key, if *active_field == 0 { "_" } else { "" }),
                    key_style,
                )),
                Line::from("Flag name:"),
                Line::from(Span::styled(
                    format!("{}{}", name, if *active_field == 1 { "_" } else { "" }),
                    name_style,
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Tab: switch field | Enter: confirm | Esc: cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ]);
            f.render_widget(text, inner);
        }

        Dialog::ConfirmDelete { message, .. } => {
            let block = Block::default()
                .title(" Confirm Delete ")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Red));
            let inner = block.inner(area);
            f.render_widget(block, area);

            let text = Paragraph::new(vec![
                Line::from(message.as_str()),
                Line::from(""),
                Line::from(Span::styled(
                    "y: confirm | any other key: cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ]);
            f.render_widget(text, inner);
        }

        Dialog::Error { message } => {
            let block = Block::default()
                .title(" Error ")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Red));
            let inner = block.inner(area);
            f.render_widget(block, area);

            let text = Paragraph::new(vec![
                Line::from(message.as_str()),
                Line::from(""),
                Line::from(Span::styled(
                    "Press any key to dismiss",
                    Style::default().fg(Color::DarkGray),
                )),
            ]);
            f.render_widget(text, inner);
        }
    }
}

/// Create a centered rectangle for dialogs.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
