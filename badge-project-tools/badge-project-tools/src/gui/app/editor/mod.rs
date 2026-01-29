mod intermediate;
mod resources;
mod nodes;
mod validation;

use crate::character::repr::{Animation, Character, State};
use crate::character::util::AsRichText;
use crate::gui::app::editor::intermediate::{find_images, InterAction, InterState, LoadedImage, SharedInterState, SharedLoadedImage};
use crate::gui::app::editor::nodes::{snarl_from_states, snarl_style, ViewerSelection};
use crate::gui::app::shared::SharedString;
use crate::gui::app::start::StartScreen;
use crate::gui::app::{util, BoxedGuiPage, GuiPage, PageResponse};
use anyhow::anyhow;
use egui::containers::menu::MenuButton;
use egui::{vec2, Button, CentralPanel, Color32, ColorImage, ComboBox, Image, InnerResponse, Key, Sense, TextureHandle, TextureOptions, TopBottomPanel, Ui, WidgetText};
use egui_snarl::ui::SnarlStyle;
use egui_snarl::Snarl;
use std::cell::{RefCell, RefMut};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::{env, fs};
use std::fmt::Display;
use std::time::Instant;
use strum::{Display, EnumIter, IntoEnumIterator};
use crate::character::process_character_archive;
use crate::gui::app::editor::validation::ValidationError;
use crate::gui::app::util::{inline_style_label, pick_unique_name, ChangeTracker};

pub struct CharacterEditor {
    tab: EditorTab,
    location: PathBuf,
    file_path: Option<PathBuf>,
    last_save: Option<Instant>,
    id: String,
    name: String,
    species: String,
    default_state: SharedString,
    images: Vec<(SharedString, SharedLoadedImage)>,
    animations: Vec<(SharedString, Animation)>,
    actions: Vec<(String, InterAction)>,
    states: Vec<(SharedString, SharedInterState)>,
    state_graph: Snarl<(SharedString, SharedInterState)>,
    graph_style: SnarlStyle,
    graph_selection: ViewerSelection,
    tracker: ChangeTracker,
    validation_errors: Vec<ValidationError>
}

#[derive(Copy, Clone, EnumIter, Default, Display, Eq, PartialEq)]
enum EditorTab {
    #[default]
    Resources,
    StateMachine
}

impl CharacterEditor {
    pub fn from_character(mut char: Character, location: PathBuf, original: Option<PathBuf>) -> CharacterEditor {
        let images = find_images(&char.states, &location);

        let animations = char.animations.into_iter()
            .map(|(k, v)| (k.into(), v))
            .collect();

        if char.states.is_empty() {
            char.states.insert("idle".to_string(), State::default());
        }

        let state_names = char.states.keys()
            .map(|k| k.clone().into())
            .collect::<Vec<SharedString>>();

        let states = char.states.into_iter()
            .filter_map(|(k, v)| Some((
                state_names.iter().find(|n| n.str_eq(&k))?.clone(),
                Rc::new(RefCell::new(InterState::from_state(
                    v,
                    &state_names,
                    &images,
                    &animations,
                )?))
            )))
            .collect::<Vec<_>>();

        let actions = char.actions.into_iter()
            .filter_map(|(k, v)| Some((k, InterAction::from_action(v, &states)?)))
            .collect();

        let mut state = Self {
            tab: EditorTab::default(),
            location,
            file_path: original,
            last_save: None,
            id: char.id,
            name: char.name,
            species: char.species,
            default_state: states.iter().find(|(k, _)| k.str_eq(&char.default_state))
                .or_else(|| states.first()).unwrap().0.clone(),
            images,
            animations,
            actions,
            state_graph: snarl_from_states(&states),
            states,
            graph_style: snarl_style(),
            graph_selection: ViewerSelection::default(),
            tracker: Default::default(),
            validation_errors: vec![],
        };

        state.validation_errors = state.validate_state();

        state
    }

    pub fn new_file(id: &str) -> BoxedGuiPage {
        Box::new(Self::from_character(Character::from_id(id), env::current_dir().unwrap(), None))
    }

    pub fn open_file(path: impl AsRef<Path>) -> anyhow::Result<BoxedGuiPage> {
        let path = path.as_ref().to_path_buf();

        let contents = fs::read_to_string(&path)?;
        let character = serde_json::from_str::<Character>(&contents)?;

        let location = if let Some(parent) = path.parent() {
            parent.to_path_buf()
        } else {
            path.clone()
        };

        Ok(Box::new(Self::from_character(character, location, Some(path))))
    }

    pub fn as_repr(&self) -> Character {
        Character {
            id: self.id.clone(),
            name: self.name.clone(),
            species: self.species.clone(),
            default_state: self.default_state.to_string(),
            states: self.states.iter()
                .filter_map(|(k, v)| Some(
                    (k.to_string(), v.borrow().clone().into_state(&self.images)?)
                ))
                .collect(),
            animations: self.animations.iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            actions: self.actions.iter()
                .filter_map(|(k, v)| Some((k.to_string(), v.clone().into_action()?)))
                .collect(),
        }
    }

    pub fn save_file(&mut self, save_over_original: bool) -> anyhow::Result<()> {
        let pick_file = || {
            let Some(picked_file) = rfd::FileDialog::new()
                .set_title("Save character JSON file")
                .add_filter("Character File", &["json"])
                .set_directory(&self.location)
                .save_file()
            else {
                return Err(anyhow!("Cancelled!"));
            };

            Ok(picked_file)
        };

        let path = if !save_over_original {
            pick_file()?
        } else {
            if let Some(original) = &self.file_path {
                original.clone()
            } else {
                pick_file()?
            }
        };

        self.file_path = Some(path.clone());

        let char = self.as_repr();
        let serialized = serde_json::to_string(&char)?;
        fs::write(path, serialized)?;

        self.tracker.mark_saved();
        self.last_save = Some(Instant::now());

        Ok(())
    }

    pub fn save(&mut self) {
        match self.save_file(true) {
            Ok(_) => {}
            Err(err) => println!("Error while saving: {err}")
        };
    }

    pub fn save_as(&mut self) {
        match self.save_file(false) {
            Ok(_) => {}
            Err(err) => eprintln!("Error while saving: {err}")
        };
    }

    pub fn export_character(&self) -> anyhow::Result<()> {
        let Some(picked_file) = rfd::FileDialog::new()
            .set_title("Save character archive file")
            .add_filter("Character Archive", &["tar"])
            .set_directory(&self.location)
            .save_file()
        else {
            return Err(anyhow!("Cancelled!"));
        };

        let char = self.as_repr();

        process_character_archive(char, picked_file)
    }

    pub fn export(&self) {
        match self.export_character() {
            Ok(_) => {}
            Err(err) => eprintln!("Error while exporting: {err}")
        }
    }
}

pub const IMAGE_EXTENSIONS: &[&'static str] = &["png", "jpg", "jpeg", "bmp", "tga", "tiff"];

const IMAGE_SIZE: f32 = 200.0;

fn pick_image_filepath(location: impl AsRef<Path>) -> Option<PathBuf> {
    let location = location.as_ref();

    let Some(picked_file) = rfd::FileDialog::new()
        .set_title("Pick image file")
        .add_filter("Image", IMAGE_EXTENSIONS)
        .set_directory(&location)
        .pick_file()
    else {
        return None;
    };

    let stripped = match picked_file.strip_prefix(&location) {
        Ok(relative) => relative.to_path_buf(),
        Err(_) => picked_file
    };

    Some(stripped)
}

fn try_set_loaded_image(
    path: impl AsRef<Path>,
    value: &mut RefMut<LoadedImage>,
    tracker: &mut ChangeTracker,
    location: impl AsRef<Path>,
) {
    let path = path.as_ref();
    let location = location.as_ref();

    let image = match util::load_image(location.join(path)) {
        Ok(image) => image,
        Err(err) => {
            eprintln!("Failed to load image! {err}");
            return;
        }
    };

    value.image = image;
    value.path = path.to_path_buf();
    value.handle = None;

    tracker.mark_change()
}

fn pick_image_file(value: &mut RefMut<LoadedImage>, tracker: &mut ChangeTracker, location: impl AsRef<Path>) {
    let location = location.as_ref();

    let Some(stripped) = pick_image_filepath(location) else {
        return
    };

    try_set_loaded_image(stripped, value, tracker, location)
}

fn get_texture_handle(ui: &mut Ui, value: &mut RefMut<LoadedImage>) -> TextureHandle {
    if value.handle.is_none() {
        let img = value.image.to_rgba8();

        value.handle = Some(ui.ctx().load_texture(
            value.path.to_string_lossy(),
            ColorImage::from_rgba_unmultiplied(
                [value.image.width() as _, value.image.height() as _],
                img.as_flat_samples().as_slice()
            ),
            TextureOptions::LINEAR
        ))
    }

    value.handle.clone().unwrap()
}

pub fn inline_image_picker(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    value: &mut SharedLoadedImage,
    location: impl AsRef<Path>,
    width: f32,
    tracker: &mut ChangeTracker
) -> InnerResponse<()> {
    let location = location.as_ref();
    let mut value = value.borrow_mut();

    let content = value.path.to_string_lossy().to_string();

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            util::inline_style_label(ui, label, width);
            if ui.button("Pick Image").clicked() {
                pick_image_file(&mut value, tracker, location)
            }
            util::disabled_text_edit(ui, content, util::BUTTON_WIDTH);
        });

        ui.horizontal(|ui| {
            ui.add_space(width + ui.style().spacing.item_spacing.x);

            let img_size = vec2(IMAGE_SIZE, IMAGE_SIZE);

            let img = Image::new(&get_texture_handle(ui, &mut value))
                .fit_to_exact_size(img_size);
            ui.add_sized(img_size, img);
        });
        ui.horizontal(|ui| {
            ui.add_space(width + ui.style().spacing.item_spacing.x);
            ui.label(format!("({} x {})", value.image.width(), value.image.height()));
        });
    })
}

pub fn inline_image_resource_picker(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    value: &mut SharedString,
    images: &mut Vec<(SharedString, SharedLoadedImage)>,
    location: impl AsRef<Path>,
    width: f32,
    tracker: &mut ChangeTracker
) -> InnerResponse<()> {
    let location = location.as_ref();

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            let id = inline_style_label(ui, label, width).response.id;
            ComboBox::new(id.with("combo"), "")
                .selected_text(value.to_string())
                .show_ui(ui, |ui| {
                    for (k, _) in &mut *images {
                        if ui.selectable_label(value == k, k.rich()).clicked() {
                            *value = k.clone();
                            tracker.mark_change()
                        }
                    }
                });

            if ui.button("Pick New Image").clicked() {
                if let Some(filepath) = pick_image_filepath(location) {
                    if let Some(file_stem) = filepath.file_stem() {
                        let new_unique = pick_unique_name(
                            file_stem.to_string_lossy().to_string(),
                            &images
                        );

                        images.insert(0, (new_unique.clone(), SharedLoadedImage::default()));
                        let (_, image) = images.get_mut(0).unwrap();
                        try_set_loaded_image(filepath, &mut image.borrow_mut(), tracker, location);

                        *value = new_unique;
                    }
                }
            }
        });

        let index = ui.memory_mut(|mem| {
            let index = mem.data.get_persisted_mut_or_insert_with::<Option<usize>>(
                ui.id().with(&value),
                || {
                    Some(images.iter().enumerate().find(|(_, (k, _))| k == value)?.0)
                }
            );

            if let Some(index) = index {
                if let Some((k, _)) = images.get(*index) {
                    if k != value {
                        if let Some((new_index, _)) = images.iter().enumerate().find(|(_, (k, _))| k == value) {
                            *index = new_index;
                        } else {
                            return None;
                        }
                    }

                    return Some(*index)
                }
            }

            None
        });

        if let Some(index) = index {
            let (_, image) = images.get_mut(index).unwrap();

            let mut image = image.borrow_mut();

            ui.horizontal(|ui| {
                ui.add_space(width + ui.style().spacing.item_spacing.x);

                let img_size = vec2(IMAGE_SIZE, IMAGE_SIZE) / 2.0;

                let img = Image::new(&get_texture_handle(ui, &mut image))
                    .fit_to_exact_size(img_size);
                ui.add_sized(img_size, img);
            });

            ui.horizontal(|ui| {
                ui.add_space(width + ui.style().spacing.item_spacing.x);
                ui.label(format!("({} x {})", image.image.width(), image.image.height()));
            });
        }
    })
}

pub fn inline_validation_error(
    ui: &mut Ui,
    validations: &Vec<ValidationError>,
    error_message: impl Display,
    condition: impl Fn(&ValidationError) -> bool,
    width: f32
) {
    for error in validations {
        if condition(error) {
            ui.horizontal(|ui| {
                ui.add_space(width + ui.style().spacing.item_spacing.x);
                ui.label(error_message.rich().color(Color32::RED))
            });
            return;
        }
    }
}

impl GuiPage for CharacterEditor {
    fn show(&mut self, ui: &mut Ui) -> PageResponse {
        if ui.input(|k| k.modifiers.ctrl && k.key_pressed(Key::S)) {
            self.save();
        }

        if ui.input(|k| k.modifiers.ctrl && k.modifiers.shift && k.key_pressed(Key::S)) {
            self.save_as();
        }

        if ui.input(|k| k.modifiers.ctrl && k.key_pressed(Key::E)) {
            self.export();
        }

        let button_resp = TopBottomPanel::top("editor.top")
            .show(ui.ctx(), |ui| {
                ui.horizontal_centered(|ui| {
                    if let Some(resp) = MenuButton::new("File")
                        .ui(ui, |ui| {
                            ui.allocate_exact_size(vec2(100.0, 0.0), Sense::empty());

                            if ui.button("Save (Ctrl + S)".rich()).clicked() {
                                self.save()
                            }

                            if ui.button("Save As (Ctrl + Shift + S)").clicked() {
                                self.save_as()
                            }

                            if ui.button("Export (Ctrl + E)").clicked() {
                                self.export()
                            }

                            ui.separator();

                            if ui.button("Exit to Start").clicked() {
                                return Some(PageResponse::SwitchPage(StartScreen::new()))
                            }

                            None
                        }).1 {
                        if let Some(resp) = resp.inner {
                            return Some(resp)
                        }
                    }

                    ui.separator();

                    for variant in EditorTab::iter() {
                        if ui.add(
                            Button::new(variant.rich())
                                .selected(variant == self.tab)
                        ).clicked() {
                            self.tab = variant
                        }
                    }

                    ui.separator();

                    if self.tracker.unsaved() {
                        ui.label("Unsaved changes!");
                    }

                    ui.label(if let Some(last_save) = &self.last_save {
                        let diff = Instant::now() - *last_save;

                        let fmt = timeago::Formatter::new();

                        format!("Last saved: {}", fmt.convert(diff))
                    } else {
                        "Never saved".to_string()
                    }.rich().color(Color32::GRAY));

                    None
                }).inner
            }).inner;

        if let Some(resp) = button_resp {
            return resp
        }

        if let Some(error) = self.validation_errors.first() {
            TopBottomPanel::bottom("editor.bottom")
                .show(ui.ctx(), |ui| {
                    ui.label(error.rich().color(Color32::RED));
                });
        }

        CentralPanel::default()
            .show(ui.ctx(), |ui| {
                match self.tab {
                    EditorTab::Resources => {
                        self.resources_ui(ui);
                    }

                    EditorTab::StateMachine => {
                        self.state_machine_ui(ui);
                    }
                }
            });

        if self.tracker.changed() {
            self.validation_errors = self.validate_state();
            self.tracker.mark_clean();
        }

        PageResponse::Nothing
    }
}