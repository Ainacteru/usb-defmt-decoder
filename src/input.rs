use std::{sync::{Mutex, atomic::{AtomicBool, Ordering::Relaxed}}, time::{Duration, Instant}};

use crossterm::{event::{self, Event, KeyCode, KeyEventKind, KeyModifiers}, terminal::{disable_raw_mode, enable_raw_mode}};

use crate::input::InputState::{Continue, Exit, Status};

enum InputState<T> {
    Exit,
    Continue,
    Status(T),
}

impl<T> InputState <T>{
    fn is_exit(&self) -> bool {
       matches!(self, Exit)
    }
}

pub static STOP: AtomicBool = AtomicBool::new(false);

pub fn poll_input() {
    enable_raw_mode().unwrap();
    let input = handle_input();
    if input.is_exit() {
        disable_raw_mode().unwrap();
        std::process::exit(0);
        // break;
    } else {
        match input {
            Status("stop") => {STOP.store(true,Relaxed);},
            Status("b") => {},
            _ => {}
        }
    }
    disable_raw_mode().unwrap();

}

fn handle_input() -> InputState<&'static str> {
        if event::poll(Duration::from_millis(50)).unwrap() {
    // 3. Read the captured event
        if let Event::Key(key) = event::read().unwrap() {
            // Focus only on the initial Press event (ignoring Release/Repeat)
            if key.kind == KeyEventKind::Press {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match key.code {
                        KeyCode::Char('c') => return program_exit(),
                        KeyCode::Char('s') => return Status("stop"),
                        _ => {},
                    }
                }
                else {
                    match key.code {
                        KeyCode::Char(c) => {
                            print!("You pressed character: {}\r\n", c);
                        }
                        KeyCode::Left => {
                            println!("Left arrow key pressed!");
                        }
                        _ => {}
                    }
                }
            }
        }
    }


    Continue
}

static LAST_CTRL_C: Mutex<Option<Instant>> = Mutex::new(None);
pub static SHOW_WARNING_UNTIL: Mutex<Option<Instant>> = Mutex::new(None);

fn program_exit() -> InputState<&'static str> {

    let now = Instant::now();
    let mut last = LAST_CTRL_C.lock().unwrap();

    match *last {
        Some(t) if now.duration_since(t) < Duration::from_millis(1000) => {
            print!("\x1b[31mStopping!\x1b[0m\r\n");
            disable_raw_mode().unwrap();
            return Exit;
        }
        _ => { 
            *last = Some(now);
            *SHOW_WARNING_UNTIL.lock().unwrap() = Some(Instant::now() + Duration::from_millis(1000));
            
            print!("\x1b[31mPress Ctrl+C twice to exit\x1b[0m\r\n");
        }
    }

    Continue
}