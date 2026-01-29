use crate::character::repr::{Action, ActionType, Animation, SequenceFrame, SequenceMode, State, StateImage, StateTransition, StateTransitionTrigger};
use crate::gui::app::shared::{MutableStringScope, SharedString};
use crate::gui::app::util::load_image_or_black;
use egui::{pos2, Pos2, TextureHandle};
use image::DynamicImage;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use strum::{Display, EnumIs};

#[derive(Clone, Debug, Default)]
pub struct InterAction {
    pub display: String,
    pub ty: InterActionType
}

#[derive(Clone, Debug, Default, Display, EnumIs)]
pub enum InterActionType {
    #[default]
    None,
    SwitchState(SharedString)
}

impl InterAction {
    pub fn from_action(action: Action, states: &Vec<(SharedString, SharedInterState)>) -> Option<InterAction> {
        let ty = match action.ty {
            ActionType::SwitchState(name) => InterActionType::SwitchState(
                states.iter().find(|(k, _)| k.str_eq(&name))?.0.clone()
            )
        };

        Some(InterAction {
            display: action.display,
            ty
        })
    }

    pub fn into_action(self) -> Option<Action> {
        Some(Action {
            display: self.display,
            ty: match self.ty {
                InterActionType::None => return None,
                InterActionType::SwitchState(state) => ActionType::SwitchState(state.to_string())
            },
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct InterState {
    pub image: InterStateImage,
    pub transitions: Vec<SharedInterStateTransition>,
    pub node_pos: Pos2
}

pub type SharedInterState = Rc<RefCell<InterState>>;

impl InterState {
    pub fn from_state(
        state: State,
        names: &Vec<SharedString>,
        images: &Vec<(SharedString, SharedLoadedImage)>,
        animations: &Vec<(SharedString, Animation)>
    ) -> Option<InterState> {
        let image = match state.image {
            StateImage::None => InterStateImage::None,
            StateImage::Single {
                name,
                width,
                height,
                alpha,
                upscale,
                preload,
                ..
            } => InterStateImage::Single {
                image: images.iter().find(|(k, _)| k.str_eq(&name))?.0.clone(),
                width,
                height,
                alpha,
                upscale,
                preload,
            },
            StateImage::Animation {
                name,
                next_state,
                loop_count,
                preload
            } => InterStateImage::Animation {
                animation: animations.iter().find(|(k, _)| k.str_eq(&name))?.0.clone(),
                next_state: names.iter().find(|k| k.str_eq(&next_state))?.clone(),
                loop_count,
                preload,
            },
            StateImage::Sequence {
                frames,
                mode
            } => InterStateImage::Sequence {
                frames: frames.into_iter()
                    .filter_map(|frame| {
                        Some(InterSequenceFrame {
                            image: images.iter().find(|(k, _)| k.str_eq(&frame.name))?.0.clone(),
                            width: frame.width,
                            height: frame.height,
                            alpha: frame.alpha,
                            upscale: frame.upscale,
                            duration: frame.duration,
                        })
                    })
                    .collect::<Vec<_>>(),
                mode,
            }
        };

        let transitions = state.transitions.into_iter()
            .filter_map(|transition| {
                Some(Rc::new(RefCell::new(InterStateTransition {
                    to_state: names.iter().find(|k| k.str_eq(&transition.to_state))?.clone(),
                    trigger: transition.trigger,
                })))
            })
            .collect();

        let node_pos = if let Some((x, y)) = state.node_pos {
            pos2(x, y)
        } else {
            pos2(0.0, 0.0)
        };

        Some(InterState {
            image,
            transitions,
            node_pos: node_pos
        })
    }

    pub fn into_state(self, images: &Vec<(SharedString, SharedLoadedImage)>) -> Option<State> {
        Some(State {
            image: match self.image {
                InterStateImage::None => StateImage::None,
                InterStateImage::Single {
                    image, width, height, alpha, upscale, preload
                } => StateImage::Single {
                    name: image.to_string(),
                    path: images.iter().find(|(k, _)| k == &image)?.1.borrow().path.clone(),
                    width,
                    height,
                    alpha,
                    upscale,
                    preload,
                },
                InterStateImage::Animation {
                    animation, next_state, loop_count, preload
                } => StateImage::Animation {
                    name: animation.to_string(),
                    next_state: next_state.to_string(),
                    loop_count,
                    preload,
                },
                InterStateImage::Sequence {
                    frames, mode
                } => StateImage::Sequence {
                    frames: frames.into_iter()
                        .filter_map(|e| {
                            Some(SequenceFrame {
                                name: e.image.to_string(),
                                path: images.iter().find(|(k, _)| k == &e.image)?.1.borrow().path.clone(),
                                width: e.width,
                                height: e.height,
                                alpha: e.alpha,
                                upscale: e.upscale,
                                duration: e.duration,
                            })
                        })
                        .collect(),
                    mode,
                }
            },
            transitions: self.transitions.into_iter()
                .map(|t| t.borrow().clone().into())
                .collect(),
            node_pos: Some(self.node_pos.into()),
        })
    }
}

#[derive(Clone, Debug, Default, Display, EnumIs)]
pub enum InterStateImage {
    #[default]
    None,
    Single {
        image: SharedString,
        width: u32,
        height: u32,
        alpha: bool,
        upscale: bool,
        preload: bool
    },
    Animation {
        animation: SharedString,
        next_state: SharedString,
        loop_count: u16,
        preload: bool
    },
    Sequence {
        frames: Vec<InterSequenceFrame>,
        mode: SequenceMode
    }
}

#[derive(Clone, Debug)]
pub struct InterSequenceFrame {
    pub image: SharedString,
    pub width: u32,
    pub height: u32,
    pub alpha: bool,
    pub upscale: bool,
    pub duration: i64
}

impl Default for InterSequenceFrame {
    fn default() -> Self {
        Self {
            image: SharedString::from("None"),
            width: 320,
            height: 320,
            alpha: false,
            upscale: false,
            duration: 1_000_000,
        }
    }
}

#[derive(Clone, Debug)]
pub struct InterStateTransition {
    pub to_state: SharedString,
    pub trigger: StateTransitionTrigger
}

pub type SharedInterStateTransition = Rc<RefCell<InterStateTransition>>;

impl From<InterStateTransition> for StateTransition {
    fn from(value: InterStateTransition) -> Self {
        Self {
            to_state: value.to_state.to_string(),
            trigger: value.trigger,
        }
    }
}

#[derive(Clone, Default)]
pub struct LoadedImage {
    pub path: PathBuf,
    pub image: DynamicImage,
    pub handle: Option<TextureHandle>
}

pub type SharedLoadedImage = Rc<RefCell<LoadedImage>>;

pub fn find_images(map: &HashMap<String, State>, location: impl AsRef<Path>) -> Vec<(SharedString, SharedLoadedImage)> {
    let mut found_images: Vec<(SharedString, PathBuf)> = vec![];

    let mut add_unique = |name: &String, path: &PathBuf| {
        if !found_images.iter().any(|(k, _)| k.refer(|k| k == name)) {
            found_images.push((name.to_string().into(), path.clone()))
        }
    };

    for state in map.values() {
        match &state.image {
            StateImage::Single { name, path, .. } => {
                add_unique(name, path);
            }
            StateImage::Sequence { frames, .. } => {
                for frame in frames {
                    add_unique(&frame.name, &frame.path)
                }
            }
            _ => {}
        }
    }

    let base_location = location.as_ref().to_path_buf();

    found_images.into_iter()
        .map(|(k, v)| (k, Rc::new(RefCell::new(LoadedImage {
            image: load_image_or_black(base_location.join(&v)),
            path: v,
            handle: None
        }))))
        .collect()
}
