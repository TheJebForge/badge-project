#pragma once

#include <filesystem>
#include <memory>
#include <string>
#include <map>
#include <unordered_map>
#include <variant>
#include <vector>

#include <lvgl.h>

#include "character.hpp"
#include "character.hpp"
#include "image.hpp"
#include "../util/allocator.hpp"

namespace bp::data {
    constexpr auto CHARACTERS_PATH = "/sdcard/characters";
    constexpr uint32_t ANIMATION_BYTES_PER_PIXEL = 2;

    struct Character;

    struct StateTransitionElapsedTime {
        int64_t duration_us;
    };

    struct StateTransitionRandom {
        int64_t duration_start_range;
        int64_t duration_end_range;
        uint32_t chance_mod;
    };

    struct StateTransitionClicked {};

    struct StateTransition {
        std::string next_state;
        std::variant<StateTransitionElapsedTime, StateTransitionClicked, StateTransitionRandom> trigger;
    };

    struct StateImage {
        std::string image_name;
        uint32_t width;
        uint32_t height;
        bool has_alpha;
        bool upscale;
        bool preload;

        [[nodiscard]] bool image_exists(const Character& character) const;
        [[nodiscard]] std::size_t get_image_size(const Character& character) const;
        void load_image(const Character& character, std::span<uint8_t> buffer) const;
    };

    struct StateAnimation {
        std::string name;
        std::string next_state;
        uint16_t loop_count;
        bool preload;
        std::filesystem::path frames_folder;

        void load_frame(std::span<uint8_t> buffer, std::size_t index) const;
    };

    struct SequenceFrame {
        std::string image_name;
        uint32_t width;
        uint32_t height;
        bool has_alpha;
        bool upscale;
        int64_t duration_us;

        [[nodiscard]] bool image_exists(const Character& character) const;
        [[nodiscard]] std::size_t get_image_size(const Character& character) const;
        void load_image(const Character& character, std::span<uint8_t> buffer) const;
    };

    enum class SequenceLoadMode {
        LoadAll,
        LoadEach,
        Preload
    };

    struct StateSequence {
        std::vector<SequenceFrame> frames;
        SequenceLoadMode mode;

        [[nodiscard]] bool frame_exists(const Character& character, std::size_t index) const;
        [[nodiscard]] std::size_t get_frame_size(const Character& character, std::size_t index) const;
        void load_frame(const Character& character, std::span<uint8_t> buffer, std::size_t index) const;
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
        std::filesystem::path folder;
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
        std::filesystem::path folder;
        std::filesystem::path animations_folder;
        std::filesystem::path images_folder;

        std::filesystem::path get_image_path(const std::string& name) const;
        bool image_exists(const std::string& name) const;
        std::size_t get_image_size(const std::string& name) const;
        void load_image(std::span<uint8_t> buffer, const std::string& name) const;
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

    using ImageDataVec = std::vector<uint8_t, PsramAllocator<uint8_t>>;

    lv_image_dsc_t make_image_dsc(bool has_alpha, uint32_t width, uint32_t height, const ImageDataVec& image_data);
    lv_image_dsc_t make_image_dsc(bool has_alpha, uint32_t width, uint32_t height, const image::SharedAllocatedImageData& image_data);

    void load_image_data(std::span<uint8_t> buffer, const std::filesystem::path& path);

    struct PreloadedData {
        StrMap<std::tuple<lv_image_dsc_t, image::SharedAllocatedImageData>> image_data;
        StrMap<std::vector<image::SharedAllocatedImageData>> animation_frames;
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