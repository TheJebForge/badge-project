use crate::character::repr::{Action, ActionType, Animation, SequenceFrame, SequenceMode, State, StateImage, StateTransition, StateTransitionTrigger};
use crate::gui::app::shared::{MutableStringScope, SharedString};
use crate::gui::app::util::{load_image_or_black, pick_unique_name};
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
    pub layer: u8,
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
        sequences: &mut Vec<(SharedString, InterSequence)>,
        animations: &Vec<(SharedString, Animation)>
    ) -> Option<InterState> {
        let image = match state.image {
            StateImage::None => InterStateImage::None,
            StateImage::Single {
                name,
                layer_load,
                ..
            } => InterStateImage::Single {
                image: images.iter().find(|(k, _)| k.str_eq(&name))?.0.clone(),
                layer_load,
            },
            StateImage::Animation {
                name,
                next_state,
                loop_count,
                layer_load
            } => InterStateImage::Animation {
                animation: animations.iter().find(|(k, _)| k.str_eq(&name))?.0.clone(),
                next_state: names.iter().find(|k| k.str_eq(&next_state))?.clone(),
                loop_count,
                layer_load,
            },
            StateImage::Sequence {
                name,
                frames,
                mode,
                layer_load
            } => {
                let from_frames = |frames: Vec<SequenceFrame>| {
                    InterSequence {
                        frames: frames.into_iter()
                            .filter_map(|frame| {
                                Some(InterSequenceFrame {
                                    image: images.iter().find(|(k, _)| k.str_eq(&frame.name))?.0.clone(),
                                    duration: frame.duration,
                                })
                            })
                            .collect::<Vec<_>>()
                    }
                };
                
                let sequence = if let Some(name) = name {
                    if let Some((existing, _)) = sequences.iter()
                        .find(|(k, _)| k.str_eq(&name)) {
                        existing.clone()
                    } else {
                        let new_name = SharedString::from(name);
                        sequences.push((new_name.clone(), from_frames(frames)));
                        new_name
                    }
                } else {
                    let new_name = pick_unique_name("unknown".to_string(), sequences);
                    sequences.push((new_name.clone(), from_frames(frames)));
                    new_name
                };
                
                InterStateImage::Sequence {
                    sequence,
                    mode,
                    layer_load
                }
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
            layer: state.layer,
            image,
            transitions,
            node_pos
        })
    }

    pub fn into_state(
        self,
        images: &Vec<(SharedString, SharedLoadedImage)>,
        sequences: &Vec<(SharedString, InterSequence)>
    ) -> Option<State> {
        Some(State {
            layer: self.layer,
            image: match self.image {
                InterStateImage::None => StateImage::None,
                InterStateImage::Single {
                    image, layer_load
                } => {
                    let image_data = images.iter().find(|(k, _)| k == &image)?.1.borrow().clone();

                    StateImage::Single {
                        name: image.to_string(),
                        path: image_data.path,
                        width: image_data.width,
                        height: image_data.height,
                        upscale: image_data.upscale,
                        layer_load,
                    }
                },
                InterStateImage::Animation {
                    animation, next_state, loop_count, layer_load
                } => StateImage::Animation {
                    name: animation.to_string(),
                    next_state: next_state.to_string(),
                    loop_count,
                    layer_load,
                },
                InterStateImage::Sequence {
                    sequence, mode, layer_load
                } => {
                    let sequence_data = sequences.iter()
                        .find(|(k, _)| k == &sequence)?.1.clone();

                    StateImage::Sequence {
                        name: Some(sequence.to_string()),
                        frames: sequence_data.frames.into_iter()
                            .filter_map(|e| {
                                let image = images.iter().find(|(k, _)| k == &e.image)?.1.borrow().clone();

                                Some(SequenceFrame {
                                    name: e.image.to_string(),
                                    path: image.path,
                                    width: image.width,
                                    height: image.height,
                                    upscale: image.upscale,
                                    duration: e.duration,
                                })
                            })
                            .collect(),
                        mode,
                        layer_load
                    }
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
        layer_load: bool
    },
    Animation {
        animation: SharedString,
        next_state: SharedString,
        loop_count: u16,
        layer_load: bool
    },
    Sequence {
        sequence: SharedString,
        mode: SequenceMode,
        layer_load: bool
    }
}


#[derive(Clone, Debug)]
#[derive(Default)]
pub struct InterSequence {
    pub frames: Vec<InterSequenceFrame>
}

#[derive(Clone, Debug)]
pub struct InterSequenceFrame {
    pub image: SharedString,
    pub duration: i64
}

impl Default for InterSequenceFrame {
    fn default() -> Self {
        Self {
            image: SharedString::from("None"),
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

#[derive(Clone)]
pub struct LoadedImage {
    pub path: PathBuf,
    pub image: DynamicImage,
    pub width: u32,
    pub height: u32,
    pub upscale: bool,
    pub handle: Option<TextureHandle>
}

impl Default for LoadedImage {
    fn default() -> Self {
        Self {
            path: Default::default(),
            image: Default::default(),
            width: 320,
            height: 320,
            upscale: false,
            handle: None,
        }
    }
}

pub type SharedLoadedImage = Rc<RefCell<LoadedImage>>;

pub fn find_images(map: &HashMap<String, State>, location: impl AsRef<Path>) -> Vec<(SharedString, SharedLoadedImage)> {
    let mut found_images: Vec<(SharedString, (PathBuf, (u32, u32), bool))> = vec![];

    let mut add_unique = |
        name: &String,
        path: &PathBuf,
        width: u32,
        height: u32,
        upscale: bool
    | {
        if !found_images.iter().any(|(k, _)| k.refer(|k| k == name)) {
            found_images.push((
                name.to_string().into(),
                (path.clone(), (width, height), upscale)
            ))
        }
    };

    for state in map.values() {
        match &state.image {
            StateImage::Single {
                name, path, width, height, upscale, ..
            } => {
                add_unique(
                    name,
                    path,
                    *width,
                    *height,
                    *upscale
                );
            }
            StateImage::Sequence { frames, .. } => {
                for frame in frames {
                    add_unique(
                        &frame.name,
                        &frame.path,
                        frame.width,
                        frame.height,
                        frame.upscale
                    )
                }
            }
            _ => {}
        }
    }

    let base_location = location.as_ref().to_path_buf();

    found_images.into_iter()
        .map(|(k, (path, (width, height), upscale))|
            (k, Rc::new(RefCell::new(LoadedImage {
                image: load_image_or_black(base_location.join(&path)),
                width,
                height,
                path,
                handle: None,
                upscale,
            }))))
        .collect()
}