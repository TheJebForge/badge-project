mod intermediate;
mod resources;
mod nodes;

use crate::character::repr::{Animation, Character, State};
use crate::character::util::AsRichText;
use crate::gui::app::editor::intermediate::{find_images, InterAction, InterState, LoadedImage, SharedInterState};
use crate::gui::app::shared::SharedString;
use crate::gui::app::start::StartScreen;
use crate::gui::app::util::SPACING;
use crate::gui::app::{util, BoxedGuiPage, GuiPage, PageResponse};
use egui::{vec2, Button, CentralPanel, ColorImage, Image, InnerResponse, ScrollArea, SidePanel, TextureOptions, TopBottomPanel, Ui, WidgetText};
use std::path::{Path, PathBuf};
use std::{env, fs};
use std::cell::RefCell;
use std::rc::Rc;
use egui_snarl::Snarl;
use egui_snarl::ui::SnarlStyle;
use strum::{Display, EnumIter, IntoEnumIterator};
use crate::gui::app::editor::nodes::{snarl_from_states, snarl_style, StateViewer};

pub struct CharacterEditor {
    tab: EditorTab,
    location: PathBuf,
    id: String,
    name: String,
    species: String,
    default_state: SharedString,
    images: Vec<(SharedString, LoadedImage)>,
    animations: Vec<(SharedString, Animation)>,
    actions: Vec<(String, InterAction)>,
    states: Vec<(SharedString, SharedInterState)>,
    state_graph: Snarl<(SharedString, SharedInterState)>,
    graph_style: SnarlStyle,
    graph_viewer: StateViewer,
}

#[derive(Copy, Clone, EnumIter, Default, Display, Eq, PartialEq)]
enum EditorTab {
    #[default]
    Resources,
    StateMachine
}

impl CharacterEditor {
    pub fn from_character(mut char: Character, location: PathBuf) -> CharacterEditor {
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

        Self {
            tab: EditorTab::default(),
            location,
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
            graph_viewer: StateViewer::default()
        }
    }

    pub fn new_file(id: &str) -> BoxedGuiPage {
        Box::new(Self::from_character(Character::from_id(id), env::current_dir().unwrap()))
    }

    pub fn open_file(path: impl AsRef<Path>) -> anyhow::Result<BoxedGuiPage> {
        let path = path.as_ref().to_path_buf();

        let contents = fs::read_to_string(&path)?;
        let character = serde_json::from_str::<Character>(&contents)?;

        let location = if let Some(parent) = path.parent() {
            parent.to_path_buf()
        } else {
            path
        };

        Ok(Box::new(Self::from_character(character, location)))
    }
}

pub const IMAGE_EXTENSIONS: &[&'static str] = &["png", "jpg", "jpeg", "bmp", "tga", "tiff"];

const IMAGE_SIZE: f32 = 200.0;

pub fn inline_image_picker(ui: &mut Ui, label: impl Into<WidgetText>, value: &mut LoadedImage, location: impl AsRef<Path>, width: f32) -> InnerResponse<()> {
    let location = location.as_ref().to_path_buf();

    let content = value.path.to_string_lossy().to_string();

    let try_pick_file = |value: &mut LoadedImage| {
        let Some(picked_file) = rfd::FileDialog::new()
            .set_title("Pick image file")
            .add_filter("Image", IMAGE_EXTENSIONS)
            .set_directory(&location)
            .pick_file()
        else {
            return;
        };

        let stripped = match picked_file.strip_prefix(&location) {
            Ok(relative) => relative.to_path_buf(),
            Err(_) => picked_file
        };

        let image = match util::load_image(location.join(&stripped)) {
            Ok(image) => image,
            Err(err) => {
                eprintln!("Failed to load image! {err}");
                return;
            }
        };

        value.image = image;
        value.path = stripped;
        value.handle = None;
    };

    let get_handle = |ui: &mut Ui, value: &mut LoadedImage| {
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
    };

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            util::inline_style_label(ui, label, width);
            if ui.button("Pick Image").clicked() {
                try_pick_file(value)
            }
            util::disabled_text_edit(ui, content, util::BUTTON_WIDTH);
        });

        ui.horizontal(|ui| {
            ui.add_space(width);

            let img_size = vec2(IMAGE_SIZE, IMAGE_SIZE);

            let img = Image::new(&get_handle(ui, value))
                .fit_to_exact_size(img_size);
            ui.add_sized(img_size, img);
        });
        ui.horizontal(|ui| {
            ui.add_space(width);
            ui.label(format!("({} x {})", value.image.width(), value.image.height()));
        });
    })
}

impl GuiPage for CharacterEditor {
    fn show(&mut self, ui: &mut Ui) -> PageResponse {
        let button_resp = TopBottomPanel::top("editor.top")
            .show(ui.ctx(), |ui| {
                ui.horizontal_centered(|ui| {
                    if ui.button("<-".rich().size(15.0)).clicked() {
                        return Some(PageResponse::SwitchPage(StartScreen::new()))
                    }

                    ui.add_space(SPACING * 2.0);

                    for variant in EditorTab::iter() {
                        if ui.add(
                            Button::new(variant.rich().size(15.0))
                                .selected(variant == self.tab)
                        ).clicked() {
                            self.tab = variant
                        }
                    }

                    None
                }).inner
            }).inner;

        if let Some(resp) = button_resp {
            return resp
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

        PageResponse::Nothing
    }
}