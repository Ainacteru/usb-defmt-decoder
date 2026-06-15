use std::{cell::RefCell, io::{Write, stdout}, sync::{Mutex, atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed}}, time::{Duration, Instant}};

use crossterm::{event::{self, Event, KeyCode, KeyEventKind, KeyModifiers}, terminal::{disable_raw_mode, enable_raw_mode}};

use crate::input::InputState::{Continue, Exit, No, Status};

enum InputState<T> {
    Exit,
    Continue,
    No,
    Status(T),
}

impl<T> InputState <T>{
    fn is_exit(&self) -> bool {
       matches!(self, Exit)
    }
}

pub static PAUSE: AtomicBool = AtomicBool::new(false);

pub fn poll_input() {
    enable_raw_mode().unwrap();
    let input = handle_input();
    if input.is_exit() {
        disable_raw_mode().unwrap();
        std::process::exit(0);
        // break;
    } else {
        match input {
            Status("pause") => {
                    PAUSE.store(true,Relaxed);  
                },
            Status("b") => {},
            Continue => {PAUSE.store(false, Relaxed);}
            _ => {}
        }
    }
    disable_raw_mode().unwrap();

}

pub static MESSAGE: Mutex<RefCell<[u8; 64]>> = Mutex::new(RefCell::new([0; 64]));
pub static INDEX: AtomicUsize = AtomicUsize::new(0);
pub static READY: AtomicBool = AtomicBool::new(false);

fn handle_input() -> InputState<&'static str> {
        if event::poll(Duration::from_millis(50)).unwrap() {
    // 3. Read the captured event
        if let Event::Key(key) = event::read().unwrap() && key.kind == KeyEventKind::Press {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match key.code {
                        KeyCode::Char('c') => return program_exit(),
                        KeyCode::Char('s') => return pause(),
                        _ => {},
                    }
                }
                else if PAUSE.load(Relaxed) {
                    match key.code {
                        KeyCode::Char(c) => {
                            MESSAGE.lock().unwrap().borrow_mut()[INDEX.load(Relaxed)] = c as u8;
                            INDEX.fetch_add(1, Relaxed);
                            print!("{}", c);
                            stdout().flush().unwrap();
                        }
                        KeyCode::Enter => {
                            READY.store(true, Relaxed);
                        }
                        _ => {}
                    }
                }
            }
    }


    No
}

fn pause() -> InputState<&'static str>{
    if !PAUSE.load(Relaxed) {
        Status("pause")
    } else {
        Continue
    }
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

    No
}