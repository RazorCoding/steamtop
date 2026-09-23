use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Gauge, Row, Table, TableState};
use ratatui::Frame;

use crate::gpu::GpuInfo;
use crate::App;

const COLS: [Constraint; 6] = [
    Constraint::Min(24),
    Constraint::Length(7),
    Constraint::Length(7),
    Constraint::Length(9),
    Constraint::Length(7),
    Constraint::Length(9),
];

pub fn draw(app: &App, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(4),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(frame.area());

    draw_header(app, frame, chunks[0]);
    draw_system(app, frame, chunks[1]);
    draw_table(app, frame, chunks[2]);
    draw_footer(frame, chunks[3]);
}

fn gauge(percent: f32, label: String, color: Color) -> Gauge<'static> {
    Gauge::default()
        .gauge_style(Style::default().fg(color).bg(Color::DarkGray))
        .ratio((percent as f64 / 100.0).clamp(0.0, 1.0))
        .label(Span::styled(label, Style::default().fg(Color::White)))
}

fn draw_header(app: &App, frame: &mut Frame, area: Rect) {
    let title = Line::from(vec![
        Span::styled(
            " steamtop ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  refresh "),
        Span::styled(
            format!("{:.1}s", app.interval.as_secs_f32()),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw("  "),
        Span::styled(
            format!("view: {}", match app.view {
                crate::View::Games => "games",
                crate::View::AllProcs => "all procs",
            }),
            Style::default().fg(Color::Magenta),
        ),
        Span::raw("  sort: "),
        Span::styled(app.sort.label(), Style::default().fg(Color::Green)),
    ]);
    frame.render_widget(ratatui::widgets::Paragraph::new(title), area);
}

fn draw_system(app: &App, frame: &mut Frame, area: Rect) {
    let cols = Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(34),
            Constraint::Percentage(32),
        ])
        .split(area);

    let cpu = gauge(
        app.cpu,
        format!("CPU {:.1}%", app.cpu),
        if app.cpu > 80.0 { Color::Red } else { Color::Green },
    );
    frame.render_widget(cpu, cols[0]);

    let mem_gauge = if app.mem_total > 0 {
        app.mem_used as f32 / app.mem_total as f32 * 100.0
    } else {
        0.0
    };
    let mem = gauge(
        mem_gauge,
        format!(
            "MEM {:.1} / {:.1} GiB",
            app.mem_used as f64 / 1024.0 / 1024.0 / 1024.0,
            app.mem_total as f64 / 1024.0 / 1024.0 / 1024.0,
        ),
        if mem_gauge > 80.0 {
            Color::Red
        } else {
            Color::Blue
        },
    );
    frame.render_widget(mem, cols[1]);

    let g: &GpuInfo = &app.gpu;
    let gpu_pct = g.utilization().map(|u| u as f32).unwrap_or(0.0);
    let gpu_label = match (g.utilization(), g.vram_used_mib, g.vram_total_mib) {
        (Some(u), Some(used), Some(total)) => format!("GPU {u}%  VRAM {used}/{total} MiB"),
        (Some(u), _, _) => format!("GPU {u}%"),
        (None, _, _) => format!("GPU ({})", g.backend),
    };
    let gpu = gauge(gpu_pct, gpu_label, Color::Magenta);
    frame.render_widget(gpu, cols[2]);
}

fn draw_table(app: &App, frame: &mut Frame, area: Rect) {
    let header = Row::new(vec![
        Cell::from("NAME"),
        Cell::from("PID"),
        Cell::from("CPU%"),
        Cell::from("MEM%"),
        Cell::from("GPU%"),
        Cell::from("VRAM%"),
    ])
    .style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::White)
            .add_modifier(Modifier::BOLD),
    );

    let gpu_pct = app.gpu.utilization().map(|u| u as f64);
    let vram_pct = app.gpu.vram_ratio().map(|r| r * 100.0);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let mut table = Table::new(vec![Row::new(vec![Cell::from("No games detected — press Tab, then Space to pin a process")])], COLS)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    if !app.display_rows().is_empty() {
        let rows: Vec<Row> = app
            .display_rows()
            .iter()
            .map(|r| {
                let marker = if r.pinned {
                    '+'
                } else if r.steam {
                    '*'
                } else {
                    ' '
                };
                let name_style = if r.pinned {
                    Style::default().fg(Color::Yellow)
                } else if r.steam {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Gray)
                };
                Row::new(vec![
                    Cell::from(format!("{marker} {}", r.name)).style(name_style),
                    Cell::from(r.pid.to_string()),
                    Cell::from(format!("{:.1}", r.cpu)),
                    Cell::from(format!(
                        "{:.1}",
                        if app.mem_total > 0 {
                            r.mem_bytes as f64 / app.mem_total as f64 * 100.0
                        } else {
                            0.0
                        }
                    )),
                    Cell::from(match gpu_pct {
                        Some(v) => format!("{v:.0}"),
                        None => "-".into(),
                    }),
                    Cell::from(match vram_pct {
                        Some(v) => format!("{v:.0}"),
                        None => "-".into(),
                    }),
                ])
            })
            .collect();
        table = table.rows(rows);
    }

    let mut state = TableState::default().with_selected(Some(app.selected));
    frame.render_stateful_widget(table, area, &mut state);
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    let help = Line::from(vec![
        Span::styled(" q ", Style::default().fg(Color::Black).bg(Color::Cyan)),
        Span::raw("quit  "),
        Span::styled(
            " Tab ",
            Style::default().fg(Color::Black).bg(Color::Magenta),
        ),
        Span::raw("view  "),
        Span::styled(
            " Space ",
            Style::default().fg(Color::Black).bg(Color::Yellow),
        ),
        Span::raw("pin  "),
        Span::styled(
            " 1-4 ",
            Style::default().fg(Color::Black).bg(Color::Green),
        ),
        Span::raw("sort  "),
        Span::styled(
            " +-/ ",
            Style::default().fg(Color::Black).bg(Color::Blue),
        ),
        Span::raw("rate   "),
        Span::styled("* steam", Style::default().fg(Color::Green)),
        Span::raw("  "),
        Span::styled("+ pinned", Style::default().fg(Color::Yellow)),
    ]);
    frame.render_widget(
        ratatui::widgets::Paragraph::new(help).alignment(Alignment::Left),
        area,
    );
}
