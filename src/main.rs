use tvk::lexer::*;
use tvk::parser::*;
use tvk::error::*;
use tvk::env::*;
use tvk::virtual_key::*;

use ratatui::crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*};
use rdev::{listen, EventType};
use std::{
    collections::HashSet,
    io::stdout,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};


use clap::Parser as ClapParser;

#[derive(ClapParser, Debug)]
pub struct Args {
    pub path: Option<String>,
}

struct AppState {
    pressed_keys: HashSet<VirtualKey>,
    kps_events: Vec<Instant>,
}

fn main() -> Result<(), AppError> {

    let mut env = Env::new();
    let args = Args::parse();
    let layout = if let Some(p) = args.path {
        let content = std::fs::read_to_string(p)?;
        let mut lexer = Lexer::new(&content);
        let tokens = lexer.tokenization();
        let mut parser = Parser::new(tokens);
        parser.parse(&mut env)?
    } else {
        return Err(AppError::WrongUsage); 
    };

    let state = Arc::new(Mutex::new(AppState {
        pressed_keys: HashSet::new(),
        kps_events: Vec::new(),
    }));


    let state_clone = Arc::clone(&state);
    thread::spawn(move || {
        listen(move |event| {
            let mut s = state_clone.lock().unwrap();
            match event.event_type {
                EventType::KeyPress(key) => {
                    if let Some(vk) = virtual_key_from_rdev(&key) {
                        s.pressed_keys.insert(vk);
                        s.kps_events.push(Instant::now());
                    }
                }
                EventType::KeyRelease(key) => {
                    if let Some(vk) = virtual_key_from_rdev(&key) {
                        s.pressed_keys.remove(&vk);
                    }
                }
                _ => {}
            }
        }).expect("The KeyEvent listening cannot be started. Please confirm if you have the necessary permissions (such as in macOS Assistive Features)");
    });

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    loop {
        terminal.draw(|f| {
            let mut s = state.lock().unwrap();
            
            // calculate KPS
            let now = Instant::now();
            s.kps_events.retain(|&t| now.duration_since(t) < Duration::from_secs(1));
            let kps = s.kps_events.len();

            tvk::render::render_ui(f, &s.pressed_keys, kps, &layout, &env);
        })?;

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Esc { break; }
            }
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
