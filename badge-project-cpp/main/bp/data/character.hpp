#pragma once

#include <memory>
#include <string>
#include <map>
#include <unordered_map>
#include <variant>
#include <vector>

#include <lvgl.h>

#include "../util/allocator.hpp"

namespace bp::data {
    struct StateTransitionElapsedTime {
        int64_t duration_us;
    };

    struct StateTransitionClicked {};

    struct StateTransition {
        std::string next_state;
        std::variant<StateTransitionElapsedTime, StateTransitionClicked> trigger;
    };

    struct StateImage {
        std::string image_name;
        uint32_t width;
        uint32_t height;
        bool has_alpha;
        bool upscale;
        bool preload;
    };

    struct StateAnimation {
        std::string name;
        std::string next_state;
        uint16_t loop_count;
        bool preload;
    };

    struct SequenceFrame {
        std::string image_name;
        uint32_t width;
        uint32_t height;
        bool has_alpha;
        bool upscale;
        int64_t duration_us;
    };

    enum class SequenceLoadMode {
        LoadAll,
        LoadEach,
        Preload
    };

    struct StateSequence {
        std::vector<SequenceFrame> frames;
        SequenceLoadMode mode;
    };

    using StateImageVariant = std::variant<std::monostate, StateImage, StateAnimation, StateSequence>;

    struct State {
        StateImageVariant image;
        std::vector<StateTransition> transitions;
    };

    template<typename T>
    using StrMap = std::unordered_map<std::string, T>;

    enum class AnimationMode {
        FromSDCard,
        FromRAM
    };

    struct Animation {
        uint16_t x;
        uint16_t y;
        uint32_t width;
        uint32_t height;
        uint32_t frame_count;
        int64_t interval_us;
        bool clear_screen;
        uint16_t background_color;
        AnimationMode mode;
        bool upscale;
    };

    struct ActionSwitchState {
        std::string state_name;
    };

    struct Action {
        std::string display;
        std::variant<ActionSwitchState> type;
    };

    struct Character {
        std::string id;
        std::string name;
        std::string species;
        std::string default_state;
        StrMap<State> states;
        StrMap<Animation> animations;
        std::map<std::string, Action> actions;
    };

    std::vector<std::string> list_characters();
    /// @throws data_exception If character files are incompatible
    Character load_character_data(const std::string& name);
    void load_character_data(Character& character, const std::string& name);

    std::optional<std::string> get_selected_character_name();
    std::optional<std::string> get_selected_character_name(const std::vector<std::string>& characters);
    void select_character(const std::string& name);
    void select_character(const std::vector<std::string>& characters, const std::string& name);
    std::optional<Character> load_selected_character();
    std::optional<Character> load_selected_character(const std::vector<std::string>& characters);



    using ImageDataVec = std::vector<uint8_t, PSRAMAllocator<uint8_t>>;

    struct PreloadedData {
        StrMap<std::tuple<lv_image_dsc_t, ImageDataVec>> image_data;
        StrMap<std::vector<ImageDataVec>> animation_frames;
    };

    /// @throws data_exception If there's not enough RAM
    /// @throws std::out_of_range If animation wasn't found
    PreloadedData preload_data(const Character& character);

    /// @throws data_exception If there's not enough RAM
    /// @throws std::out_of_range If animation wasn't found
    void preload_data(PreloadedData& preloaded_data, const Character& character);

    enum class Error {
        IncompatibleFiles,
        OutOfRAM
    };

    class data_exception final : public std::exception {
        Error kind;
    public:
        explicit data_exception(Error kind);
        [[nodiscard]] const char* what() const noexcept override;
    };

    // class Character {
    //     std::string name;
    //     std::string species;
    //     std::string default_state;
    //
    //     int64_t last_transition_time = 0;
    //     StateMap states;
    //     std::shared_ptr<State> current_state;
    //
    // public:
    //     explicit Character(const bp_character_file_s& character);
    //
    //     [[nodiscard]] const std::string& get_name() const;
    //     [[nodiscard]] const std::string& get_species() const;
    //     [[nodiscard]] const std::string& get_default_state() const;
    //
    //     [[nodiscard]] const StateMap& get_states() const;
    //     [[nodiscard]] const State& get_current_state() const;
    //
    //     bool set_state(const std::string& state_name);
    //
    //     friend Character test_character();
    // };
    //
    // Character test_character();
}