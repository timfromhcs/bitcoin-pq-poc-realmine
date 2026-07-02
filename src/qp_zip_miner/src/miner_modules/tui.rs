use crossterm::{event::{KeyCode},execute,terminal::{disable_raw_mode,enable_raw_mode,EnterAlternateScreen,LeaveAlternateScreen}};
use ratatui::{prelude::*,widgets::*};
use std::sync::{Arc,Mutex,atomic::{AtomicBool, Ordering}};

pub struct TuiState{
    pub cpu_hashrate:f64,
    pub gpu_hashrate:f64,
    pub vram_used_mb:f64,
    pub ram_used_mb:f64,
    pub shares_accepted:u64,
    pub shares_rejected:u64,
    pub total_hashes:u64,
    pub pool_connected:bool,
    pub log_messages:Vec<String>
}

impl TuiState{
    pub fn new()->Self{
        Self{
            cpu_hashrate:0.0,gpu_hashrate:0.0,vram_used_mb:0.0,ram_used_mb:0.0,
            shares_accepted:0,shares_rejected:0,total_hashes:0,
            pool_connected:false,log_messages:Vec::new()
        }
    }
    pub fn add_log(&mut self,msg:String){
        self.log_messages.push(msg);
        if self.log_messages.len()>100{self.log_messages.remove(0);}
    }
}

pub fn run_tui(state:Arc<Mutex<TuiState>>, running:Arc<AtomicBool>)->Result<(),Box<dyn std::error::Error>>{
    enable_raw_mode()?;
    let mut stdout=std::io::stdout();
    execute!(stdout,EnterAlternateScreen)?;
    let mut terminal=Terminal::new(CrosstermBackend::new(stdout))?;
    
    while running.load(Ordering::Relaxed) {
        let s=match state.lock(){
            Ok(s)=>s,
            Err(_)=>{
                break;
            }
        };
        
        terminal.draw(|f|render_ui(f,&s))?;
        drop(s);
        
        if crossterm::event::poll(std::time::Duration::from_millis(50))?{
            if let crossterm::event::Event::Key(key)=crossterm::event::read()?{
                if key.code==KeyCode::Char('q')||key.code==KeyCode::Esc{
                    running.store(false, Ordering::Relaxed);
                }
            }
        }
    }
    
    disable_raw_mode()?;
    execute!(terminal.backend_mut(),LeaveAlternateScreen)?;
    Ok(())
}

fn render_ui(f:&mut Frame,s:&TuiState){
    let vp=((s.vram_used_mb.min(16000.0))/16000.0*100.0)as u16;
    let rp=((s.ram_used_mb.min(32000.0))/32000.0*100.0)as u16;
    let ch=Layout::default().direction(Direction::Vertical).margin(1)
        .constraints([Constraint::Length(3),Constraint::Length(3),Constraint::Length(8),Constraint::Min(0)])
        .split(f.size());
    f.render_widget(
        Paragraph::new(Span::styled(
            format!("HCSminer v3.0 | Hashes: {} | Pool: {} | [Q]uit",
                s.total_hashes,
                if s.pool_connected{"CONN"}else{"DOWN"}),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        )),
        ch[0]
    );
    f.render_widget(
        Paragraph::new(format!(
            "CPU: {:>8.1} H/s | Total: {:>8.1} H/s | Shares: {}/{}",
            s.cpu_hashrate,
            s.cpu_hashrate+s.gpu_hashrate,
            s.shares_accepted,s.shares_rejected
        )),
        ch[1]
    );
    let gc=Layout::default().direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50),Constraint::Percentage(50)])
        .split(ch[2]);
    f.render_widget(
        Gauge::default()
            .block(Block::default().title("VRAM").borders(Borders::ALL))
            .percent(vp.min(100)),
        gc[0]
    );
    f.render_widget(
        Gauge::default()
            .block(Block::default().title("RAM").borders(Borders::ALL))
            .percent(rp.min(100)),
        gc[1]
    );
    f.render_widget(
        Paragraph::new(
            s.log_messages.iter().rev().take(10)
                .fold(String::new(),|a,b|a+"\n"+b)
        )
        .block(Block::default().title("Log").borders(Borders::ALL)),
        ch[3]
    );
}
