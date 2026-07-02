use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::*,
};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct TuiState {
    pub cpu_hashrate: f64,
    pub gpu_hashrate: f64,
    pub vram_used_mb: f64,
    pub ram_used_mb: f64,
    pub prob_acceptance_rate: f64,
    pub shares_accepted: u64,
    pub shares_rejected: u64,
    pub total_hashes: u64,
    pub running: bool,
    pub log_messages: Vec<String>,
}

impl TuiState {
    pub fn new() -> Self {
        Self {
            cpu_hashrate: 0.0,
            gpu_hashrate: 0.0,
            vram_used_mb: 0.0,
            ram_used_mb: 0.0,
            prob_acceptance_rate: 0.0,
            shares_accepted: 0,
            shares_rejected: 0,
            total_hashes: 0,
            running: true,
            log_messages: Vec::new(),
        }
    }
    pub fn add_log(&mut self, msg: String) {
        self.log_messages.push(msg);
        if self.log_messages.len() > 100 {
            self.log_messages.remove(0);
        }
    }
}

pub fn run_tui(state: Arc<Mutex<TuiState>>) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let mut last_update = Instant::now();
    while state.lock().unwrap().running {
        terminal.draw(|f| render_ui(f, &state.lock().unwrap()))?;
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                    state.lock().unwrap().running = false;
                }
            }
        }
        if last_update.elapsed().as_secs_f64() > 0.5 {
            last_update = Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn render_ui(f: &mut Frame, state: &TuiState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(8),
            Constraint::Min(0),
        ])
        .split(f.size());

    // Title
    let title = Paragraph::new(Span::styled(
        format!("BIP-QP-ZIP MTP MINER v2.0  |  Hashes: {}  |  [Q]uit", state.total_hashes),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    ));
    f.render_widget(title, chunks[0]);

    // Hashrate
    let hashrate_text = format!(
        "CPU: {:>8.1} H/s  |  GPU: {:>8.1} H/s  |  Total: {:>8.1} H/s  |  Prob. Acceptance: {:>5.1}%",
        state.cpu_hashrate, state.gpu_hashrate,
        state.cpu_hashrate + state.gpu_hashrate,
        state.prob_acceptance_rate * 100.0
    );
    f.render_widget(Paragraph::new(hashrate_text), chunks[1]);

    // Gauges
    let gauge_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);
    let vram_gauge = Gauge::default()
        .block(Block::default().title("VRAM Usage").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Green))
        .percent((state.vram_used_mb / 2048.0 * 100.0) as u16);
    f.render_widget(vram_gauge, gauge_chunks[0]);
    let ram_gauge = Gauge::default()
        .block(Block::default().title("RAM Usage").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Yellow))
        .percent((state.ram_used_mb / 16000.0 * 100.0) as u16);
    f.render_widget(ram_gauge, gauge_chunks[1]);

    // Log
    let log_text: String = state.log_messages.iter().rev().take(10).fold(String::new(), |a, b| a + "\n" + b);
    f.render_widget(
        Paragraph::new(log_text)
            .block(Block::default().title("Mining Log").borders(Borders::ALL))
            .style(Style::default().fg(Color::White)),
        chunks[3],
    );
}
