use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Cell, Padding, Paragraph, Row, Table},
    Frame,
};

use crate::app::{App, AppStatus};
use crate::stats::StageStatus;

pub fn draw(frame: &mut Frame, app: &App) {
    if !app.is_loaded() {
        draw_welcome_screen(frame, frame.size(), app);
        return;
    }
    draw_pipeline_screen(frame, frame.size(), app);
}

fn draw_welcome_screen(frame: &mut Frame, area: Rect, app: &App) {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

    draw_header(frame, vert[0], app);
    draw_status_bar(frame, vert[1], app);

    if app.file_input_active {
        draw_file_input(frame, vert[2], app);
    } else {
        draw_welcome_message(frame, vert[2], app);
    }

    draw_welcome_footer(frame, vert[3], app);
}

fn draw_welcome_message(frame: &mut Frame, area: Rect, app: &App) {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(12),
            Constraint::Fill(1),
        ])
        .split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .title("");

    let inner = block.inner(vert[1]);
    frame.render_widget(block, vert[1]);

    let mut lines = Vec::new();

    lines.push(Line::from(Span::styled(
        "Pipevision",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));

    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(Span::styled(
        "TUI for debugging Linux command pipelines!",
        Style::default().fg(Color::White),
    )));
    lines.push(Line::from(Span::styled(
        "Inspect real-time throughput, filtering, and output at each stage.",
        Style::default().fg(Color::White),
    )));
    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(Span::styled(
        "Press [L] to load a pipeline file.",
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(Span::styled(
        "Example pipeline file (pipe.txt, pipe.sh):",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  cat test.sh | grep ERROR | sort",
        Style::default().fg(Color::Yellow),
    )));
    lines.push(Line::from(Span::raw("")));

    if let Some(ref err) = app.error_message {
        lines.push(Line::from(Span::styled(
            err,
            Style::default().fg(Color::Red),
        )));
    }

    let paragraph =
        Paragraph::new(Text::from(lines)).block(Block::default().padding(Padding::new(2, 0, 0, 0)));
    frame.render_widget(paragraph, inner);
}

fn draw_file_input(frame: &mut Frame, area: Rect, app: &App) {
    let show_n = app.file_suggestions.len().min(8);
    let inner_content_h = 4 + show_n as u16;
    let box_h = (inner_content_h + 2).min(area.height.saturating_sub(2));

    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(box_h),
            Constraint::Fill(1),
        ])
        .split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .title(" Load Pipeline File ");

    let inner = block.inner(vert[1]);
    frame.render_widget(block, vert[1]);

    let input_width = (inner.width as usize).saturating_sub(4);
    let display = if app.file_input_buffer.len() > input_width {
        format!(
            "...{}",
            &app.file_input_buffer[app
                .file_input_buffer
                .len()
                .saturating_sub(input_width.saturating_sub(3))..]
        )
    } else {
        app.file_input_buffer.clone()
    };

    let cursor_visible = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() / 500 % 2 == 0)
        .unwrap_or(true);

    let input_line = if cursor_visible {
        format!("{}|", display)
    } else {
        format!("{} ", display)
    };

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(vec![
        Span::raw("  Path: ").style(Style::default().fg(Color::Cyan)),
        Span::raw(&input_line),
    ]));

    if !app.file_suggestions.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Suggestions", Style::default().fg(Color::DarkGray)),
            Span::raw(format!(
                " ({}/{})",
                app.file_suggestions.len().min(8),
                app.file_suggestions.len()
            )),
        ]));

        let highlight = Style::default()
            .fg(Color::Black)
            .bg(Color::LightGreen)
            .add_modifier(Modifier::BOLD);

        for (i, sug) in app.file_suggestions.iter().take(8).enumerate() {
            let style = if i == app.selected_suggestion {
                highlight
            } else {
                Style::default().fg(Color::White)
            };
            let arrow = if i == app.selected_suggestion {
                "\u{2192} "
            } else {
                "  "
            };
            lines.push(Line::from(vec![
                Span::raw(format!("   {}{}", arrow, sug)).style(style)
            ]));
        }

        if app.file_suggestions.len() > 8 {
            lines.push(Line::from(vec![Span::styled(
                format!("   \u{2026} and {} more", app.file_suggestions.len() - 8),
                Style::default().fg(Color::DarkGray),
            )]));
        }
    } else if !app.file_input_buffer.is_empty() {
        lines.push(Line::from(Span::raw("")));
        lines.push(Line::from(Span::styled(
            "  No matching files",
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines.push(Line::from(Span::raw("")));
    let help = if app.file_suggestions.is_empty() {
        "  Enter to load, Esc to cancel"
    } else {
        "  Enter to load  \u{2191}/\u{2193} Navigate  Tab Cycle  Esc Cancel"
    };
    lines.push(Line::from(Span::styled(
        help,
        Style::default().fg(Color::DarkGray),
    )));

    if let Some(ref err) = app.error_message {
        lines.push(Line::from(Span::styled(
            format!("  {}", err),
            Style::default().fg(Color::Red),
        )));
    }

    let paragraph =
        Paragraph::new(Text::from(lines)).block(Block::default().padding(Padding::new(0, 0, 0, 0)));
    frame.render_widget(paragraph, inner);

    let cursor_x = inner.x + 2 + 7 + display.len() as u16;
    let cursor_y = inner.y + 1;
    frame.set_cursor(cursor_x, cursor_y);
}

fn draw_pipeline_screen(frame: &mut Frame, area: Rect, app: &App) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(1),
            Constraint::Length(7),
            Constraint::Min(8),
            Constraint::Length(1),
        ])
        .split(area);

    draw_header(frame, main_chunks[0], app);
    draw_status_bar(frame, main_chunks[1], app);
    draw_stage_table(frame, main_chunks[2], app);

    let bottom_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(1)])
        .split(main_chunks[3]);

    draw_detail_panel(frame, bottom_chunks[0], app);
    draw_output_pane(frame, bottom_chunks[1], app);
    draw_footer(frame, main_chunks[4], app);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .title(" Pipevision");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let file_line = match app.pipeline_file {
        Some(ref f) => Line::from(vec![
            Span::raw("File: ").style(Style::default().fg(Color::Cyan)),
            Span::raw(f).style(Style::default().fg(Color::White)),
        ]),
        None => Line::from(Span::styled(
            "No file loaded",
            Style::default().fg(Color::DarkGray),
        )),
    };

    let pipeline_line = match app.pipeline {
        Some(ref p) => {
            let max_w = inner.width as usize;
            let display = if p.original.len() > max_w.saturating_sub(4) {
                format!("{}...", &p.original[..max_w.saturating_sub(7)])
            } else {
                p.original.clone()
            };
            Line::from(vec![
                Span::raw("Pipeline: ").style(Style::default().fg(Color::Cyan)),
                Span::raw(display).style(Style::default().fg(Color::Yellow)),
            ])
        }
        None => Line::from(Span::styled(
            "No pipeline loaded",
            Style::default().fg(Color::DarkGray),
        )),
    };

    let header_text = Text::from(vec![file_line, pipeline_line]);
    let paragraph =
        Paragraph::new(header_text).block(Block::default().padding(Padding::new(1, 0, 0, 0)));
    frame.render_widget(paragraph, inner);
}

fn draw_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let (icon, color) = match app.status {
        AppStatus::Idle => ("\u{25cb}", Color::Gray),
        AppStatus::Running => ("\u{25cf}", Color::Green),
        AppStatus::Completed => {
            if app.exit_status == Some(0) {
                ("\u{2713}", Color::Green)
            } else {
                ("\u{2717}", Color::Red)
            }
        }
    };

    let elapsed = match app.status {
        AppStatus::Completed => app.elapsed_seconds,
        _ => app
            .start_time
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0),
    };
    let stages_info = if app.is_loaded() {
        format!("{} stages", app.num_stages())
    } else {
        "no pipeline".to_string()
    };

    let status_line = Line::from(vec![
        Span::styled(
            format!(" {} ", icon),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &app.status_message,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("  |  {}  |  {:.1}s", stages_info, elapsed))
            .style(Style::default().fg(Color::DarkGray)),
    ]);

    let paragraph = Paragraph::new(Text::from(status_line));
    frame.render_widget(paragraph, area);
}

fn draw_welcome_footer(frame: &mut Frame, area: Rect, app: &App) {
    let line = if app.file_input_active {
        Line::from(vec![
            Span::styled(" [Enter] Confirm ", Style::default().fg(Color::Green)),
            Span::styled(" [Esc] Cancel ", Style::default().fg(Color::Red)),
        ])
    } else {
        Line::from(vec![
            Span::styled(" [L] Load File ", Style::default().fg(Color::Green)),
            Span::styled(" [Q] Quit ", Style::default().fg(Color::Red)),
        ])
    };
    let paragraph = Paragraph::new(Text::from(line));
    frame.render_widget(paragraph, area);
}

fn draw_stage_table(frame: &mut Frame, area: Rect, app: &App) {
    let selected_style = Style::default()
        .fg(Color::Black)
        .bg(Color::LightGreen)
        .add_modifier(Modifier::BOLD);

    let header = Row::new(
        vec!["Stage", "Command", "Lines", "Bytes", "L/s", "B/s"]
            .iter()
            .map(|&s| {
                Cell::from(s).style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
            }),
    )
    .height(1);

    let rows: Vec<Row> = app
        .metrics
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let is_selected = i == app.selected;
            let style = if is_selected {
                selected_style
            } else {
                Style::default()
            };

            let status_icon = match m.status {
                StageStatus::Pending => "\u{25cb}",
                StageStatus::Running => "\u{25cf}",
                StageStatus::Completed => "\u{2713}",
            };

            Row::new(vec![
                Cell::from(format!("{} {}", status_icon, m.stage)).style(style),
                Cell::from(shorten_command(&m.command, 20)).style(style),
                Cell::from(if m.total_lines > 0 {
                    format_num(m.total_lines)
                } else {
                    "-".to_string()
                })
                .style(style),
                Cell::from(if m.total_bytes > 0 {
                    format_bytes(m.total_bytes)
                } else {
                    "-".to_string()
                })
                .style(style),
                Cell::from(if m.lines_per_second > 0.0 {
                    format!("{:.1}", m.lines_per_second)
                } else {
                    "-".to_string()
                })
                .style(style),
                Cell::from(if m.bytes_per_second > 0.0 {
                    format_speed(m.bytes_per_second)
                } else {
                    "-".to_string()
                })
                .style(style),
            ])
            .height(1)
        })
        .collect();

    let widths = [
        Constraint::Length(10),
        Constraint::Length(22),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Plain)
                .title(" Stages "),
        )
        .highlight_style(selected_style);

    frame.render_widget(table, area);
}

fn draw_detail_panel(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .title(" Stage Details ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.metrics.is_empty() {
        return;
    }

    let idx = app.selected.min(app.metrics.len().saturating_sub(1));
    let stage = &app.metrics[idx];
    let cmd = &app
        .pipeline
        .as_ref()
        .map(|p| p.stages[idx].clone())
        .unwrap_or_default();
    let filtered = app.filtered_lines(idx);
    let reduction = app.reduction_pct(idx);

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::raw("Command:  ").style(Style::default().fg(Color::Cyan)),
        Span::raw(cmd).style(Style::default().fg(Color::White)),
    ]));

    lines.push(Line::from(vec![
        Span::raw("Status:   ").style(Style::default().fg(Color::Cyan)),
        Span::styled(
            format!("{:?}", stage.status),
            match stage.status {
                StageStatus::Pending => Style::default().fg(Color::Gray),
                StageStatus::Running => Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
                StageStatus::Completed => Style::default().fg(Color::Blue),
            },
        ),
    ]));

    if stage.status != StageStatus::Pending {
        let prev_lines = if idx > 0 {
            app.metrics.get(idx - 1).map(|p| p.total_lines)
        } else {
            None
        };

        let in_out = match prev_lines {
            Some(prev) => format!(
                "{}  \u{2192}  {}",
                format_num(prev),
                format_num(stage.total_lines)
            ),
            None => format!("{}", format_num(stage.total_lines)),
        };

        lines.push(Line::from(vec![
            Span::raw("Lines:    ").style(Style::default().fg(Color::Cyan)),
            Span::raw(in_out).style(Style::default().fg(Color::White)),
        ]));

        if idx > 0 {
            if let Some(f) = filtered {
                let line = match reduction {
                    Some(pct) => format!("{}  ({:.1}% removed)", format_num(f), pct),
                    None => format_num(f),
                };
                lines.push(Line::from(vec![
                    Span::raw("Filtered: ").style(Style::default().fg(Color::Cyan)),
                    Span::raw(line).style(Style::default().fg(Color::Yellow)),
                ]));
            }
        }

        lines.push(Line::from(vec![
            Span::raw("Bytes:    ").style(Style::default().fg(Color::Cyan)),
            Span::raw(format_bytes(stage.total_bytes)).style(Style::default().fg(Color::White)),
        ]));

        lines.push(Line::from(vec![
            Span::raw("Throughput:").style(Style::default().fg(Color::Cyan)),
            Span::raw(format!(
                " {:.1} L/s  |  {}",
                stage.lines_per_second,
                format_speed(stage.bytes_per_second)
            ))
            .style(Style::default().fg(Color::White)),
        ]));
    }

    let paragraph =
        Paragraph::new(Text::from(lines)).block(Block::default().padding(Padding::new(1, 0, 0, 0)));
    frame.render_widget(paragraph, inner);
}

fn draw_output_pane(frame: &mut Frame, area: Rect, app: &App) {
    let stage_num = app.selected + 1;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .title(format!(" Stage {} Output ", stage_num));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let total = app.output_total_lines();
    if total == 0 {
        let msg = if app.status == AppStatus::Running {
            "Waiting for stage output..."
        } else {
            "No output captured for this stage."
        };
        let paragraph = Paragraph::new(Text::from(Line::from(
            Span::raw(msg).style(Style::default().fg(Color::DarkGray)),
        )))
        .block(Block::default().padding(Padding::new(1, 0, 0, 0)));
        frame.render_widget(paragraph, inner);
        return;
    }

    let scroll = app.output_scroll.min(total.saturating_sub(1));
    let visible_height = (inner.height as usize).saturating_sub(1);
    let end = (scroll + visible_height).min(total);

    let display_lines: Vec<Line> = app.stage_output[scroll..end]
        .iter()
        .map(|l| {
            let truncated = if l.len() > inner.width as usize {
                format!("{}...", &l[..(inner.width as usize).saturating_sub(3)])
            } else {
                l.clone()
            };
            Line::from(Span::raw(truncated))
        })
        .collect();

    let scroll_pct = if total > visible_height {
        let pct = (scroll as f64 / (total.saturating_sub(visible_height)) as f64) * 100.0;
        format!(" [{:.0}%]", pct.min(100.0))
    } else {
        String::new()
    };

    let header = Line::from(vec![
        Span::raw(format!(
            "Showing {}-{} of {} lines (including empty lines!)",
            scroll + 1,
            end,
            format_num(total)
        ))
        .style(Style::default().fg(Color::DarkGray)),
        Span::raw(scroll_pct).style(Style::default().fg(Color::DarkGray)),
    ]);

    let mut text_lines = Vec::new();
    text_lines.push(header);
    text_lines.extend(display_lines);

    let paragraph = Paragraph::new(Text::from(text_lines))
        .block(Block::default().padding(Padding::new(1, 0, 0, 0)));
    frame.render_widget(paragraph, inner);
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let rerun_label = if app.status == AppStatus::Completed {
        " [R] Rerun "
    } else {
        " [R] Run "
    };

    let abort_label = if app.status == AppStatus::Running {
        " [X] Abort "
    } else {
        ""
    };

    let line = Line::from(vec![
        Span::styled(
            " [\u{2191}/\u{2193}] Select Stage ",
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            " [PgUp/PgDn] Scroll Output ",
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(rerun_label, Style::default().fg(Color::Green)),
        Span::styled(abort_label, Style::default().fg(Color::Red)),
        Span::styled(" [Q] Quit ", Style::default().fg(Color::Red)),
    ]);

    let paragraph = Paragraph::new(Text::from(line));
    frame.render_widget(paragraph, area);
}

fn format_num(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn format_bytes(n: usize) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = n as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{} {}", n, UNITS[unit_idx])
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

fn format_speed(bytes_per_sec: f64) -> String {
    const UNITS: &[&str] = &["B/s", "KB/s", "MB/s", "GB/s"];
    let mut size = bytes_per_sec;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{:.1} {}", size, UNITS[unit_idx])
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

fn shorten_command(cmd: &str, max_len: usize) -> String {
    if cmd.len() <= max_len {
        return cmd.to_string();
    }
    format!("{}...", &cmd[..max_len.saturating_sub(3)])
}
