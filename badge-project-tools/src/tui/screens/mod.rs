pub mod file_selector;
mod character_screen;

use std::fmt::Debug;
use ratatui::crossterm::event::Event;
use ratatui::Frame;

pub trait TuiScreen : Debug {
    fn draw(&self, frame: &mut Frame);
    fn handle_events(&mut self, event: &Event) -> anyhow::Result<NavigationAction>;
}

#[derive(Debug)]
pub enum NavigationAction {
    NoOp,
    PopScreen,
    PushScreen(Box<dyn TuiScreen>)
}