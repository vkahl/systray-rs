// Systray Lib

#[macro_use]
extern crate log;
#[cfg(target_os = "windows")]
extern crate winapi;
#[cfg(target_os = "windows")]
extern crate kernel32;
#[cfg(target_os = "windows")]
extern crate user32;
#[cfg(target_os = "windows")]
extern crate libc;
#[cfg(target_os = "linux")]
extern crate gtk;
#[cfg(target_os = "linux")]
extern crate glib;
#[cfg(target_os = "linux")]
extern crate libappindicator;

pub mod api;

use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Clone, Debug)]
pub enum SystrayError {
    OsError(String),
    NotImplementedError,
    UnknownError,
}

#[derive(Clone, Copy)]
pub struct SystrayEvent {
    menu_index: u32,
}

impl std::fmt::Display for SystrayError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            &SystrayError::OsError(ref err_str) => write!(f, "OsError: {}", err_str),
            &SystrayError::NotImplementedError => write!(f, "Functionality is not implemented yet"),
            &SystrayError::UnknownError => write!(f, "Unknown error occurrred"),
        }
    }
}

impl From<u32> for SystrayEvent {
    fn from(menu_index: u32) -> Self {
        Self{menu_index: menu_index}
    }
}

pub struct Application<'a> {
    window: api::api::Window,
    menu_idx: u32,
    callback: HashMap<u32, Callback<'a>>,
    // Each platform-specific window module will set up its own thread for
    // dealing with the OS main loop. Use this channel for receiving events from
    // that thread.
    rx: Receiver<SystrayEvent>,
    pub tx: Option<Sender<SystrayEvent>>
}

type Callback<'a> = Box<(Fn(&mut Application) -> () + 'a)>;

fn make_callback<'a, F>(f: F) -> Callback<'a>
    where F: std::ops::Fn(&mut Application) -> () + 'a {
    Box::new(f) as Callback<'a>
}

impl<'a> Application<'a> {
    pub fn new() -> Result<Application<'a>, SystrayError> {
        let (event_tx, event_rx) = channel();
        match api::api::Window::new(event_tx.clone()) {
            Ok(w) => Ok(Application {
                window: w,
                menu_idx: 0,
                callback: HashMap::new(),
                rx: event_rx,
                tx: Some(event_tx)
            }),
            Err(e) => Err(e)
        }
    }

    pub fn add_callback<F>(&mut self, f: F) -> u32
        where F: std::ops::Fn(&mut Application) -> () + 'a
    {
        let idx = self.menu_idx;
        self.callback.insert(idx, make_callback(f));
        self.menu_idx += 1;
        idx
    }

    pub fn add_menu_item<F>(&mut self, item_name: &String, f: F) -> Result<u32, SystrayError>
        where F: std::ops::Fn(&mut Application) -> () + 'a
    {
        let idx = self.menu_idx;
        if let Err(e) = self.window.add_menu_entry(idx, item_name) {
            return Err(e);
        }
        Ok(self.add_callback(f))
    }

    pub fn add_menu_separator(&mut self) -> Result<u32, SystrayError> {
        let idx = self.menu_idx;
        if let Err(e) = self.window.add_menu_separator(idx) {
            return Err(e);
        }
        self.menu_idx += 1;
        Ok(idx)
    }

    pub fn set_icon_from_file(&self, file: &String) -> Result<(), SystrayError> {
        self.window.set_icon_from_file(file)
    }

    pub fn set_icon_from_resource(&self, resource: &String) -> Result<(), SystrayError> {
        self.window.set_icon_from_resource(resource)
    }

    pub fn set_icon_from_buffer(&self, buffer: &[u8], width: u32, height: u32) -> Result<(), SystrayError> {
        self.window.set_icon_from_buffer(buffer, width, height)
    }

    pub fn shutdown(&self) -> Result<(), SystrayError> {
        self.window.shutdown()
    }

    pub fn set_tooltip(&self, tooltip: &String) -> Result<(), SystrayError> {
        self.window.set_tooltip(tooltip)
    }

    pub fn quit(&mut self) {
        self.window.quit();
        self.tx = None;
    }

    pub fn wait_for_message(&mut self) {
        loop {
            let msg;
            match self.rx.recv() {
                Ok(m) => msg = m,
                Err(_) => {
                    self.quit();
                    break;
                }
            }
            if let Some(f) = self.callback.remove(&msg.menu_index) {
                f(self);
                self.callback.insert(msg.menu_index, f);
            }
        }
    }
}

impl<'a> Drop for Application<'a> {
    fn drop(&mut self) {
        self.shutdown().unwrap();
        self.tx = None;
    }
}
