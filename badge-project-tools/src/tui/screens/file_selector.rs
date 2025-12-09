use crate::tui::screens::character_screen::CharacterScreen;
use crate::tui::screens::{NavigationAction, TuiScreen};
use crate::tui::util::{make_controls_line, ControlsInfo, Popup};
use ratatui::crossterm::event::{Event, KeyCode};
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use ratatui_explorer::{FileExplorer, Theme};
use tui_textarea::TextArea;

#[derive(Debug)]
pub struct FileSelectionScreen<'a> {
    explorer: FileExplorer,
    error: Option<(String, String)>,
    new_dialog: Option<TextArea<'a>>
}

impl FileSelectionScreen<'_> {
    pub fn new() -> Self {
        Self {
            explorer: FileExplorer::with_theme(
                Theme::default()
                    .add_default_title()
            ).unwrap(),
            error: None,
            new_dialog: None,
        }
    }
}

const CONTROLS: ControlsInfo = &[
    ("Q", "exit"),
    ("Enter", "select file"),
    ("N", "create new character")
];

const ERROR_CONTROLS: ControlsInfo = &[
    ("Any", "close")
];

const NEW_DIALOG_CONTROLS: ControlsInfo = &[
    ("Esc", "cancel"),
    ("Enter", "create")
];

impl TuiScreen for FileSelectionScreen<'_> {
    fn draw(&self, frame: &mut Frame) {
        let rects = Layout::vertical([
             Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1)
        ]).split(frame.area());

        frame.render_widget(
            Paragraph::new("Select character's json file")
                .centered(),
            rects[0]
        );

        frame.render_widget(&self.explorer.widget(), rects[1]);

        let area = frame.area();

        let controls = if let Some((title, err)) = &self.error {
            let popup_area = Rect {
                x: area.width / 4,
                y: area.height / 3,
                width: area.width / 2,
                height: area.height / 3,
            };

            frame.render_widget(
                Popup::default()
                    .title(format!(" {} ", title))
                    .title_style(Style::new().light_red())
                    .border_style(Style::new().light_red())
                    .content(err.as_str()),
                popup_area
            );

            ERROR_CONTROLS
        } else if let Some(textarea) = &self.new_dialog {
            let popup_area = Rect {
                x: area.width / 4,
                y: area.height / 2 - 1,
                width: area.width / 2,
                height: 3,
            };

            frame.render_widget(Clear::default(), popup_area);

            frame.render_widget(
                Block::new()
                    .title(" Input new file name ")
                    .borders(Borders::all()),
                popup_area
            );

            let inner_area = popup_area.inner(Margin::new(1, 1));
            frame.render_widget(
                textarea,
                inner_area
            );

            NEW_DIALOG_CONTROLS
        } else {
            CONTROLS
        };

        frame.render_widget(
            Paragraph::new(make_controls_line(controls)),
            rects[2]
        );
    }

    fn handle_events(&mut self, event: &Event) -> anyhow::Result<NavigationAction> {
        if self.error.is_some() {
            if let Event::Key(_) = event {
                self.error = None;
            }

            return Ok(NavigationAction::NoOp);
        }

        if let Some(textarea) = &mut self.new_dialog {
            if let Event::Key(key) = event {
                match key.code {
                    KeyCode::Esc => {
                        self.new_dialog = None;
                        return Ok(NavigationAction::NoOp)
                    }

                    KeyCode::Enter => {
                        let id = textarea.lines()
                            .first()
                            .map(String::clone)
                            .unwrap_or("unnamed".to_string());

                        self.new_dialog = None;

                        return Ok(NavigationAction::PushScreen(
                            Box::new(CharacterScreen::new_character(&id))
                        ))
                    }

                    _ => {}
                }

                textarea.input(key.clone());
            }

            return Ok(NavigationAction::NoOp);
        }

        match event {
            Event::Key(key) => {
                match key.code {
                    KeyCode::Char('q') => { // Exit action
                        return Ok(NavigationAction::PopScreen)
                    }

                    KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => { // File selection
                        let selection = self.explorer.current().clone();

                        if selection.is_dir() {
                            self.explorer.set_cwd(selection.path())?;
                        } else {
                            match CharacterScreen::try_from_file(selection.path()) {
                                Ok(screen) => {
                                    return Ok(NavigationAction::PushScreen(Box::new(screen)))
                                }
                                Err(err) => {
                                    self.error = Some((
                                        "Failed to open file".to_string(),
                                        err.to_string()
                                    ));
                                }
                            }
                        }

                        return Ok(NavigationAction::NoOp)
                    }

                    KeyCode::Char('n') => {
                        let mut textarea = TextArea::default();
                        textarea.set_cursor_line_style(Style::new());

                        self.new_dialog = Some(textarea);

                        return Ok(NavigationAction::NoOp)
                    }

                    _ => {}
                }
            }
            _ => {}
        }

        self.explorer.handle(event)?;


        Ok(NavigationAction::NoOp)
    }
}