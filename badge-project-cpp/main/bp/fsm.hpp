#pragma once

#include <unordered_set>
#include "data/character.hpp"
#include "data/image.hpp"
#include "init/bluetooth.hpp"

namespace bp {
    enum class ClientCommandType : uint8_t {
        GetAction,
        GetActionDisplayName,
        InvokeAction,
        GetCharacter,
        SwitchCharacter
    };

    ClientCommandResponse bluetooth_command_handler(uint8_t op, std::span<char, 200> data);
    void layer_loader();
    void single_image_cooker(const data::StateImage* image_state);
    void animation_cooker(const data::StateAnimation* anim_state);
    void sequence_cooker(const data::StateSequence* sequence);

    class CharacterFSM {
        bool ready = false;
        bool busy = false;
        bool in_tick = false;

        // Character data
        data::Character character_data;
        data::LoadedLayerData loaded_layer_data;

        // FSM State
        std::string current_state;
        int64_t last_transition_time = 0;
        std::size_t current_sequence_index = -1;
        int64_t next_frame_time = 0;
        std::optional<std::string> queued_state;
        std::unordered_map<std::string, int64_t> random_durations{};

        uint16_t prepared_load_layer;
        bool preparing_load_layer = false;
        std::optional<std::string> preparing_load_layer_for;
        bool preparing_ui_dirty = false;
        std::unordered_set<std::string> layer_images_to_remove;
        std::unordered_set<std::string> layer_anims_to_remove;

        std::string being_cooked_state;
        bool state_is_cooking = false;

        bool cooking_progress_dirty = false;
        bool new_cooking_visible = false;
        int32_t new_cooking_current = 0;
        int32_t new_cooking_max = 0;

        // FSM Image data
        std::vector<lv_image_dsc_t> prepared_descriptors{};
        std::vector<image::SharedAllocatedImageData> prepared_images{};
        std::vector<lv_image_dsc_t> loaded_descriptors{};
        std::vector<image::SharedAllocatedImageData> loaded_images{};

        image::SharedAllocatedImageData current_image;
        lv_image_dsc_t current_descriptor;

        // UI Objects
        lv_obj_t* screen_obj = nullptr;
        lv_obj_t* char_name_obj = nullptr;
        lv_obj_t* char_species_obj = nullptr;
        lv_obj_t* image_obj = nullptr;
        lv_obj_t* progress_box_obj = nullptr;
        lv_obj_t* progress_bar_obj = nullptr;
        lv_obj_t* error_box_obj = nullptr;
        lv_obj_t* error_text_obj = nullptr;
        bool ui_dirty = false;

        // Synchronization
        portMUX_TYPE spinlock = portMUX_INITIALIZER_UNLOCKED;

        class BusyLock {
            bool still_holding;
            CharacterFSM* parent;

        public:
            explicit BusyLock(CharacterFSM* parent);
            void free();
            ~BusyLock();
        };


        bool is_data_in_use();
        bool is_free_sl();
        bool is_busy_sl();
        bool is_cooking_sl();
        BusyLock get_busy_sl();
        void wait_until_free_sl();
        void wait_until_data_unused_sl();
        void wait_until_not_busy_sl();

        void play_animation(
            const data::StateAnimation& state_desc, const data::Animation& animation_desc
        ) const;
        void update_display(const image::SharedAllocatedImageData& image, const lv_image_dsc_t& desc, bool upscale);
        void set_ui_image(const data::StateImageVariant& variant);

        void address_queue();
        void switch_state_internal(const std::string& state_name);
        void switch_state_unchecked(const std::string& state_name);

        int64_t get_random_duration(const std::string& state_name, const data::StateTransitionRandom& rng_specs, int64_t time_since_transition);
        void clear_random_duration(const std::string& state_name);

        bool cook_if_needed(const data::StateImageVariant& image) const;
        void set_progress_visible(bool visibility);
        void set_cooking_progress(int32_t current, int32_t max);
        void update_progress_if_needed();
        void done_cooking_sl(bool success);

        void remove_unneeded_layer_date();

        void display_error(const std::string& error) const;

    public:
        bool alive = true;

        void create_ui();
        bool is_ready_sl();
        void load_character_sl(const std::string& name);
        data::State& get_current_state_sl();
        void switch_state_sl(const std::string& next_state);
        void switch_to_default_sl();
        bool invoke_action_sl(const std::string& action_id);
        void mark_dirty();
        void tick();

        friend ClientCommandResponse bluetooth_command_handler(uint8_t, std::span<char, 200>);
        friend void layer_loader();
        friend void single_image_cooker(const data::StateImage* image_state);
        friend void animation_cooker(const data::StateAnimation* anim_state);
        friend void sequence_cooker(const data::StateSequence* sequence);
    };

    void start_fsm_task();
}
