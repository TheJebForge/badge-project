pub mod screens;
pub mod util;

use crate::tui::screens::file_selector::FileSelectionScreen;
use crate::tui::screens::{NavigationAction, TuiScreen};
use ratatui::crossterm::event;
use ratatui::{DefaultTerminal, Frame};

#[derive(clap::Parser, Debug)]
#[command(
    about="Enters terminal interface mode for the program",
    long_about=None
)]
pub struct TuiCli;

pub fn start_tui() -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    let result = TuiApp::new().run(&mut terminal);
    ratatui::restore();
    result
}

#[derive(Debug)]
pub struct TuiApp {
    exit: bool,
    screen_stack: Vec<Box<dyn TuiScreen>>
}

impl TuiApp {
    pub fn new() -> Self {
        Self {
            exit: false,
            screen_stack: vec![
                Box::new(FileSelectionScreen::new())
            ]
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let Some(screen) = self.screen_stack.last() else {
            return;
        };

        screen.draw(frame);
    }

    fn handle_events(&mut self) -> anyhow::Result<()> {
        let ev = event::read()?;

        if self.screen_stack.is_empty() {
            return self.exit()
        }

        let top_most = self.screen_stack.last_mut().unwrap();
        match top_most.handle_events(&ev)? {
            NavigationAction::NoOp => {}

            NavigationAction::PopScreen => {
                if self.screen_stack.len() > 1 {
                    self.screen_stack.pop();
                } else {
                    return self.exit()
                }
            }

            NavigationAction::PushScreen(new_screen) => {
                self.screen_stack.push(new_screen)
            }
        }

        Ok(())
    }

    fn exit(&mut self) -> anyhow::Result<()> {
        self.exit = true;
        Ok(())
    }
}