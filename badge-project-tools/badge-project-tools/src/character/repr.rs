use std::collections::HashMap;
use crate::character::util::{any_as_u8_vec, string_to_char_array, TuplePick};
use crate::{bp_character_action_e_BP_CHARACTER_ACTION_SWITCH_STATE, bp_character_action_file_s, bp_character_action_u, bp_character_animation_file_s, bp_character_animation_mode_e_BP_CHARACTER_ANIMATION_MODE_FROM_RAM, bp_character_animation_mode_e_BP_CHARACTER_ANIMATION_MODE_FROM_SDCARD, bp_character_file_s, bp_character_image_descriptor_s, bp_character_sequence_mode_e_BP_CHARACTER_SEQUENCE_MODE_LOAD_ALL, bp_character_sequence_mode_e_BP_CHARACTER_SEQUENCE_MODE_LOAD_EACH, bp_character_sequence_mode_e_BP_CHARACTER_SEQUENCE_MODE_PRELOAD, bp_character_state_animation_descriptor_s, bp_character_state_file_s, bp_character_state_image_e_BP_CHARACTER_STATE_ANIMATION, bp_character_state_image_e_BP_CHARACTER_STATE_NO_IMAGE, bp_character_state_image_e_BP_CHARACTER_STATE_SEQUENCE, bp_character_state_image_e_BP_CHARACTER_STATE_SINGLE_IMAGE, bp_character_state_image_u, bp_character_state_sequence_descriptor_s, bp_data_FORMAT_VERSION, bp_sequence_frame_file_s, bp_state_transition_file_s, bp_state_trigger_e_BP_STATE_TRIGGER_CLICKED, bp_state_trigger_e_BP_STATE_TRIGGER_ELAPSED_TIME, bp_state_trigger_e_BP_STATE_TRIGGER_RANDOM, bp_state_trigger_random_s, bp_state_trigger_s, bp_state_trigger_u};
use serde::{Deserialize, Serialize};
use std::ffi::NulError;
use std::path::PathBuf;
use either::Either;
use strum::{Display, EnumIs, EnumIter};
use crate::image::rgb_to_565;

pub trait BinaryRepr {
    fn to_bin(&self) -> Result<Vec<u8>, NulError>;
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub species: String,
    pub default_state: String,
    #[serde(default)]
    pub states: HashMap<String, State>,
    #[serde(default)]
    pub animations: HashMap<String, Animation>,
    #[serde(default)]
    pub actions: HashMap<String, Action>
}

impl Default for Character {
    fn default() -> Self {
        Self::from_id("unnamed")
    }
}

impl Character {
    pub fn from_id(name: &str) -> Self {
        Self {
            id: name.to_string(),
            name: "Unnamed".to_string(),
            species: "Unknown Species".to_string(),
            default_state: "idle".to_string(),
            states: HashMap::from([
                ("idle".to_string(), State {
                    image: StateImage::None,
                    transitions: vec![],
                    node_pos: None,
                })
            ]),
            animations: Default::default(),
            actions: Default::default(),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct State {
    pub image: StateImage,
    #[serde(default)]
    pub transitions: Vec<StateTransition>,
    #[serde(default)]
    pub node_pos: Option<(f32, f32)>
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub enum StateImage {
    #[default]
    None,
    Single {
        name: String,
        path: PathBuf,
        width: u32,
        height: u32,
        #[serde(default)]
        alpha: bool,
        #[serde(default)]
        upscale: bool,
        #[serde(default)]
        preload: bool
    },
    Animation {
        name: String,
        next_state: String,
        loop_count: u16,
        #[serde(default)]
        preload: bool
    },
    Sequence {
        #[serde(default)]
        name: Option<String>,
        frames: Vec<SequenceFrame>,
        mode: SequenceMode
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct StateTransition {
    pub to_state: String,
    pub trigger: StateTransitionTrigger
}

#[derive(Deserialize, Serialize, Clone, Debug, Display, EnumIs)]
pub enum StateTransitionTrigger {
    ElapsedTime {
        duration: i64
    },
    Clicked,
    Random {
        #[serde(with = "either::serde_untagged")]
        duration_range: Either<(i64, i64), i64>,
        chance: u32
    }
}

impl Default for StateTransitionTrigger {
    fn default() -> Self {
        Self::ElapsedTime {
            duration: 1000000
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, EnumIs)]
pub enum AnimationFrameSource {
    Indexed {
        folder: PathBuf,
        extension: String,
        count: u32
    },
    List(Vec<PathBuf>)
}

impl Default for AnimationFrameSource {
    fn default() -> Self {
        Self::List(vec![])
    }
}

impl AnimationFrameSource {
    pub fn count(&self) -> u32 {
        match self {
            &AnimationFrameSource::Indexed { count, .. } => count,
            AnimationFrameSource::List(list) => list.len() as u32
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Animation {
    pub x: u16,
    pub y: u16,
    pub width: u32,
    pub height: u32,
    pub frames: AnimationFrameSource,
    pub fps: f64,
    #[serde(default)]
    pub clear_screen: bool,
    #[serde(default)]
    pub background_color: (u8, u8, u8),
    #[serde(default)]
    pub mode: AnimationMode,
    #[serde(default)]
    pub upscale: bool
}

impl Default for Animation {
    fn default() -> Self {
        Self {
            x: 0,
            y: 160,
            width: 320,
            height: 320,
            frames: Default::default(),
            fps: 10.0,
            clear_screen: false,
            background_color: (0, 0, 0),
            mode: Default::default(),
            upscale: false,
        }
    }
}

#[derive(Deserialize, Serialize, Copy, Clone, Debug, Default, EnumIter, Display, PartialEq, Eq, EnumIs)]
pub enum AnimationMode {
    #[default]
    FromSDCard,
    FromRAM
}

#[derive(Deserialize, Serialize, Copy, Clone, Debug, Default, EnumIter, Display, PartialEq, Eq, EnumIs)]
pub enum SequenceMode {
    LoadAll,
    #[default]
    LoadEach,
    Preload
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct SequenceFrame {
    pub name: String,
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub alpha: bool,
    #[serde(default)]
    pub upscale: bool,
    pub duration: i64
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct Action {
    pub display: String,
    pub ty: ActionType
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum ActionType {
    SwitchState(String)
}

impl Default for ActionType {
    fn default() -> Self {
        Self::SwitchState("idle".to_string())
    }
}

impl BinaryRepr for Character {
    fn to_bin(&self) -> Result<Vec<u8>, NulError> {
        let file = bp_character_file_s {
            format_version: bp_data_FORMAT_VERSION,
            name: string_to_char_array(&self.name)?,
            species: string_to_char_array(&self.species)?,
            default_state: string_to_char_array(&self.default_state)?,
        };

        Ok(unsafe { any_as_u8_vec(&file) })
    }
}

impl BinaryRepr for SequenceFrame {
    fn to_bin(&self) -> Result<Vec<u8>, NulError> {
        let file = bp_sequence_frame_file_s {
            image_name: string_to_char_array(&self.name)?,
            width: self.width,
            height: self.height,
            has_alpha: self.alpha,
            upscale: self.upscale,
            duration_us: self.duration,
        };

        Ok(unsafe { any_as_u8_vec(&file) })
    }
}

impl Animation {
    pub fn real_width(&self) -> u32 {
        if self.upscale {
            self.width / 2
        } else {
            self.width
        }
    }

    pub fn real_height(&self) -> u32 {
        if self.upscale {
            self.height / 2
        } else {
            self.height
        }
    }
}

impl BinaryRepr for Animation {
    fn to_bin(&self) -> Result<Vec<u8>, NulError> {
        let bg = self.background_color;

        let file = bp_character_animation_file_s {
            x: self.x,
            y: self.y,
            width: self.real_width(),
            height: self.real_height(),
            frame_count: self.frames.count(),
            interval_us: (1_000_000_f64 / self.fps).floor() as i64,
            clear_screen: self.clear_screen,
            background_color: rgb_to_565(bg.0, bg.1, bg.2).to_be(),
            mode: match self.mode {
                AnimationMode::FromSDCard => bp_character_animation_mode_e_BP_CHARACTER_ANIMATION_MODE_FROM_SDCARD,
                AnimationMode::FromRAM => bp_character_animation_mode_e_BP_CHARACTER_ANIMATION_MODE_FROM_RAM,
            },
            upscale: self.upscale,
        };

        Ok(unsafe { any_as_u8_vec(&file) })
    }
}

impl BinaryRepr for StateTransition {
    fn to_bin(&self) -> Result<Vec<u8>, NulError> {
        let file = match &self.trigger {
            StateTransitionTrigger::ElapsedTime { duration } => {
                bp_state_transition_file_s {
                    trigger: bp_state_trigger_s {
                        type_: bp_state_trigger_e_BP_STATE_TRIGGER_ELAPSED_TIME,
                        data: bp_state_trigger_u {
                            state_duration_us: *duration
                        },
                    },
                }
            }
            StateTransitionTrigger::Clicked => {
                bp_state_transition_file_s {
                    trigger: bp_state_trigger_s {
                        type_: bp_state_trigger_e_BP_STATE_TRIGGER_CLICKED,
                        data: bp_state_trigger_u {
                            no_data: 0
                        },
                    },
                }
            },
            StateTransitionTrigger::Random { duration_range, chance } => {
                bp_state_transition_file_s {
                    trigger: bp_state_trigger_s {
                        type_: bp_state_trigger_e_BP_STATE_TRIGGER_RANDOM,
                        data: bp_state_trigger_u {
                            random_s: duration_range.either(
                                |tuple| bp_state_trigger_random_s {
                                    duration_start_range: tuple.pick_min(),
                                    duration_end_range: tuple.pick_max(),
                                    chance_mod: *chance,
                                },
                                |num| bp_state_trigger_random_s {
                                    duration_start_range: num,
                                    duration_end_range: num,
                                    chance_mod: *chance
                                }
                            )
                        }
                    }
                }
            }
        };

        Ok(unsafe { any_as_u8_vec(&file) })
    }
}

impl BinaryRepr for State {
    fn to_bin(&self) -> Result<Vec<u8>, NulError> {
        let file = match &self.image {
            StateImage::None => {
                bp_character_state_file_s {
                    image_type: bp_character_state_image_e_BP_CHARACTER_STATE_NO_IMAGE,
                    image: bp_character_state_image_u {
                        no_data: 0
                    },
                }
            },
            StateImage::Single { name, width, height, alpha, upscale, preload, .. } => {
                bp_character_state_file_s {
                    image_type: bp_character_state_image_e_BP_CHARACTER_STATE_SINGLE_IMAGE,
                    image: bp_character_state_image_u {
                        image: bp_character_image_descriptor_s {
                            image_name: string_to_char_array(name)?,
                            width: if *upscale { *width / 2 } else { *width },
                            height: if *upscale { *height / 2 } else { *height },
                            has_alpha: *alpha,
                            upscale: *upscale,
                            preload: *preload
                        }
                    },
                }
            }
            StateImage::Animation { name, next_state, loop_count, preload } => {
                bp_character_state_file_s {
                    image_type: bp_character_state_image_e_BP_CHARACTER_STATE_ANIMATION,
                    image: bp_character_state_image_u {
                        animation: bp_character_state_animation_descriptor_s {
                            name: string_to_char_array(name)?,
                            next_state: string_to_char_array(next_state)?,
                            loop_count: *loop_count,
                            preload: *preload
                        }
                    }
                }
            },
            StateImage::Sequence { frames, mode, .. } => {
                bp_character_state_file_s {
                    image_type: bp_character_state_image_e_BP_CHARACTER_STATE_SEQUENCE,
                    image: bp_character_state_image_u {
                        sequence: bp_character_state_sequence_descriptor_s {
                            frame_count: frames.len() as u16,
                            mode: match mode {
                                SequenceMode::LoadAll => bp_character_sequence_mode_e_BP_CHARACTER_SEQUENCE_MODE_LOAD_ALL,
                                SequenceMode::LoadEach => bp_character_sequence_mode_e_BP_CHARACTER_SEQUENCE_MODE_LOAD_EACH,
                                SequenceMode::Preload => bp_character_sequence_mode_e_BP_CHARACTER_SEQUENCE_MODE_PRELOAD
                            }
                        }
                    }
                }
            }
        };

        Ok(unsafe { any_as_u8_vec(&file) })
    }
}

impl BinaryRepr for Action {
    fn to_bin(&self) -> Result<Vec<u8>, NulError> {
        let file = match &self.ty {
            ActionType::SwitchState(state) => {
                bp_character_action_file_s {
                    display: string_to_char_array(&self.display)?,
                    type_: bp_character_action_e_BP_CHARACTER_ACTION_SWITCH_STATE,
                    data: bp_character_action_u {
                        state_name: string_to_char_array(state)?
                    },
                }
            }
        };

        Ok(unsafe { any_as_u8_vec(&file) })
    }
}