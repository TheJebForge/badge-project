use std::env::current_dir;
use eframe::emath::Vec2;
use eframe::epaint::Color32;
use egui::{Button, Key, Popup, PopupCloseBehavior, Response, RichText, Ui, Widget};
use rfd::FileDialog;
use crate::character::util::AsRichText;
use crate::gui::app::{BoxedGuiPage, GuiPage, PageResponse};
use crate::gui::app::editor::CharacterEditor;

#[derive(Default)]
pub struct StartScreen {
    new_character_id: Option<String>,
    new_character_show_error: bool,
}

impl StartScreen {
    pub fn new() -> BoxedGuiPage {
        Box::new(Self::default())
    }
}

impl GuiPage for StartScreen {
    fn show(&mut self, ui: &mut Ui) -> PageResponse {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() / 2.0 - 100.0);

            ui.label("Character Editor".rich().size(40.0));
            ui.label("Choose how to start:");

            ui.add_space(10.0);

            let new_file = ui.add(
                Button::new("New File".rich().size(25.0))
                    .min_size(Vec2::new(200.0, 50.0))
            );

            let open_file = ui.add(
                Button::new("Open File".rich().size(25.0))
                    .min_size(Vec2::new(200.0, 50.0))
            );

            if open_file.clicked() {
                if let Some(picked_file) = FileDialog::new()
                    .add_filter("Character JSON", &["json"])
                    .set_directory(current_dir().unwrap())
                    .pick_file() {
                    match CharacterEditor::open_file(picked_file) {
                        Ok(page) => {
                            return PageResponse::SwitchPage(page)
                        }
                        Err(err) => {
                            println!("Failed! {err}")
                        }
                    }
                }
            }

            let mut just_opened = false;

            if new_file.clicked() && self.new_character_id.is_none() {
                self.new_character_id = Some("".to_string());
                just_opened = true;
            }

            if self.new_character_id.is_some() {
                if let Some(response) = Popup::from_response(&new_file)
                    .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                    .show(|ui| {
                        ui.label("Enter ID");

                        let text_resp = ui.text_edit_singleline(self.new_character_id.as_mut().unwrap());

                        if just_opened {
                            ui.memory_mut(|mem| mem.request_focus(text_resp.id))
                        }

                        if self.new_character_show_error {
                            ui.label("ID cannot be empty!".rich().color(Color32::RED));
                        }

                        if ui.button("Create").clicked()
                            || text_resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                            let str = self.new_character_id.clone().unwrap();

                            if str.is_empty() {
                                self.new_character_show_error = true;
                            } else {
                                self.new_character_id = None;
                                self.new_character_show_error = false;
                                return Some(str);
                            }
                        }

                        None
                    }) {

                    if response.response.should_close() {
                        self.new_character_id = None;
                    }

                    if let Some(new_file) = response.inner {
                        return PageResponse::SwitchPage(CharacterEditor::new_file(&new_file))
                    }
                }
            }

            PageResponse::Nothing
        }).inner
    }
}