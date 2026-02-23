use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Clear, Dataset, Gauge, Paragraph},
    Frame,
};

use crate::state::{format_linear_power, AppState};

/// Top-level draw function: renders the entire UI from AppState.
pub fn draw(f: &mut Frame, state: &AppState) {
    let chunks = Layout::vertical([
        Constraint::Length(4),  // Title + readings
        Constraint::Min(8),    // Waveform chart
        Constraint::Length(1), // Status bar
    ])
    .split(f.area());

    draw_header(f, chunks[0], state);
    draw_chart(f, chunks[1], state);
    draw_status_bar(f, chunks[2], state);

    if state.show_help {
        draw_help_overlay(f);
    }

    if let Some((loaded, total)) = state.calibration_progress {
        draw_calibration_progress(f, loaded, total);
    }

    if !state.connected && state.calibration_progress.is_none() {
        draw_connecting_screen(f);
    }
}

/// Format a dBm value for display: 3 decimal places, plus linear power in parens.
/// Returns something like "-12.345 dBm  (56.234 uW)"
fn format_reading(dbm: f64) -> String {
    let (linear_val, linear_unit) = format_linear_power(dbm);
    format!("{:>8.3} dBm  ({:.3} {})", dbm, linear_val, linear_unit)
}

// ─── Header Section ──────────────────────────────────────────────────────────

fn draw_header(f: &mut Frame, area: Rect, state: &AppState) {
    let title = if state.connected {
        let model = state
            .model_name
            .as_deref()
            .unwrap_or("Unknown");
        let serial = state
            .serial_number
            .map(|s| format!("{}", s))
            .unwrap_or_else(|| "---".to_string());
        let firmware = state
            .firmware_version
            .as_deref()
            .unwrap_or("---");
        let rate = state.sampling_rate.label();
        format!(
            " RF Power Meter \u{2500}\u{2500} {} \u{2500}\u{2500} S/N: {} \u{2500}\u{2500} v{} \u{2500}\u{2500}\u{2500} {} ",
            model, serial, firmware, rate
        )
    } else {
        " RF Power Meter \u{2500}\u{2500} Disconnected ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title);

    let has_data = state.stats.count > 0;

    let label_style = Style::default().fg(Color::Green);
    let value_style = Style::default().fg(Color::Cyan);
    let placeholder_style = Style::default().fg(Color::DarkGray);

    let over_range = state.connected && has_data && state.current_dbm.is_none();

    let (avg_text, cur_text, min_text, max_text) = if has_data {
        let avg = format_reading(state.stats.avg);
        let cur = if over_range {
            "OVR".to_string()
        } else {
            state
                .current_dbm
                .map(|v| format_reading(v))
                .unwrap_or_else(|| "---".to_string())
        };
        let min = format_reading(state.stats.min);
        let max = format_reading(state.stats.max);
        (avg, cur, min, max)
    } else {
        let dash = "---".to_string();
        (dash.clone(), dash.clone(), dash.clone(), dash)
    };

    let ovr_style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);

    let style_for = |text: &str| -> Style {
        if text == "---" {
            placeholder_style
        } else if text == "OVR" {
            ovr_style
        } else {
            value_style
        }
    };

    let line1 = Line::from(vec![
        Span::raw("  "),
        Span::styled("Avg: ", label_style),
        Span::styled(avg_text.clone(), style_for(&avg_text)),
        Span::raw("   "),
        Span::styled("Cur: ", label_style),
        Span::styled(cur_text.clone(), style_for(&cur_text)),
    ]);

    let line2 = Line::from(vec![
        Span::raw("  "),
        Span::styled("Min: ", label_style),
        Span::styled(min_text.clone(), style_for(&min_text)),
        Span::raw("   "),
        Span::styled("Max: ", label_style),
        Span::styled(max_text.clone(), style_for(&max_text)),
    ]);

    let paragraph = Paragraph::new(vec![line1, line2]).block(block);
    f.render_widget(paragraph, area);
}

// ─── Chart Section ───────────────────────────────────────────────────────────

fn draw_chart(f: &mut Frame, area: Rect, state: &AppState) {
    // Build data points from waveform ring buffer, right-aligned to capacity
    let capacity = state.waveform.capacity();
    let offset = capacity - state.waveform.len();
    let data_points: Vec<(f64, f64)> = state
        .waveform
        .iter()
        .enumerate()
        .map(|(i, &v)| ((offset + i) as f64, v))
        .collect();

    let x_max = capacity as f64;

    // Compute Y bounds
    let (y_min, y_max) = if state.auto_scale && !state.waveform.is_empty() {
        let min_val = state
            .waveform
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let max_val = state
            .waveform
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        let range = (max_val - min_val).max(1.0);
        let padding = range * 0.1;
        let mut lo = min_val - padding;
        let mut hi = max_val + padding;
        // Enforce minimum 10 dBm range
        let min_range = 10.0;
        if hi - lo < min_range {
            let mid = (hi + lo) / 2.0;
            lo = mid - min_range / 2.0;
            hi = mid + min_range / 2.0;
        }
        // Cap y_max at +2.5 dBm unless data exceeds it
        let y_cap = 2.5;
        if hi > y_cap && max_val <= y_cap {
            hi = y_cap;
            lo = hi - (hi - lo).max(min_range);
        }
        // Floor y_min at -40 dBm
        let y_floor = -40.0;
        if lo < y_floor {
            lo = y_floor;
        }
        (lo, hi)
    } else {
        (state.y_min, state.y_max)
    };

    let title_spans = if state.paused {
        vec![
            Span::raw(" Waveform "),
            Span::styled(
                "\u{2500}\u{2500} PAUSED ",
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            ),
        ]
    } else {
        vec![Span::raw(" Waveform ")]
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(title_spans));

    let y_min_label = format!("{:.1}", y_min);
    let y_max_label = format!("{:.1}", y_max);

    let datasets = vec![Dataset::default()
        .marker(Marker::Braille)
        .style(Style::default().fg(Color::Yellow))
        .data(&data_points)];

    let chart = Chart::new(datasets)
        .block(block)
        .x_axis(
            Axis::default()
                .bounds([0.0, x_max])
                .labels(Vec::<Line>::new()),
        )
        .y_axis(
            Axis::default()
                .bounds([y_min, y_max])
                .labels(vec![Line::from(y_min_label), Line::from(y_max_label)]),
        );

    f.render_widget(chart, area);
}

// ─── Status Bar ──────────────────────────────────────────────────────────────

fn draw_status_bar(f: &mut Frame, area: Rect, state: &AppState) {
    if let Some((ref label, ref buf)) = state.input_prompt {
        let line = Line::from(vec![
            Span::styled(
                format!(" {}", label),
                Style::default().fg(Color::Black).bg(Color::Yellow),
            ),
            Span::styled(
                format!("{}_ ", buf),
                Style::default().fg(Color::White).bg(Color::Yellow),
            ),
        ]);
        let bar = Paragraph::new(line).style(Style::default().bg(Color::Yellow));
        f.render_widget(bar, area);
        return;
    }

    let log_span = if state.logging {
        Span::styled("ON", Style::default().fg(Color::Green))
    } else {
        Span::styled("OFF", Style::default().fg(Color::DarkGray))
    };

    let line = Line::from(vec![
        Span::raw(format!(
            " Freq: {:.1} MHz  Offset: {:+.1} dB  Log: ",
            state.frequency_mhz, state.offset_db,
        )),
        log_span,
        Span::raw("     [?] Help"),
    ]);

    let bar = Paragraph::new(line).style(
        Style::default()
            .fg(Color::White)
            .bg(Color::DarkGray),
    );

    f.render_widget(bar, area);
}

// ─── Help Overlay ────────────────────────────────────────────────────────────

fn draw_help_overlay(f: &mut Frame) {
    let area = centered_rect(50, 60, f.area());

    f.render_widget(Clear, area);

    let help_text = vec![
        Line::from(""),
        Line::from("  Key Bindings:"),
        Line::from(""),
        Line::from("    q       Quit"),
        Line::from("    Space   Pause/Resume"),
        Line::from("    f       Set frequency"),
        Line::from("    o       Set power offset"),
        Line::from("    s       Cycle sampling rate"),
        Line::from("    l       Toggle CSV logging"),
        Line::from("    r       Reset statistics"),
        Line::from("    a       Toggle auto-scale"),
        Line::from("    +/-     Zoom Y axis"),
        Line::from("    ?       Close this help"),
        Line::from(""),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Help ")
        .style(Style::default().bg(Color::Black));

    let paragraph = Paragraph::new(help_text).block(block);
    f.render_widget(paragraph, area);
}

// ─── Calibration Progress Screen ─────────────────────────────────────────────

fn draw_calibration_progress(f: &mut Frame, loaded: usize, total: usize) {
    let area = centered_rect(50, 5, f.area());

    f.render_widget(Clear, area);

    let ratio = if total > 0 {
        loaded as f64 / total as f64
    } else {
        0.0
    };

    let label = format!("Loading calibration: {}/{}", loaded, total);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Calibration ");

    let gauge = Gauge::default()
        .block(block)
        .gauge_style(Style::default().fg(Color::Cyan))
        .ratio(ratio.min(1.0))
        .label(label);

    f.render_widget(gauge, area);
}

// ─── Connecting Screen ───────────────────────────────────────────────────────

fn draw_connecting_screen(f: &mut Frame) {
    let area = centered_rect(40, 5, f.area());

    f.render_widget(Clear, area);

    let block = Block::default().borders(Borders::ALL);

    let text = Paragraph::new(Line::from("Connecting to device..."))
        .block(block)
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(text, area);
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Compute a centered rectangle of the given percentage width and absolute height
/// within the given outer rectangle.
fn centered_rect(percent_x: u16, height: u16, outer: Rect) -> Rect {
    let popup_width = outer.width * percent_x / 100;
    let x = outer.x + (outer.width.saturating_sub(popup_width)) / 2;
    let y = outer.y + (outer.height.saturating_sub(height)) / 2;
    Rect::new(
        x,
        y,
        popup_width.min(outer.width),
        height.min(outer.height),
    )
}
