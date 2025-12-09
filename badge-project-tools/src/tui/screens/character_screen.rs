use crate::character::repr::Character;
use crate::tui::screens::{NavigationAction, TuiScreen};
use crate::tui::util::{make_controls_line, ControlsInfo};
use ratatui::crossterm::event::{Event, KeyCode};
use ratatui::layout::{Constraint, Layout};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub struct CharacterScreen {
    character: Character
}

impl CharacterScreen {
    pub fn new(character: Character) -> Self {
        Self {
            character
        }
    }

    pub fn try_from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let contents = fs::read_to_string(path)?;
        let parsed_character = serde_json::from_str::<Character>(&contents)?;

        Ok(Self::new(parsed_character))
    }

    pub fn new_character(id: &str) -> Self {
        Self::new(Character::from_id(id))
    }
}

const CONTROLS: ControlsInfo = &[
    ("Q", "go back")
];

impl TuiScreen for CharacterScreen {
    fn draw(&self, frame: &mut Frame) {
        let rects = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1)
        ]).split(frame.area());

        frame.render_widget(
            Paragraph::new("Character Editor").centered(),
            rects[0]
        );

        frame.render_widget("Yes", rects[1]);

        frame.render_widget(
            Paragraph::new(make_controls_line(CONTROLS)),
            rects[2]
        );
    }

    fn handle_events(&mut self, event: &Event) -> anyhow::Result<NavigationAction> {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char('q') => {
                    return Ok(NavigationAction::PopScreen)
                }

                _ => {}
            }
        }

        Ok(NavigationAction::NoOp)
    }
}