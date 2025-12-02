#include "character.hpp"

#include <fstream>
#include <string>
#include <filesystem>
#include "format.hpp"

#include "esp_log.h"
#include "freertos/FreeRTOS.h"

namespace fs = std::filesystem;

constexpr auto TAG = "bp_data";

namespace bp::data {
    std::vector<std::string> list_characters() {
        std::vector<std::string> names{};

        for (const auto& entry: fs::directory_iterator{CHARACTERS_PATH}) {
            if (!entry.is_directory()) continue;
            names.emplace_back(entry.path().filename());
        }

        return names;
    }

    std::optional<std::string> get_selected_character_name() {
        return get_selected_character_name(list_characters());
    }

    std::optional<std::string> get_selected_character_name(const std::vector<std::string>& characters) {
        if (characters.empty()) return std::nullopt;

        for (const auto& name: characters) {
            if (const fs::path selected_path{std::format("{}/{}/selected.lock", CHARACTERS_PATH, name)};
                fs::exists(selected_path)) {
                return name;
            }
        }

        return std::nullopt;
    }

    std::optional<Character> load_selected_character() {
        return load_selected_character(list_characters());
    }

    std::optional<Character> load_selected_character(const std::vector<std::string>& characters) {
        if (const auto selected_character_name = get_selected_character_name(characters)) {
            return load_character_data(*selected_character_name);
        }

        return std::nullopt;
    }

    void select_character(const std::string& name) {
        select_character(list_characters(), name);
    }

    void select_character(const std::vector<std::string>& characters, const std::string& name) {
        if (const auto existing_selection = get_selected_character_name(characters)) {
            fs::remove(std::format("{}/{}/selected.lock", CHARACTERS_PATH, *existing_selection));
        }

        const fs::path lock_filename{std::format("{}/{}/selected.lock", CHARACTERS_PATH, name)};
        std::ofstream lock_file(lock_filename);
        lock_file << '1';
        lock_file.close();
    }

    Character load_character_data(const std::string& name) {
        Character character{};
        load_character_data(character, name);
        return character;
    }

    void load_character_data(Character& character, const std::string& name) {
        const fs::path char_folder{std::format("{}/{}", CHARACTERS_PATH, name)};

        {
            bp_character_file_s character_struct{};

            {
                const fs::path char_filename = char_folder / "character.bin";
                auto character_file = std::make_unique<std::ifstream>(char_filename);
                character_file->read(
                    reinterpret_cast<std::istream::char_type*>(&character_struct),
                    sizeof(bp_character_file_s)
                );
                character_file->close();
            }

            if (character_struct.format_version != FORMAT_VERSION) {
                throw data_exception{Error::IncompatibleFiles};
            }

            character.id = name;
            character.name = character_struct.name;
            character.species = character_struct.species;
            character.default_state = character_struct.default_state;
        }

        StrMap<State>& states = character.states;
        states.clear();

        if (const fs::path state_folder = char_folder / "states"; fs::exists(state_folder)) {
            for (const auto& state_entry: fs::directory_iterator{state_folder}) {
                if (!state_entry.is_directory()) continue;

                bp_character_state_file_s state_struct{};

                {
                    const fs::path state_filename = state_entry.path() / "state.bin";
                    auto state_file = std::make_unique<std::ifstream>(state_filename);
                    state_file->read(
                        reinterpret_cast<std::istream::char_type*>(&state_struct),
                        sizeof(bp_character_state_file_s)
                    );
                    state_file->close();
                }

                auto state_pair = states.emplace(state_entry.path().filename(), State{}).first;
                auto& [image, transitions] = state_pair->second;

                switch (state_struct.image_type) {
                    case BP_CHARACTER_STATE_NO_IMAGE:
                        image = std::monostate{};
                        break;
                    case BP_CHARACTER_STATE_SINGLE_IMAGE: {
                        auto& [image_name, width, height, has_alpha, upscale, preload] = state_struct.image.image;
                        image = StateImage{
                            .image_name = image_name,
                            .width = width,
                            .height = height,
                            .has_alpha = has_alpha,
                            .upscale = upscale,
                            .preload = preload
                        };
                        break;
                    }
                    case BP_CHARACTER_STATE_ANIMATION: {
                        auto& [name, next_state, loop_count, preload] = state_struct.image.animation;
                        image = StateAnimation{
                            .name = name,
                            .next_state = next_state,
                            .loop_count = loop_count,
                            .preload = preload
                        };
                        break;
                    }
                    case BP_CHARACTER_STATE_SEQUENCE: {
                        auto& [frame_count, mode] = state_struct.image.sequence;

                        image = StateSequence{
                            .frames{},
                        };

                        // ReSharper disable once CppUseStructuredBinding
                        auto& sequence = std::get<StateSequence>(image);

                        switch (mode) {
                            case BP_CHARACTER_SEQUENCE_MODE_LOAD_ALL:
                                sequence.mode = SequenceLoadMode::LoadAll;
                                break;
                            case BP_CHARACTER_SEQUENCE_MODE_LOAD_EACH:
                                sequence.mode = SequenceLoadMode::LoadEach;
                                break;
                            case BP_CHARACTER_SEQUENCE_MODE_PRELOAD:
                                sequence.mode = SequenceLoadMode::Preload;
                                break;
                        }

                        for (int frame_index = 0; frame_index < frame_count; frame_index++) {
                            const auto frame_path = state_entry.path() / "frames" / std::format("{}.bin", frame_index);

                            if (!fs::exists(frame_path)) continue;

                            bp_sequence_frame_file_s frame_struct{};

                            {
                                auto frame_file = std::make_unique<std::ifstream>(frame_path);
                                frame_file->read(
                                    reinterpret_cast<std::istream::char_type*>(&frame_struct),
                                    sizeof(bp_sequence_frame_file_s)
                                );
                                frame_file->close();
                            }

                            sequence.frames.emplace_back(
                                frame_struct.image_name,
                                frame_struct.width,
                                frame_struct.height,
                                frame_struct.has_alpha,
                                frame_struct.upscale,
                                frame_struct.duration_us
                            );
                        }
                        break;
                    }
                    default:
                        break;
                }

                {
                    if (const fs::path transition_folder = state_entry.path() / "transitions";
                        fs::exists(transition_folder)) {
                        for (const auto& transition_entry: fs::directory_iterator(transition_folder)) {
                            if (!transition_entry.is_directory()) continue;

                            bp_state_transition_file_s transition_struct{};

                            {
                                const fs::path transition_filename = transition_entry.path() / "transition.bin";
                                auto transition_file = std::make_unique<std::ifstream>(transition_filename);
                                transition_file->read(
                                    reinterpret_cast<std::istream::char_type*>(&transition_struct),
                                    sizeof(bp_state_transition_file_s)
                                );
                                transition_file->close();
                            }

                            StateTransition transition{
                                .next_state = transition_entry.path().filename()
                            };

                            switch (transition_struct.trigger.type) {
                                case BP_STATE_TRIGGER_ELAPSED_TIME:
                                    transition.trigger = StateTransitionElapsedTime{
                                        transition_struct.trigger.data.state_duration_us
                                    };
                                    break;
                                case BP_STATE_TRIGGER_CLICKED:
                                    transition.trigger = StateTransitionClicked{};
                                    break;
                            }

                            transitions.emplace_back(std::move(transition));
                        }
                    }
                }
            }
        }


        StrMap<Animation>& animations = character.animations;
        animations.clear();

        if (const fs::path animations_folder = char_folder / "animations"; fs::exists(animations_folder)) {
            for (const auto& animation_entry: fs::directory_iterator{animations_folder}) {
                if (!animation_entry.is_directory()) continue;

                bp_character_animation_file_s animation_struct{};

                {
                    const fs::path animation_filename = animation_entry.path() / "animation.bin";
                    auto animation_file = std::make_unique<std::ifstream>(animation_filename);
                    animation_file->read(
                        reinterpret_cast<std::istream::char_type*>(&animation_struct),
                        sizeof(bp_character_animation_file_s)
                    );
                    animation_file->close();
                }

                auto& [
                    x,
                    y,
                    width,
                    height,
                    frame_count,
                    interval_us,
                    clear_screen,
                    background_color,
                    mode,
                    upscale
                ] = animation_struct;

                Animation animation{
                    .x = x,
                    .y = y,
                    .width = width,
                    .height = height,
                    .frame_count = frame_count,
                    .interval_us = interval_us,
                    .clear_screen = clear_screen,
                    .background_color = background_color,
                    .upscale = upscale
                };

                switch (mode) {
                    case BP_CHARACTER_ANIMATION_MODE_FROM_SDCARD:
                        animation.mode = AnimationMode::FromSDCard;
                        break;
                    case BP_CHARACTER_ANIMATION_MODE_FROM_RAM:
                        animation.mode = AnimationMode::FromRAM;
                        break;
                }

                animations.emplace(animation_entry.path().filename(), animation);
            }
        }


        std::map<std::string, Action>& actions = character.actions;
        actions.clear();

        if (const fs::path actions_folder = char_folder / "actions"; fs::exists(actions_folder)) {
            for (const auto& action_entry: fs::directory_iterator{actions_folder}) {
                if (!action_entry.is_directory()) continue;

                bp_character_action_file_s action_struct{};

                {
                    const fs::path action_filename = action_entry.path() / "action.bin";
                    auto action_file = std::make_unique<std::ifstream>(action_filename);
                    action_file->read(
                        reinterpret_cast<std::istream::char_type*>(&action_struct),
                        sizeof(bp_character_action_file_s)
                    );
                    action_file->close();
                }

                Action action{
                    .display = action_struct.display
                };

                switch (action_struct.type) {
                    case BP_CHARACTER_ACTION_SWITCH_STATE:
                        action.type = ActionSwitchState{
                            .state_name = action_struct.data.state_name
                        };
                        break;
                }

                actions.emplace(action_entry.path().filename(), std::move(action));
            }
        }
    }

    PreloadedData preload_data(const Character& character) {
        PreloadedData preloaded_data{};
        preload_data(preloaded_data, character);
        return preloaded_data;
    }

    lv_image_dsc_t make_image_dsc(
        const bool has_alpha, const uint32_t width, const uint32_t height,
        const ImageDataVec& image_data
    ) {
        return {
            .header{
                .magic = LV_IMAGE_HEADER_MAGIC,
                .cf = static_cast<uint32_t>(has_alpha ? LV_COLOR_FORMAT_RGB565A8 : LV_COLOR_FORMAT_RGB565),
                .w = width,
                .h = height
            },
            .data_size = static_cast<uint32_t>(image_data.size()),
            .data = image_data.data()
        };
    }

    void preload_image(
        PreloadedData& preloaded_data, const std::string& image_name, const fs::path& images_folder,
        const bool has_alpha, const uint32_t width, const uint32_t height
    ) {
        ImageDataVec image_data{};

        {
            uintmax_t file_size = 0;
            const fs::path image_filename = images_folder / std::format("{}.bin", image_name);

            file_size = fs::file_size(image_filename);

            if (heap_caps_get_largest_free_block(MALLOC_CAP_SPIRAM) < file_size) {
                throw data_exception{Error::OutOfRAM};
            }

            image_data.resize(file_size);

            const auto image_file = std::make_unique<std::ifstream>(image_filename);
            image_file->read(
                reinterpret_cast<std::istream::char_type*>(image_data.data()),
                static_cast<std::streamsize>(file_size)
            );
            image_file->close();
        }

        preloaded_data.image_data.emplace(
            image_name,
            std::make_tuple(make_image_dsc(has_alpha, width, height, image_data), std::move(image_data))
        );
    }

    void preload_data(PreloadedData& preloaded_data, const Character& character) {
        const fs::path char_folder{std::format("{}/{}", CHARACTERS_PATH, character.id)};

        for (const fs::path images_folder = char_folder / "images";
             const auto& [_, state]: character.states) {
            if (const auto* image = std::get_if<StateImage>(&state.image)) {
                if (!image->preload) continue;
                preload_image(
                    preloaded_data, image->image_name, images_folder,
                    image->has_alpha, image->width, image->height
                );
            } else if (const auto* sequence = std::get_if<StateSequence>(&state.image)) {
                if (sequence->mode != SequenceLoadMode::Preload) continue;

                for (const auto& frame : sequence->frames) {
                    preload_image(
                        preloaded_data, frame.image_name, images_folder,
                        frame.has_alpha, frame.width, frame.height
                    );
                }
            }
        }

        for (const fs::path animations_folder = char_folder / "animations";
             const auto& [_, state]: character.states) {
            if (!std::holds_alternative<StateAnimation>(state.image)) continue;

            const auto& state_anim = std::get<StateAnimation>(state.image);
            if (!state_anim.preload) continue;

            const auto& anim_desc = character.animations.at(state_anim.name);
            const auto data_size = anim_desc.width * anim_desc.height * ANIMATION_BYTES_PER_PIXEL;

            const fs::path frames_folder = animations_folder / state_anim.name / "frames";

            std::vector<ImageDataVec> frames{};
            frames.reserve(anim_desc.frame_count);

            for (int frame_index = 1; frame_index <= anim_desc.frame_count; frame_index++) {
                if (heap_caps_get_largest_free_block(MALLOC_CAP_SPIRAM) < data_size) {
                    throw data_exception{Error::OutOfRAM};
                }

                ImageDataVec frame(static_cast<std::size_t>(data_size));

                {
                    const fs::path image_filename = frames_folder / std::format("{}.bin", frame_index);
                    const auto image_file = std::make_unique<std::ifstream>(image_filename);
                    image_file->read(
                        reinterpret_cast<std::istream::char_type*>(frame.data()),
                        static_cast<std::streamsize>(data_size)
                    );
                    image_file->close();
                }

                frames.emplace_back(std::move(frame));
            }

            preloaded_data.animation_frames.emplace(state_anim.name, std::move(frames));
        }
    }

    data_exception::data_exception(const Error kind) {
        this->kind = kind;
    }

    const char* data_exception::what() const noexcept {
        switch (this->kind) {
            case Error::IncompatibleFiles:
                return "incompatible files";
            default:
                return "unknown error";
        }
    }
}
