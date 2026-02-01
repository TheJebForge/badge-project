use crate::character::repr::{Animation, AnimationFrameSource};
use crate::character::util::AsRichText;
use crate::gui::app::editor::intermediate::{InterAction, InterActionType, InterSequence, SharedLoadedImage};
use crate::gui::app::editor::validation::ValidationError;
use crate::gui::app::editor::{inline_image_picker, inline_image_resource_picker, inline_validation_error, CharacterEditor, SharedInterState, IMAGE_EXTENSIONS};
use crate::gui::app::shared::SharedString;
use crate::gui::app::util::{inline_checkbox, inline_color_edit_rgb_tuple, inline_drag_value, inline_duration_value, inline_enum_edit, inline_folder_picker, inline_resource_picker, inline_style_label, inline_text_edit, pair_list_ui, vec_ui, ChangeTracker, SPACING};
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
                    inline_style_label(ui, "ID:", WIDTH);
                    ui.label(&self.id);
                });

                inline_text_edit(ui, "Name:", &mut self.name, WIDTH, &mut self.tracker);
                inline_text_edit(ui, "Species:", &mut self.species, WIDTH, &mut self.tracker);
                inline_resource_picker(ui, "Default State:", &mut self.default_state, &self.states, WIDTH, &mut self.tracker);
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
                            pair_list_ui(ui, &mut self.images, (), |ui, _i, key, el, _, tracker| {
                                const TEXT_WIDTH: f32 = 60.0;

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

                                ui.separator();

                                ui.horizontal(|ui| {
                                    ui.add_space(ui.style().spacing.item_spacing.x);
                                    ui.label("Target Settings:".rich().size(15.0))
                                });

                                ui.add_space(SPACING);

                                let mut borrowed = el.borrow_mut();

                                inline_drag_value(ui, "Width:", &mut borrowed.width, TEXT_WIDTH, tracker);
                                inline_drag_value(ui, "Height:", &mut borrowed.height, TEXT_WIDTH, tracker);
                                inline_checkbox(ui, "Alpha:", &mut borrowed.alpha, TEXT_WIDTH, tracker);
                                inline_checkbox(ui, "Upscale:", &mut borrowed.upscale, TEXT_WIDTH, tracker);
                            }, &mut self.tracker)
                        });

                        ui.collapsing("Sequences", |ui| {
                            pair_list_ui(ui, &mut self.sequences, &mut self.images, |ui, _, key, el, images, tracker| {
                                sequence_edit_ui(ui, key, el, images, &self.location, tracker, &self.validation_errors)
                            }, &mut self.tracker)
                        });

                        ui.collapsing("Animations", |ui| {
                            pair_list_ui(ui, &mut self.animations, (), |ui, _, key, el, _, tracker| {
                                animation_edit_ui(ui, key, el, &self.location, tracker, &self.validation_errors)
                            }, &mut self.tracker)
                        });

                        ui.collapsing("Actions", |ui| {
                            pair_list_ui(ui, &mut self.actions, (), |ui, _, key, el, _, tracker| {
                                action_edit_ui(ui, key, el, &self.states, tracker, &self.validation_errors)
                            }, &mut self.tracker)
                        });
                    });
            });
    }
}

pub fn sequence_edit_ui(
    ui: &mut Ui,
    key: &mut SharedString,
    element: &mut InterSequence,
    images: &mut Vec<(SharedString, SharedLoadedImage)>,
    location: &PathBuf,
    tracker: &mut ChangeTracker,
    validations: &Vec<ValidationError>
) {
    const TEXT_WIDTH: f32 = 80.0;

    inline_validation_error(
        ui,
        validations,
        "Duplicate name!",
        |err| {
            let ValidationError::DuplicateSequence(name) = err else {
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
            let ValidationError::EmptySequenceName = err else {
                return false;
            };

            key.to_string().is_empty()
        },
        TEXT_WIDTH
    );

    ui.vertical(|ui| {
        ui.label("Frames:");

        vec_ui(ui, &mut element.frames, images, |ui, index, frame, images, tracker| {
            inline_image_resource_picker(
                ui,
                "Image:",
                &mut frame.image,
                images,
                location,
                TEXT_WIDTH,
                tracker,
            );
            inline_validation_error(
                ui,
                validations,
                "Invalid image!",
                |err| {
                    let ValidationError::InvalidImageInSequenceFrame(name, err_index) = err else {
                        return false;
                    };

                    key.str_eq(name) && index == *err_index
                },
                TEXT_WIDTH,
            );
            inline_duration_value(
                ui,
                "Duration:",
                &mut frame.duration,
                TEXT_WIDTH,
                tracker,
            );
        }, tracker);
    });

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

    inline_drag_value(ui, "X:", &mut element.x, TEXT_WIDTH, tracker);
    inline_drag_value(ui, "Y:", &mut element.y, TEXT_WIDTH, tracker);
    inline_drag_value(ui, "Width:", &mut element.width, TEXT_WIDTH, tracker);
    inline_drag_value(ui, "Height:", &mut element.height, TEXT_WIDTH, tracker);

    CollapsingHeader::new("Frames")
        .id_salt(key.0.as_ptr())
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                inline_style_label(ui, "Frame Source:", TEXT_WIDTH);
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
                    inline_folder_picker(ui, "Folder:", folder, location, TEXT_WIDTH, tracker);
                    inline_text_edit(ui, "Extension:", extension, TEXT_WIDTH, tracker);
                    inline_drag_value(ui, "Count:", count, TEXT_WIDTH, tracker);
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

    inline_checkbox(ui, "Clear Screen:", &mut element.clear_screen, TEXT_WIDTH, tracker);

    if element.clear_screen {
        inline_color_edit_rgb_tuple(ui, "Background Color:", &mut element.background_color, TEXT_WIDTH, tracker);
    }

    inline_enum_edit(ui, "Mode:", &mut element.mode, TEXT_WIDTH, tracker);
    inline_checkbox(ui, "Upscale:", &mut element.upscale, TEXT_WIDTH, tracker);
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

    inline_text_edit(ui, "Display Name:", &mut element.display, TEXT_WIDTH, tracker);

    ui.horizontal(|ui| {
        let id = inline_style_label(ui, "Type:", TEXT_WIDTH).response.id;
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
            inline_resource_picker(ui, "State:", state, states, TEXT_WIDTH, tracker);
        }
    }
}