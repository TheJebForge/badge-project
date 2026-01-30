use std::collections::HashSet;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use strum::Display;
use crate::gui::app::editor::CharacterEditor;
use crate::gui::app::editor::intermediate::{InterActionType, InterStateImage};

#[derive(Clone, Debug, Display)]
#[derive(PartialEq, Eq)]
pub enum ValidationError {
    #[strum(to_string = "Duplicate state '{0}'!")]
    DuplicateState(String),
    #[strum(to_string = "Duplicate animation '{0}'!")]
    DuplicateAnimation(String),
    #[strum(to_string = "Duplicate action '{0}'!")]
    DuplicateAction(String),
    #[strum(to_string = "Duplicate image '{0}'!")]
    DuplicateImage(String),
    #[strum(to_string = "Selected animation in state '{0}' doesn't exist!")]
    InvalidAnimationInState(String),
    #[strum(to_string = "Selected next state in animation of state '{0}' doesn't exist!")]
    InvalidNextStateInAnimation(String),
    #[strum(to_string = "Selected image in state '{0}' doesn't exist!")]
    InvalidImageInState(String),
    #[strum(to_string = "Selected image in sequence frame #{1} of state '{0}' doesn't exist!")]
    InvalidImageInSequenceFrame(String, usize),
    #[strum(to_string = "Selected action type in action '{0}' is invalid!")]
    InvalidActionType(String),
    #[strum(to_string = "Selected default state doesn't exist!")]
    InvalidDefaultState,
    #[strum(to_string = "Selected state in action '{0}' doesn't exist!")]
    InvalidActionState(String),
    #[strum(to_string = "State name can't be empty!")]
    EmptyStateName,
    #[strum(to_string = "Animation name can't be empty!")]
    EmptyAnimationName,
    #[strum(to_string = "Action name can't be empty!")]
    EmptyActionName,
    #[strum(to_string = "Image name can't be empty!")]
    EmptyImageName
}

impl CharacterEditor {
    pub fn validate_state(&self) -> Vec<ValidationError> {
        let mut errors = vec![];

        // Check for duplicates
        errors.extend(
            find_duplicates(&self.states)
            .into_iter()
            .map(|e| ValidationError::DuplicateState(e.to_string()))
        );

        errors.extend(
            find_duplicates(&self.animations)
                .into_iter()
                .map(|e| ValidationError::DuplicateAnimation(e.to_string()))
        );

        errors.extend(
            find_duplicates(&self.actions)
                .into_iter()
                .map(|e| ValidationError::DuplicateAction(e.to_string()))
        );

        errors.extend(
            find_duplicates(&self.images)
                .into_iter()
                .map(|e| ValidationError::DuplicateImage(e.to_string()))
        );

        // Check for empty names
        if check_for_empty(&self.states) {
            errors.push(ValidationError::EmptyStateName)
        }
        if check_for_empty(&self.animations) {
            errors.push(ValidationError::EmptyAnimationName)
        }
        if check_for_empty(&self.actions) {
            errors.push(ValidationError::EmptyActionName)
        }
        if check_for_empty(&self.images) {
            errors.push(ValidationError::EmptyImageName)
        }

        // Check for unassigned stuff
        let image_names = self.images.iter()
            .map(|(k, _)| k.clone())
            .collect::<HashSet<_>>();

        let animation_names = self.animations.iter()
            .map(|(k, _)| k.clone())
            .collect::<HashSet<_>>();

        let state_names = self.states.iter()
            .map(|(k, _)| k.clone())
            .collect::<HashSet<_>>();

        for (state_name, v) in &self.states {
            let b_state = v.borrow();

            match &b_state.image {
                InterStateImage::Animation { animation, next_state, .. } => {
                    if !animation_names.contains(animation) {
                        errors.push(ValidationError::InvalidAnimationInState(state_name.to_string()));
                    }

                    if !state_names.contains(next_state) {
                        errors.push(ValidationError::InvalidNextStateInAnimation(state_name.to_string()));
                    }
                }

                InterStateImage::Single { image, .. } => {
                    if !image_names.contains(image) {
                    errors.push(ValidationError::InvalidImageInState(state_name.to_string()));
                    }
                }

                InterStateImage::Sequence { frames, .. } => {
                    for (index, frame) in frames.iter().enumerate() {
                    if !image_names.contains(&frame.image) {
                    errors.push(ValidationError::InvalidImageInSequenceFrame(state_name.to_string(), index))
                    }
                    }
                }
                _ => {}
            }
        }

        for (action_name, action) in &self.actions {
            if let InterActionType::None = &action.ty {
                errors.push(ValidationError::InvalidActionType(action_name.to_string()))
            }

            if let InterActionType::SwitchState(state) = &action.ty {
                if !state_names.contains(state) {
                    errors.push(ValidationError::InvalidActionState(action_name.to_string()))
                }
            }
        }

        if !state_names.contains(&self.default_state) {
            errors.push(ValidationError::InvalidDefaultState)
        }

        errors
    }
}

fn find_duplicates<K, V>(vec: &Vec<(K, V)>) -> Vec<K>
where
    K: Hash + Clone + Eq + Display
{
    let mut found_duplicates = vec![];

    let mut set = HashSet::new();

    for (k, _) in vec {
        let k_str = k.to_string();

        if set.contains(&k_str) {
            found_duplicates.push(k.clone());
        } else {
            set.insert(k_str);
        }
    }

    found_duplicates
}

fn check_for_empty<K, V>(vec: &Vec<(K, V)>) -> bool
where
    K: Hash + Clone + Eq + Display
{
    for (k, _) in vec {
        let k_str = k.to_string();

        if k_str.is_empty() {
            return true;
        }
    }

    false
}