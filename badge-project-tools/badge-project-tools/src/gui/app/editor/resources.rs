use crate::character::repr::{Animation, AnimationFrameSource};
use crate::character::util::AsRichText;
use crate::gui::app::editor::intermediate::{InterAction, InterActionType};
use crate::gui::app::editor::validation::ValidationError;
use crate::gui::app::editor::{inline_image_picker, inline_validation_error, CharacterEditor, SharedInterState, IMAGE_EXTENSIONS};
use crate::gui::app::shared::SharedString;
use crate::gui::app::util;
use crate::gui::app::util::{ChangeTracker, SPACING};
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

                util::inline_text_edit(ui, "Name:", &mut self.name, WIDTH, &mut self.tracker);
                util::inline_text_edit(ui, "Species:", &mut self.species, WIDTH, &mut self.tracker);
                util::inline_resource_picker(ui, "Default State:", &mut self.default_state, &self.states, WIDTH, &mut self.tracker);
                inline_validation_error(
                    ui,
                    &self.validation_errors,
                    "Invalid state!",
                    |err| {
                        let ValidationError::InvalidDefaultState = err else {
                            return false;
                        };

                        true
                    },
                    WIDTH
                );
            });

        CentralPanel::default()
            .show(ui.ctx(), |ui| {
                ScrollArea::vertical()
                    .show(ui, |ui| {
                        ui.collapsing("Images", |ui| {
                            util::pair_list_ui(ui, &mut self.images, |ui, _i, key, el, tracker| {
                                const TEXT_WIDTH: f32 = 50.0;

                                inline_validation_error(
                                    ui,
                                    &self.validation_errors,
                                    "Duplicate name!",
                                    |err| {
                                        let ValidationError::DuplicateImage(name) = err else {
                                            return false;
                                        };

                                        key.str_eq(name)
                                    },
                                    TEXT_WIDTH
                                );

                                inline_validation_error(
                                    ui,
                                    &self.validation_errors,
                                    "Empty name!",
                                    |err| {
                                        let ValidationError::EmptyImageName = err else {
                                            return false;
                                        };

                                        key.to_string().is_empty()
                                    },
                                    TEXT_WIDTH
                                );

                                inline_image_picker(ui, "Image:", el, &self.location, TEXT_WIDTH, tracker);
                            }, &mut self.tracker)
                        });

                        ui.collapsing("Animations", |ui| {
                            util::pair_list_ui(ui, &mut self.animations, |ui, _, key, el, tracker| {
                                animation_edit_ui(ui, key, el, &self.location, tracker, &self.validation_errors)
                            }, &mut self.tracker)
                        });

                        ui.collapsing("Actions", |ui| {
                            util::pair_list_ui(ui, &mut self.actions, |ui, _, key, el, tracker| {
                                action_edit_ui(ui, key, el, &self.states, tracker, &self.validation_errors)
                            }, &mut self.tracker)
                        });
                    });
            });
    }
}

pub fn animation_edit_ui(
    ui: &mut Ui,
    key: &mut SharedString,
    element: &mut Animation,
    location: &PathBuf,
    tracker: &mut ChangeTracker,
    validations: &Vec<ValidationError>
) {
    const TEXT_WIDTH: f32 = 120.0;

    inline_validation_error(
        ui,
        validations,
        "Duplicate name!",
        |err| {
            let ValidationError::DuplicateAnimation(name) = err else {
                return false;
            };

            key.str_eq(name)
        },
        TEXT_WIDTH
    );

    inline_validation_error(
        ui,
        validations,
        "Empty name!",
        |err| {
            let ValidationError::EmptyAnimationName = err else {
                return false;
            };

            key.to_string().is_empty()
        },
        TEXT_WIDTH
    );

    util::inline_drag_value(ui, "X:", &mut element.x, TEXT_WIDTH, tracker);
    util::inline_drag_value(ui, "Y:", &mut element.y, TEXT_WIDTH, tracker);
    util::inline_drag_value(ui, "Width:", &mut element.width, TEXT_WIDTH, tracker);
    util::inline_drag_value(ui, "Height:", &mut element.height, TEXT_WIDTH, tracker);

    CollapsingHeader::new("Frames")
        .id_salt(key.0.as_ptr())
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                util::inline_style_label(ui, "Frame Source:", TEXT_WIDTH);
                if ui.radio(element.frames.is_indexed(), "Indexed").clicked() {
                    element.frames = AnimationFrameSource::Indexed {
                        folder: Default::default(),
                        extension: "".to_string(),
                        count: 0,
                    };
                    tracker.mark_change();
                }

                if ui.radio(element.frames.is_list(), "List").clicked() {
                    element.frames = AnimationFrameSource::List(vec![]);
                    tracker.mark_change();
                }
            });

            match &mut element.frames {
                AnimationFrameSource::Indexed { folder, extension, count } => {
                    util::inline_folder_picker(ui, "Folder:", folder, location, TEXT_WIDTH, tracker);
                    util::inline_text_edit(ui, "Extension:", extension, TEXT_WIDTH, tracker);
                    util::inline_drag_value(ui, "Count:", count, TEXT_WIDTH, tracker);
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

    util::inline_checkbox(ui, "Clear Screen:", &mut element.clear_screen, TEXT_WIDTH, tracker);

    if element.clear_screen {
        util::inline_color_edit_rgb_tuple(ui, "Background Color:", &mut element.background_color, TEXT_WIDTH, tracker);
    }

    util::inline_enum_edit(ui, "Mode:", &mut element.mode, TEXT_WIDTH, tracker);
    util::inline_checkbox(ui, "Upscale:", &mut element.upscale, TEXT_WIDTH, tracker);
}

pub fn action_edit_ui(
    ui: &mut Ui,
    key: &mut String,
    element: &mut InterAction,
    states: &Vec<(SharedString, SharedInterState)>,
    tracker: &mut ChangeTracker,
    validations: &Vec<ValidationError>
) {
    const TEXT_WIDTH: f32 = 100.0;

    inline_validation_error(
        ui,
        validations,
        "Duplicate name!",
        |err| {
            let ValidationError::DuplicateAction(name) = err else {
                return false;
            };

            key == name
        },
        TEXT_WIDTH
    );

    inline_validation_error(
        ui,
        validations,
        "Empty name!",
        |err| {
            let ValidationError::EmptyActionName = err else {
                return false;
            };

            key.is_empty()
        },
        TEXT_WIDTH
    );

    util::inline_text_edit(ui, "Display Name:", &mut element.display, TEXT_WIDTH, tracker);

    ui.horizontal(|ui| {
        let id = util::inline_style_label(ui, "Type:", TEXT_WIDTH).response.id;
        ComboBox::new(id.with("combo"), "")
            .selected_text(element.ty.rich())
            .show_ui(ui, |ui| {
                let ty = &mut element.ty;

                if ui.selectable_label(ty.is_none(), "None").clicked() {
                    *ty = InterActionType::None;
                    tracker.mark_change();
                }

                if ui.selectable_label(ty.is_switch_state(), "Switch State").clicked() {
                    if let Some((key, _)) = states.first() {
                        *ty = InterActionType::SwitchState(key.clone());
                        tracker.mark_change();
                    }
                }
            });
    });

    match &mut element.ty {
        InterActionType::None => {}
        InterActionType::SwitchState(state) => {
            util::inline_resource_picker(ui, "State:", state, states, TEXT_WIDTH, tracker);
        }
    }
}