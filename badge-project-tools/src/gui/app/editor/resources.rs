use crate::character::repr::{Animation, AnimationFrameSource};
use crate::character::util::AsRichText;
use crate::gui::app::editor::intermediate::{InterAction, InterActionType, InterState};
use crate::gui::app::editor::{inline_image_picker, CharacterEditor, SharedInterState, IMAGE_EXTENSIONS};
use crate::gui::app::shared::SharedString;
use crate::gui::app::util;
use crate::gui::app::util::{SPACING};
use egui::{CentralPanel, CollapsingHeader, ComboBox, ScrollArea, SidePanel, Ui};
use std::path::PathBuf;

impl CharacterEditor {
    pub(crate) fn resources_ui(&mut self, ui: &mut Ui) {
        SidePanel::left("resources.left")
            .default_width(250.0)
            .show(ui.ctx(), |ui| {
                ui.heading("Character Info");

                const WIDTH: f32 = 80.0;

                ui.separator();

                ui.horizontal(|ui| {
                    util::inline_style_label(ui, "ID:", WIDTH);
                    ui.label(&self.id);
                });

                util::inline_text_edit(ui, "Name:", &mut self.name, WIDTH);
                util::inline_text_edit(ui, "Species:", &mut self.species, WIDTH);
                util::inline_resource_picker(ui, "Default State:", &mut self.default_state, &self.states, WIDTH);
            });

        CentralPanel::default()
            .show(ui.ctx(), |ui| {
                ScrollArea::vertical()
                    .show(ui, |ui| {
                        ui.collapsing("Images", |ui| {
                            util::list_ui(ui, &mut self.images, |ui, i, key, el| {
                                inline_image_picker(ui, "Image:", el, &self.location, 50.0);
                            })
                        });

                        ui.collapsing("Animations", |ui| {
                            util::list_ui(ui, &mut self.animations, |ui, _, key, el| {
                                animation_edit_ui(ui, key, el, &self.location)
                            })
                        });

                        ui.collapsing("Actions", |ui| {
                            util::list_ui(ui, &mut self.actions, |ui, _, key, el| {
                                action_edit_ui(ui, el, &self.states)
                            })
                        });
                    });
            });
    }
}

pub fn animation_edit_ui(ui: &mut Ui, key: &mut SharedString, element: &mut Animation, location: &PathBuf) {
    const TEXT_WIDTH: f32 = 120.0;

    util::inline_drag_value(ui, "X:", &mut element.x, TEXT_WIDTH);
    util::inline_drag_value(ui, "Y:", &mut element.y, TEXT_WIDTH);
    util::inline_drag_value(ui, "Width:", &mut element.width, TEXT_WIDTH);
    util::inline_drag_value(ui, "Height:", &mut element.height, TEXT_WIDTH);

    CollapsingHeader::new("Frames")
        .id_salt(key)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                util::inline_style_label(ui, "Frame Source:", TEXT_WIDTH);
                if ui.radio(element.frames.is_indexed(), "Indexed").clicked() {
                    element.frames = AnimationFrameSource::Indexed {
                        folder: Default::default(),
                        extension: "".to_string(),
                        count: 0,
                    }
                }

                if ui.radio(element.frames.is_list(), "List").clicked() {
                    element.frames = AnimationFrameSource::List(vec![])
                }
            });

            match &mut element.frames {
                AnimationFrameSource::Indexed { folder, extension, count } => {
                    util::inline_folder_picker(ui, "Folder:", folder, location, TEXT_WIDTH);
                    util::inline_text_edit(ui, "Extension:", extension, TEXT_WIDTH);
                    util::inline_drag_value(ui, "Count:", count, TEXT_WIDTH);
                }

                AnimationFrameSource::List(list) => {
                    ui.horizontal(|ui| {
                        if ui.button("Pick Files").clicked() {
                            if let Some(picked_files) = rfd::FileDialog::new()
                                .set_title("Pick images")
                                .add_filter("Image", IMAGE_EXTENSIONS)
                                .set_directory(location)
                                .pick_files() {
                                list.extend(
                                    picked_files.into_iter()
                                        .map(|path| {
                                            match path.strip_prefix(location) {
                                                Ok(stripped) => stripped.to_path_buf(),
                                                Err(_) => path
                                            }
                                        })
                                )
                            }
                        }

                        if ui.button("Clear").clicked() {
                            list.clear();
                        }
                    });

                    ui.separator();

                    ui.horizontal_wrapped(|ui| {
                        let mut to_delete = None;

                        for (index, item) in list.iter_mut().enumerate() {
                            if ui.button(format!("X {}", item.to_string_lossy())).clicked() {
                                to_delete = Some(index)
                            }
                        }

                        if let Some(index) = to_delete {
                            list.remove(index);
                        }
                    });

                    ui.add_space(SPACING);
                }
            }
        });

    ui.add_space(SPACING);

    util::inline_checkbox(ui, "Clear Screen:", &mut element.clear_screen, TEXT_WIDTH);

    if element.clear_screen {
        util::inline_color_edit_rgb_tuple(ui, "Background Color:", &mut element.background_color, TEXT_WIDTH);
    }

    util::inline_enum_edit(ui, "Mode:", &mut element.mode, TEXT_WIDTH);
    util::inline_checkbox(ui, "Upscale:", &mut element.upscale, TEXT_WIDTH);
}

pub fn action_edit_ui(ui: &mut Ui, element: &mut InterAction, states: &Vec<(SharedString, SharedInterState)>) {
    const TEXT_WIDTH: f32 = 100.0;

    util::inline_text_edit(ui, "Display Name:", &mut element.display, TEXT_WIDTH);

    ui.horizontal(|ui| {
        let id = util::inline_style_label(ui, "Type:", TEXT_WIDTH).response.id;
        ComboBox::new(id.with("combo"), "")
            .selected_text(element.ty.rich())
            .show_ui(ui, |ui| {
                let ty = &mut element.ty;

                if ui.selectable_label(ty.is_none(), "None").clicked() {
                    *ty = InterActionType::None
                }

                if ui.selectable_label(ty.is_switch_state(), "Switch State").clicked() {
                    if let Some((key, _)) = states.first() {
                        *ty = InterActionType::SwitchState(key.clone())
                    }
                }
            });
    });

    match &mut element.ty {
        InterActionType::None => {}
        InterActionType::SwitchState(state) => {
            util::inline_resource_picker(ui, "State:", state, states, TEXT_WIDTH);
        }
    }
}