use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use crate::app::{App, ConnectState, ClusterStatus};

/// Main entry point for drawing the UI.
pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header / Status
            Constraint::Min(0),    // Main Content
            Constraint::Length(3), // Footer / Help
        ])
        .split(f.area());

    draw_header(f, app, chunks[0]);
    draw_main(f, app, chunks[1]);
    draw_footer(f, app, chunks[2]);

    if app.show_password_prompt {
        draw_password_prompt(f, app);
    }
}

/// Draws the application header with host and cluster status.
fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let status_text = match app.connect_state {
        ConnectState::Disconnected => Span::styled("Disconnected", Style::default().fg(Color::Red)),
        ConnectState::Connecting => Span::styled("Connecting...", Style::default().fg(Color::Yellow)),
        ConnectState::Connected => Span::styled("Connected", Style::default().fg(Color::Green)),
        ConnectState::Failed(_) => Span::styled("Connection Failed", Style::default().fg(Color::Red)),
    };

    let cluster_text = match app.cluster_status {
        ClusterStatus::Unknown => Span::raw("?"),
        ClusterStatus::Checking => Span::styled("Checking...", Style::default().fg(Color::Yellow)),
        ClusterStatus::Running => Span::styled("RUNNING", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ClusterStatus::Stopped => Span::styled("STOPPED", Style::default().fg(Color::Red)),
    };

    let title = Paragraph::new(Line::from(vec![
        Span::styled(" vLLM Cluster Manager ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" | SSH: "),
        status_text,
        Span::raw(format!(" | Host: {} | Status: ", app.config.remote_host)),
        cluster_text,
    ]))
    .block(Block::default().borders(Borders::ALL).title("Status"));
    
    f.render_widget(title, area);
}

/// Draws the main content area with model list and logs.
fn draw_main(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(70),
        ])
        .split(area);

    // Models List
    let items: Vec<ListItem> = app.models.iter().enumerate().map(|(i, m)| {
        let style = if i == app.selected_model_index {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let prefix = if i == app.selected_model_index { "> " } else { "  " };
        ListItem::new(format!("{}{}", prefix, m.name)).style(style)
    }).collect();

    let models_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Models"))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    f.render_widget(models_list, chunks[0]);

    // Logs Panel
    let height = chunks[1].height.saturating_sub(2) as usize; 
    let logs_to_show = if app.logs.len() > height {
        &app.logs[app.logs.len() - height..]
    } else {
        &app.logs[..]
    };
    
    let log_text: String = logs_to_show.join("\n");
    let logs_widget = Paragraph::new(log_text)
        .block(Block::default().borders(Borders::ALL).title("Logs / Output"))
        .wrap(Wrap { trim: true });
    
    f.render_widget(logs_widget, chunks[1]);
}

/// Draws the application footer with status messages and help text.
fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let msg = format!("Status: {}", app.status_message);
    let help = "q: Quit | c: Connect | d: Disconnect | s: Start | x: Stop | t: Tail Logs | T: Stop Tail | r: Refresh";
    
    let text = vec![
        Line::from(msg),
        Line::from(Span::styled(help, Style::default().fg(Color::DarkGray))),
    ];

    let footer = Paragraph::new(text)
        .block(Block::default().borders(Borders::TOP));
    f.render_widget(footer, area);
}

/// Draws a centered password prompt modal.
fn draw_password_prompt(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, f.area());
    let block = Block::default().title("SSH Password Required").borders(Borders::ALL);
    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(block, area);

    let inner_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .margin(1)
        .split(area);

    let msg = Paragraph::new("Enter SSH password for authentication:");
    f.render_widget(msg, inner_area[0]);

    let masked: String = "*".repeat(app.password_input.len());
    let input = Paragraph::new(masked)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(input, inner_area[1]);
    
    let help = Paragraph::new("Enter: Submit | Esc: Cancel");
    f.render_widget(help, inner_area[2]);
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
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
