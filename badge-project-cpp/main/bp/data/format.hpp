#pragma once
#include <cstdint>

namespace bp::data {
    constexpr uint16_t FORMAT_VERSION = 1;
    constexpr std::size_t NAME_MAX_LEN = 64;
    constexpr std::size_t SPECIES_MAX_LEN = 64;
    constexpr std::size_t IMAGE_NAME_MAX_LEN = 64;
    constexpr std::size_t STATE_NAME_MAX_LEN = 64;
    constexpr std::size_t ANIMATION_NAME_MAX_LEN = 64;
    constexpr std::size_t ACTION_DISPLAY_MAX_LEN = 64;
}

extern "C" {
/// (character.bin) Main definition of the character
struct bp_character_file_s {
    uint16_t format_version;
    char name[bp::data::NAME_MAX_LEN];
    char species[bp::data::SPECIES_MAX_LEN];
    char default_state[bp::data::STATE_NAME_MAX_LEN];
};

/// Enumeration of different possible state transition triggers
enum bp_state_trigger_e {
    BP_STATE_TRIGGER_ELAPSED_TIME,
    BP_STATE_TRIGGER_CLICKED
};

/// Attached data for state transition triggers
union bp_state_trigger_u {
    char no_data;
    /// Time that the state has to be active for until it will trigger a state transition
    int64_t state_duration_us;
};

/// Describes what has to happen for state transition to be triggered
struct bp_state_trigger_s {
    bp_state_trigger_e type;
    bp_state_trigger_u data;
};

/// (transition.bin) Definition of character's state transition, describes what should happen for transition to trigger,
/// and what state to transition into
struct bp_state_transition_file_s {
    bp_state_trigger_s trigger;
};

/// Describes a character image
struct bp_character_image_descriptor_s {
    char image_name[bp::data::IMAGE_NAME_MAX_LEN];
    uint32_t width;
    uint32_t height;
    bool has_alpha;
    bool upscale;
    bool preload;
};

/// Describes how animation will be loaded and played
enum bp_character_animation_mode_e {
    BP_CHARACTER_ANIMATION_MODE_FROM_SDCARD,
    BP_CHARACTER_ANIMATION_MODE_FROM_RAM
};

/// (animation.bin) Definition of an animation file
struct bp_character_animation_file_s {
    /// Top left corner X
    uint16_t x;
    /// Top left corner Y
    uint16_t y;
    uint32_t width;
    uint32_t height;
    uint32_t frame_count;
    int64_t interval_us;
    bool clear_screen;
    /// Must be in big endian RGB565
    uint16_t background_color;
    bp_character_animation_mode_e mode;
    /// If 2x upscale is required
    bool upscale;
};

/// Describes data when state is an animation
struct bp_character_state_animation_descriptor_s {
    char name[bp::data::ANIMATION_NAME_MAX_LEN];
    char next_state[bp::data::STATE_NAME_MAX_LEN];
    uint16_t loop_count;
    bool preload;
};

/// (frames/<index>.bin) File representing a frame in a sequence of images
struct bp_sequence_frame_file_s {
    char image[bp::data::IMAGE_NAME_MAX_LEN];
    int64_t duration_us;
};

/// Loading mode for sequences, either load all frames at once when state switches,
/// load new frame each time, or preload all frames
enum bp_character_sequence_mode_e {
    BP_CHARACTER_SEQUENCE_MODE_LOAD_ALL,
    BP_CHARACTER_SEQUENCE_MODE_LOAD_EACH,
    BP_CHARACTER_SEQUENCE_MODE_PRELOAD
};

/// Describes data when state is a sequence
struct bp_character_state_sequence_descriptor_s {
    uint16_t frame_count;
    bp_character_sequence_mode_e mode;
};

/// Enum for deciding what the state should currently show
enum bp_character_state_image_e {
    BP_CHARACTER_STATE_NO_IMAGE,
    BP_CHARACTER_STATE_SINGLE_IMAGE,
    BP_CHARACTER_STATE_ANIMATION,
    BP_CHARACTER_STATE_SEQUENCE
};

/// Union of possible data options for the state's image
union bp_character_state_image_u {
    char no_data;
    bp_character_image_descriptor_s image;
    bp_character_state_animation_descriptor_s animation;
    bp_character_state_sequence_descriptor_s sequence;
};

/// (state.bin) Definition of character's state
struct bp_character_state_file_s {
    bp_character_state_image_e image_type;
    bp_character_state_image_u image;
};

/// Enum for whatever action user might want to invoke in the state machine
enum bp_character_action_e {
    BP_CHARACTER_ACTION_SWITCH_STATE,
    BP_CHARACTER_ACTION_START_ANIMATION
};

/// Union of possible action data
union bp_character_action_u {
    char no_data;
    char state_name[bp::data::STATE_NAME_MAX_LEN];
    char animation[bp::data::ANIMATION_NAME_MAX_LEN];
};

/// (action.bin) Definition of character action that can be performed from bluetooth
struct bp_character_action_file_s {
    char display[bp::data::ACTION_DISPLAY_MAX_LEN];
    bp_character_action_e type;
    bp_character_action_u data;
};
} // extern "C"
