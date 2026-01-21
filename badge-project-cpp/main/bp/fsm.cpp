#include "fsm.hpp"

#include <fstream>
#include <filesystem>
#include "esp_lcd_panel_ops.h"
#include "esp_log.h"
#include "data/character.hpp"
#include "freertos/FreeRTOS.h"
#include "esp_timer.h"
#include "init/display.hpp"
#include "misc/lv_event_private.h"
#include "util/critical.hpp"
#include "util/frame_timer.hpp"
#include "util/lvgl_lock.hpp"
#include "util/task_delete.hpp"

constexpr auto TAG = "char_fsm";
constexpr uint32_t TASK_STACK = 0x2000;
constexpr UBaseType_t TASK_PRIORITY = configMAX_PRIORITIES / 2;
constexpr TickType_t TASK_INTERVAL = 50 / portTICK_PERIOD_MS;

constexpr auto PROGRESS_BAR_HEIGHT = 3;

constexpr uint32_t COOKER_STACK = 0x1000;
constexpr TickType_t COOKER_LOAD_DELAY = 30 / portTICK_PERIOD_MS;

using namespace bp;
namespace fs = std::filesystem;

static TaskHandle_t fsm_task_handle;

static CharacterFSM char_fsm;

static std::vector<std::string> character_names;

ClientCommandResponse bp::bluetooth_command_handler(
    uint8_t op, const std::span<char, 200> data
) {
    switch (static_cast<ClientCommandType>(op)) {
        case ClientCommandType::GetAction: {
            uint16_t action_index = 0;
            memcpy(&action_index, data.data(), sizeof action_index);

            ESP_LOGI(TAG, "GetAction(%d)", action_index);

            for (const auto& actions = char_fsm.character_data.actions;
                 const auto& [id, _]: actions) {
                if (action_index-- == 0) {
                    ESP_LOGI(TAG, "returning '%s'", id.c_str());
                    return {true, id};
                }
            }

            return {false, "Unknown action"};
        }

        case ClientCommandType::GetActionDisplayName: {
            std::string action_id{data.data(), data.size()};
            action_id.erase(std::ranges::find(action_id, '\0'), action_id.end());

            ESP_LOGI(TAG, "GetActionDisplayName(%s)", action_id.c_str());

            try {
                const auto& [display, _] = char_fsm.character_data.actions.at(action_id);
                ESP_LOGI(TAG, "returning '%s'", display.c_str());
                return {true, display};
            } catch (const std::out_of_range&) {
                ESP_LOGI(TAG, "returning unknown");
                return {false, "Unknown action"};
            }
        }

        case ClientCommandType::InvokeAction: {
            std::string action_id{data.data(), data.size()};
            action_id.erase(std::ranges::find(action_id, '\0'), action_id.end());

            ESP_LOGI(TAG, "InvokeAction(%s)", action_id.c_str());

            if (char_fsm.invoke_action_sl(action_id)) {
                return {true, ""};
            }

            return {false, "Unknown action"};
        }

        case ClientCommandType::GetCharacter: {
            uint16_t character_index = 0;
            memcpy(&character_index, data.data(), sizeof character_index);

            ESP_LOGI(TAG, "GetCharacter(%d)", character_index);

            for (const auto& name: character_names) {
                if (character_index-- == 0) {
                    ESP_LOGI(TAG, "returning '%s'", name.c_str());
                    return {true, name};
                }
            }

            return {false, "Unknown action"};
        }

        case ClientCommandType::SwitchCharacter: {
            std::string character_name{data.data(), data.size()};
            character_name.erase(std::ranges::find(character_name, '\0'), character_name.end());

            ESP_LOGI(TAG, "SwitchCharacter(%s)", character_name.c_str());

            if (std::ranges::find(character_names, character_name) != character_names.end()) {
                ESP_LOGI(TAG, "trying to load character '%s'", character_name.c_str());

                data::select_character(character_names, character_name);
                char_fsm.load_character_sl(character_name);

                return {true, ""};
            }

            return {false, "Unknown character"};
        }

        default:
            ESP_LOGI(TAG, "Received unknown command: %d", op);
            return {false, {}};
    }
}

// ReSharper disable once CppParameterMayBeConstPtrOrRef
void image_clicked(lv_event_t* event) {
    ESP_LOGI(TAG, "Image clicked!");
    auto* instance = static_cast<CharacterFSM*>(event->user_data);

    for (const auto [_, transitions] = instance->get_current_state_sl();
         const auto& [next_state, trigger]: transitions) {
        if (std::holds_alternative<data::StateTransitionClicked>(trigger)) {
            instance->switch_state_sl(next_state);
        }
    }
}

void CharacterFSM::create_ui() {
    {
        LVGLLockGuard guard(0);
        screen_obj = lv_obj_create(nullptr);

        lv_obj_t* header = lv_obj_create(screen_obj);
        lv_obj_set_size(header, DISPLAY_WIDTH, DISPLAY_HEIGHT - DISPLAY_WIDTH);
        lv_obj_set_layout(header, LV_LAYOUT_FLEX);
        lv_obj_set_flex_flow(header, LV_FLEX_FLOW_COLUMN);
        lv_obj_set_flex_align(header, LV_FLEX_ALIGN_CENTER, LV_FLEX_ALIGN_CENTER, LV_FLEX_ALIGN_CENTER);
        lv_obj_set_style_pad_row(header, 1, 0);
        lv_obj_set_width(header, lv_pct(100));
        lv_obj_set_style_border_width(header, 0, 0);

        char_name_obj = lv_label_create(header);
        lv_obj_set_style_text_font(char_name_obj, &lv_font_montserrat_36, 0);
        lv_label_set_text(char_name_obj, character_data.name.c_str());

        lv_obj_t* image_button = lv_button_create(screen_obj);
        lv_obj_set_pos(image_button, 0, DISPLAY_HEIGHT - DISPLAY_WIDTH);
        lv_obj_set_size(image_button, DISPLAY_WIDTH, DISPLAY_WIDTH);
        lv_obj_add_event_cb(image_button, image_clicked, LV_EVENT_CLICKED, this);

        image_obj = lv_image_create(image_button);
        lv_obj_set_size(image_obj, DISPLAY_WIDTH, DISPLAY_WIDTH);
        lv_obj_center(image_obj);
        lv_image_set_antialias(image_obj, false);

        progress_box_obj = lv_obj_create(screen_obj);
        lv_obj_set_size(progress_box_obj, DISPLAY_WIDTH, PROGRESS_BAR_HEIGHT);
        lv_obj_set_pos(progress_box_obj, 0, DISPLAY_HEIGHT - PROGRESS_BAR_HEIGHT);
        lv_obj_set_flag(progress_box_obj, LV_OBJ_FLAG_SCROLLABLE, false);
        lv_obj_set_flag(progress_box_obj, LV_OBJ_FLAG_HIDDEN, true);
        lv_obj_set_style_pad_all(progress_box_obj, 0, 0);

        progress_bar_obj = lv_obj_create(progress_box_obj);
        lv_obj_set_size(progress_bar_obj, 50, PROGRESS_BAR_HEIGHT);
        lv_obj_set_style_bg_color(progress_bar_obj, lv_color_hex(0xffffff), 0);

        constexpr auto ERROR_HEIGHT = 40;
        error_box_obj = lv_obj_create(screen_obj);
        lv_obj_set_size(error_box_obj, DISPLAY_WIDTH, ERROR_HEIGHT);
        lv_obj_set_pos(error_box_obj, 0, DISPLAY_HEIGHT - ERROR_HEIGHT);
        lv_obj_set_flag(error_box_obj, LV_OBJ_FLAG_SCROLLABLE, false);
        lv_obj_set_flag(error_box_obj, LV_OBJ_FLAG_HIDDEN, true);

        error_text_obj = lv_label_create(error_box_obj);
        lv_obj_center(error_text_obj);
        lv_obj_set_style_text_font(error_text_obj, &lv_font_montserrat_16, 0);
        lv_obj_set_style_text_align(error_text_obj, LV_TEXT_ALIGN_CENTER, 0);
        lv_obj_set_style_text_color(error_text_obj, lv_color_hex(0xFF3300), 0);

        lv_screen_load(screen_obj);
    }

    mark_dirty();
}

bool CharacterFSM::is_ready_sl() {
    CriticalGuard guard(&spinlock);
    return ready;
}

void CharacterFSM::load_character_sl(const std::string& name) {
    wait_until_data_unused_sl();

    {
        CriticalGuard guard(&spinlock);
        ready = false;
    }

    bp::data::load_character_data(character_data, name);
    bp::data::preload_data(preloaded_data, character_data);

    loaded_images = {};
    loaded_descriptors = {};

    switch_state_unchecked(character_data.default_state);

    if (char_name_obj != nullptr) {
        LVGLLockGuard guard(0);

        lv_label_set_text(char_name_obj, character_data.name.c_str());
    }

    bp_characteristics->set_character_info(
        name,
        character_data.name,
        character_data.species,
        character_data.actions.size()
    );

    {
        CriticalGuard guard(&spinlock);
        ready = true;
    }
}

data::State CharacterFSM::get_current_state_sl() {
    CriticalGuard guard{&spinlock};
    return character_data.states.at(current_state);
}

static portMUX_TYPE checker_spinlock = portMUX_INITIALIZER_UNLOCKED;

bool check_if_no_ram_sl(const std::size_t wanted_ram) {
    CriticalGuard guard(&checker_spinlock);
    return heap_caps_get_largest_free_block(MALLOC_CAP_SPIRAM) < wanted_ram;
}

static TaskHandle_t cooker_task_handle;

constexpr auto IMG_TAG = "single_image_cooker";
void bp::single_image_cooker(const data::StateImage* image_state) {
    TaskDeleteGuard guard{};
    ESP_LOGI(IMG_TAG, "Cooking an image...");

    auto& frames = char_fsm.prepared_images;
    auto& descriptors = char_fsm.prepared_descriptors;

    frames.clear();
    descriptors.clear();

    image::SharedAllocatedImageData inserted_image;

    {
        if (!image_state->image_exists(char_fsm.character_data)) {
            ESP_LOGE(IMG_TAG, "Image '%s' not found!", image_state->image_name.c_str());
            char_fsm.done_cooking_sl(false);
            return;
        }

        const auto file_size = image_state->get_image_size(char_fsm.character_data);

        if (const auto opt_image = image::allocator.allocate_image_data_sl(file_size)) {
            inserted_image = opt_image.value();
        } else {
            ESP_LOGE(IMG_TAG, "Failed to allocate %d bytes!", file_size);
            char_fsm.done_cooking_sl(false);
            return;
        }

        image_state->load_image(char_fsm.character_data, inserted_image->span());
    }

    descriptors.emplace_back(data::make_image_dsc(
        image_state->has_alpha, image_state->width, image_state->height, inserted_image
    ));

    frames.emplace_back(std::move(inserted_image));

    char_fsm.set_cooking_progress(1, 1);

    frames.shrink_to_fit();
    char_fsm.done_cooking_sl(true);
}

constexpr auto ANIM_TAG = "animation_cooker";
void bp::animation_cooker(const data::StateAnimation* anim_state) {
    TaskDeleteGuard guard{};
    ESP_LOGI(ANIM_TAG, "Cooking an animation...");

    const auto& anim_desc = char_fsm.character_data.animations.at(anim_state->name);
    auto& frames = char_fsm.prepared_images;

    frames.clear();
    frames.reserve(anim_desc.frame_count);

    const auto data_size = anim_desc.width * anim_desc.height * data::ANIMATION_BYTES_PER_PIXEL;

    for (std::size_t frame_index = 1; frame_index <= anim_desc.frame_count; frame_index++) {
        image::SharedAllocatedImageData inserted_image;

        if (const auto opt_image = image::allocator.allocate_image_data_sl(data_size)) {
            inserted_image = opt_image.value();
        } else {
            ESP_LOGE(ANIM_TAG, "Failed to allocate %d bytes!", data_size);
            char_fsm.done_cooking_sl(false);
            return;
        }

        ESP_LOGI(ANIM_TAG, "Allocated at %x-%x for #%d frame", inserted_image->start(), inserted_image->end(), frame_index);

        anim_state->load_frame(inserted_image->span(), frame_index);

        frames.emplace_back(std::move(inserted_image));

        char_fsm.set_cooking_progress(frame_index, static_cast<int32_t>(anim_desc.frame_count));
        vTaskDelay(COOKER_LOAD_DELAY);
    }

    frames.shrink_to_fit();
    char_fsm.done_cooking_sl(true);
}

constexpr auto SEQ_TAG = "sequence_cooker";
void bp::sequence_cooker(const data::StateSequence* sequence) {
    TaskDeleteGuard guard{};
    ESP_LOGI(SEQ_TAG, "Cooking a sequence...");

    auto& frames = char_fsm.prepared_images;
    auto& descriptors = char_fsm.prepared_descriptors;

    frames.clear();
    descriptors.clear();

    if (sequence->mode == data::SequenceLoadMode::LoadAll) {
        frames.reserve(sequence->frames.size());
        descriptors.reserve(sequence->frames.size());

        for (std::size_t frame_index = 0; frame_index < sequence->frames.size(); frame_index++) {
            const auto& seq_frame = sequence->frames.at(frame_index);
            image::SharedAllocatedImageData inserted_image;

            {
                if (!seq_frame.image_exists(char_fsm.character_data)) {
                    ESP_LOGE(SEQ_TAG, "Image '%s' not found!", seq_frame.image_name.c_str());
                    char_fsm.done_cooking_sl(false);
                    return;
                }

                const auto file_size = seq_frame.get_image_size(char_fsm.character_data);

                if (const auto opt_image = image::allocator.allocate_image_data_sl(file_size)) {
                    inserted_image = opt_image.value();
                } else {
                    ESP_LOGE(SEQ_TAG, "Failed to allocate %d bytes!", file_size);
                    char_fsm.done_cooking_sl(false);
                    return;
                }

                seq_frame.load_image(char_fsm.character_data, inserted_image->span());
            }

            descriptors.emplace_back(data::make_image_dsc(
                seq_frame.has_alpha, seq_frame.width, seq_frame.height, inserted_image
            ));

            frames.emplace_back(std::move(inserted_image));

            char_fsm.set_cooking_progress(frame_index + 1, sequence->frames.size());
            vTaskDelay(COOKER_LOAD_DELAY);
        }
    } else if (sequence->mode == data::SequenceLoadMode::LoadEach && !sequence->frames.empty()) {
        std::size_t largest_frame_size = 0;
        for (const auto& seq_frame: sequence->frames) {
            if (
                auto size = seq_frame.get_image_size(char_fsm.character_data);
                size > largest_frame_size
            ) {
                largest_frame_size = size;
            }
        }

        for (std::size_t index = 0; index < 2; index++) {
            if (const auto opt_image = image::allocator.allocate_image_data_sl(largest_frame_size)) {
                ESP_LOGI(SEQ_TAG, "Allocated %x-%x for sequence buffer #%d", opt_image.value()->start(), opt_image.value()->end(), index);
                frames.emplace_back(std::move(opt_image.value()));
            } else {
                ESP_LOGE(SEQ_TAG, "Failed to allocate %d bytes!", largest_frame_size);
                char_fsm.done_cooking_sl(false);
                return;
            }
        }

        descriptors.resize(2);

        auto& first_image = frames[0];

        {
            const auto& first_seq_frame = sequence->frames.at(0);

            if (!first_seq_frame.image_exists(char_fsm.character_data)) {
                ESP_LOGE(SEQ_TAG, "Image '%s' not found!", first_seq_frame.image_name.c_str());
                char_fsm.done_cooking_sl(false);
                return;
            }

            first_seq_frame.load_image(char_fsm.character_data, first_image->span());
        }

        const auto& seq_frame = sequence->frames.at(0);
        descriptors[0] = data::make_image_dsc(
            seq_frame.has_alpha, seq_frame.width, seq_frame.height, first_image
        );

        char_fsm.set_cooking_progress(1, 1);
    }

    frames.shrink_to_fit();
    descriptors.shrink_to_fit();
    char_fsm.done_cooking_sl(true);
}

bool CharacterFSM::cook_if_needed(const std::string& state_name) const {
    const auto& [image, _] = character_data.states.at(state_name);

    if (const auto* image_state = std::get_if<data::StateImage>(&image)) {
        if (image_state->preload) return false;

        // Cook for image
        if (const portBASE_TYPE result = xTaskCreate(
            reinterpret_cast<TaskFunction_t>(single_image_cooker),
            IMG_TAG,
            COOKER_STACK,
            const_cast<data::StateImage*>(image_state),
            configMAX_PRIORITIES / 2,
            &cooker_task_handle
        ); result != pdPASS) {
            ESP_LOGE(TAG, "Failed to start cooker task! %d", result);
        }

        return true;
    }

    if (const auto* anim_state = std::get_if<data::StateAnimation>(&image)) {
        if (anim_state->preload) return false;
        if (const auto& anim_desc = character_data.animations.at(anim_state->name);
            anim_desc.mode != data::AnimationMode::FromRAM) return false;

        // Cook for anim frames
        if (const portBASE_TYPE result = xTaskCreate(
            reinterpret_cast<TaskFunction_t>(animation_cooker),
            ANIM_TAG,
            COOKER_STACK,
            const_cast<data::StateAnimation*>(anim_state),
            configMAX_PRIORITIES / 2,
            &cooker_task_handle
        ); result != pdPASS) {
            ESP_LOGE(TAG, "Failed to start cooker task! %d", result);
        }

        return true;
    }

    if (const auto* sequence_state = std::get_if<data::StateSequence>(&image)) {
        if (sequence_state->mode == data::SequenceLoadMode::Preload) return false;

        // Cook for sequence frames
        if (const portBASE_TYPE result = xTaskCreate(
            reinterpret_cast<TaskFunction_t>(sequence_cooker),
            SEQ_TAG,
            COOKER_STACK,
            const_cast<data::StateSequence*>(sequence_state),
            configMAX_PRIORITIES / 2,
            &cooker_task_handle
        ); result != pdPASS) {
            ESP_LOGE(TAG, "Failed to start cooker task! %d", result);
        }

        return true;
    }

    return false;
}

void CharacterFSM::set_progress_visible(const bool visibility) {
    CriticalGuard guard(&spinlock);
    cooking_progress_dirty = true;
    new_cooking_visible = visibility;
}

void CharacterFSM::set_cooking_progress(const int32_t current, const int32_t max) {
    CriticalGuard guard(&spinlock);
    cooking_progress_dirty = true;
    new_cooking_current = current;
    new_cooking_max = max;
}

void CharacterFSM::update_cooking_progress_if_needed() {
    bool dirty;

    {
        CriticalGuard guard(&spinlock);
        dirty = cooking_progress_dirty;
    }

    if (dirty) {
        LVGLLockGuard guard(0);

        lv_obj_set_flag(progress_box_obj, LV_OBJ_FLAG_HIDDEN, !new_cooking_visible);
        const auto progress = static_cast<float>(DISPLAY_WIDTH)
                              / static_cast<float>(new_cooking_max)
                              * static_cast<float>(new_cooking_current);
        lv_obj_set_size(progress_bar_obj, static_cast<int32_t>(progress), PROGRESS_BAR_HEIGHT);
    }
}

void CharacterFSM::switch_state_internal(const std::string& state_name) {
    current_state = state_name;
    last_transition_time = esp_timer_get_time();
    current_sequence_index = -1;
    next_frame_time = 0;
    mark_dirty();
}

void CharacterFSM::switch_state_unchecked(const std::string& state_name) {
    if (!cook_if_needed(state_name)) {
        ESP_LOGI(TAG, "Switching to '%s' state", state_name.c_str());

        CriticalGuard guard(&spinlock);
        switch_state_internal(state_name);

        // Clear unnecessary memory
        loaded_images = {};
        loaded_descriptors = {};
    } else {
        ESP_LOGI(TAG, "'%s' state needs to be cooked first, started task", state_name.c_str());

        {
            CriticalGuard guard(&spinlock);
            state_is_cooking = true;
            being_cooked_state = state_name;
        }

        set_cooking_progress(0, 100);
        set_progress_visible(true);
    }
}

void CharacterFSM::switch_state_sl(const std::string& next_state) {
    if (!is_free_sl() || !is_ready_sl()) {
        ESP_LOGI(TAG, "Can't switch to '%s' state right now, queuing if possible", next_state.c_str());

        CriticalGuard guard(&spinlock);
        if (!state_is_cooking && being_cooked_state != next_state) {
            queued_state = next_state;
        }
        return;
    }

    switch_state_unchecked(next_state);
}

void CharacterFSM::address_queue() {
    std::optional<std::string> to_queue;

    {
        CriticalGuard guard(&spinlock);
        if (queued_state && !(busy || state_is_cooking)) {
            to_queue = queued_state;
            queued_state = std::nullopt;
        }
    }

    if (to_queue) switch_state_sl(to_queue.value());
}

void CharacterFSM::switch_to_default_sl() {
    switch_state_sl(character_data.default_state);
}

void CharacterFSM::done_cooking_sl(const bool success) {
    wait_until_not_busy_sl();

    if (success) {
        ESP_LOGI(TAG, "Cooker reported success!");

        CriticalGuard guard(&spinlock);
        loaded_images = std::move(prepared_images);
        loaded_descriptors = std::move(prepared_descriptors);
        prepared_images = {};
        prepared_descriptors = {};
        state_is_cooking = false;

        switch_state_internal(being_cooked_state);

    } else {
        ESP_LOGI(TAG, "Cooker failed :(");

        CriticalGuard guard(&spinlock);
        state_is_cooking = false;
    }

    set_progress_visible(false);
}

static lv_timer_t* error_hide_timer_handle = nullptr;

void hide_error(lv_timer_t* timer) {
    auto* obj = static_cast<lv_obj_t*>(lv_timer_get_user_data(timer));
    lv_obj_set_flag(obj, LV_OBJ_FLAG_HIDDEN, true);
}

void CharacterFSM::display_error(const std::string& error) const {
    LVGLLockGuard guard(0);

    if (error_hide_timer_handle) {
        lv_timer_delete(error_hide_timer_handle);
        error_hide_timer_handle = nullptr;
    }

    error_hide_timer_handle = lv_timer_create(hide_error, 3000, error_box_obj);
    lv_obj_set_flag(error_box_obj, LV_OBJ_FLAG_HIDDEN, false);
    lv_label_set_text(error_text_obj, error.c_str());
}

bool CharacterFSM::invoke_action_sl(const std::string& action_id) {
    try {
        const auto& [_, action] = char_fsm.character_data.actions.at(action_id);

        if (const auto* switch_state = std::get_if<data::ActionSwitchState>(&action)) {
            switch_state_sl(switch_state->state_name);
        }

        return true;
    } catch (const std::out_of_range&) {
        return false;
    }
}

void CharacterFSM::mark_dirty() {
    ui_dirty = true;
}

CharacterFSM::BusyLock::BusyLock(CharacterFSM* parent) : still_holding(true), parent(parent) {
    CriticalGuard guard(&parent->spinlock);
    parent->busy = true;
}

void CharacterFSM::BusyLock::free() {
    if (still_holding) {
        still_holding = false;
        CriticalGuard guard(&parent->spinlock);
        parent->busy = false;
    }
}

CharacterFSM::BusyLock::~BusyLock() {
    free();
}

bool CharacterFSM::is_data_in_use() {
    CriticalGuard guard(&spinlock);
    return !(busy || state_is_cooking || in_tick);
}

bool CharacterFSM::is_free_sl() {
    CriticalGuard guard(&spinlock);
    return !(busy || state_is_cooking);
}

bool CharacterFSM::is_busy_sl() {
    CriticalGuard guard(&spinlock);
    return busy;
}

bool CharacterFSM::is_cooking_sl() {
    CriticalGuard guard(&spinlock);
    return state_is_cooking;
}

CharacterFSM::BusyLock CharacterFSM::get_busy_sl() {
    return BusyLock(this);
}

void CharacterFSM::wait_until_free_sl() {
    while (!is_free_sl()) {
        ESP_LOGI(TAG, "Waiting FSM to be free...");
        vTaskDelay(50 / portTICK_PERIOD_MS);
    }
}

void CharacterFSM::wait_until_data_unused_sl() {
    while (!is_data_in_use()) {
        ESP_LOGI(TAG, "Waiting for gap in FSM ticks...");
        vTaskDelay(50 / portTICK_PERIOD_MS);
    }
}

void CharacterFSM::wait_until_not_busy_sl() {
    while (is_busy_sl()) {
        vTaskDelay(50 / portTICK_PERIOD_MS);
    }
}

static void integer_upscale(const uint16_t* src, uint32_t* dst, const uint32_t columns, const uint32_t rows) {
    for (uint32_t row = 0; row < rows; row++) {
        const uint32_t src_start = row * columns;
        const uint32_t dst_start = row * 2 * columns;

        for (uint32_t col = 0; col < columns; col++) {
            uint32_t p = src[src_start + col];
            p |= p << 16;
            dst[dst_start + col] = p;
        }

        memcpy(dst + dst_start + columns, dst + dst_start, columns * sizeof(uint32_t));
    }
}

constexpr auto BYTES_PER_PIXEL = 2;
constexpr uint32_t ROWS_AT_A_TIME = 80;

void upload_to_screen(
    const uint16_t x, const uint16_t y,
    const uint32_t width, const uint32_t height,
    const uint16_t* image_data
) {
    const uint32_t divisions = height / ROWS_AT_A_TIME + (height % ROWS_AT_A_TIME > 0);
    for (uint32_t division = 0; division < divisions; division++) {
        const uint32_t current_row = ROWS_AT_A_TIME * division;
        const uint32_t rows_to_send = std::min(height - current_row, ROWS_AT_A_TIME);

        const uint32_t x_start = x, y_start = y + current_row, x_end = x + width, y_end =
                y + current_row + rows_to_send;

        ESP_ERROR_CHECK(
            esp_lcd_panel_draw_bitmap(
                bp_disp_lcd_panel,
                x_start, y_start,
                x_end, y_end,
                image_data)
        );

        image_data += width * rows_to_send;
    }
}

void CharacterFSM::play_animation(
    const data::StateAnimation& state_desc,
    const data::Animation& animation_desc
) const {
    uint32_t width = animation_desc.width, height = animation_desc.height;

    data::ImageDataVec upscaled_fb{};

    if (animation_desc.upscale) {
        width *= 2;
        height *= 2;

        upscaled_fb.resize(width * height * BYTES_PER_PIXEL);
    }

    LVGLLockGuard guard(0);

    if (animation_desc.clear_screen) {
        constexpr std::size_t PIXEL_COUNT = DISPLAY_WIDTH * ROWS_AT_A_TIME;
        data::ImageDataVec clear_fb{};
        clear_fb.resize(PIXEL_COUNT * BYTES_PER_PIXEL);

        auto* pixel_view = reinterpret_cast<uint16_t*>(clear_fb.data());

        // Set everything to background color
        for (std::size_t i = 0; i < PIXEL_COUNT; i++) {
            auto* ptr = pixel_view + i;
            *ptr = animation_desc.background_color;
        }

        uint16_t y = 0;
        while (y < DISPLAY_HEIGHT) {
            upload_to_screen(
                0,
                y,
                DISPLAY_WIDTH,
                std::min(ROWS_AT_A_TIME, static_cast<uint32_t>(DISPLAY_HEIGHT - y)),
                reinterpret_cast<const uint16_t*>(clear_fb.data())
            );
            y += ROWS_AT_A_TIME;
        }
    }

    FrameTimer timer{animation_desc.interval_us};

    if (state_desc.preload) {
        for (int repeat = 0; repeat < state_desc.loop_count; repeat++) {
            for (const auto& frame: preloaded_data.animation_frames.at(state_desc.name)) {
                timer.frame_start();

                uint8_t* frame_ptr = frame->data();
                if (animation_desc.upscale) {
                    integer_upscale(
                        reinterpret_cast<const uint16_t*>(frame->data()),
                        reinterpret_cast<uint32_t*>(upscaled_fb.data()),
                        animation_desc.width,
                        animation_desc.height
                    );
                    frame_ptr = upscaled_fb.data();
                }

                upload_to_screen(
                    animation_desc.x,
                    animation_desc.y,
                    width,
                    height,
                    reinterpret_cast<const uint16_t*>(frame_ptr)
                );

                timer.frame_end();
            }
        }
    } else {
        switch (animation_desc.mode) {
            case data::AnimationMode::FromSDCard: {
                const std::size_t IMAGE_FB_SIZE = animation_desc.width * animation_desc.height * BYTES_PER_PIXEL;

                data::ImageDataVec image_fb{};
                image_fb.resize(IMAGE_FB_SIZE);

                for (int repeat = 0; repeat < state_desc.loop_count; repeat++) {
                    for (uint32_t frame_index = 0; frame_index < animation_desc.frame_count; frame_index++) {
                        timer.frame_start();

                        // Load frame into memory
                        state_desc.load_frame(image_fb, frame_index + 1);

                        const auto* frame_buf = &image_fb;
                        if (animation_desc.upscale) {
                            integer_upscale(
                                reinterpret_cast<const uint16_t*>(image_fb.data()),
                                reinterpret_cast<uint32_t*>(upscaled_fb.data()),
                                animation_desc.width,
                                animation_desc.height
                            );
                            frame_buf = &upscaled_fb;
                        }

                        upload_to_screen(
                            animation_desc.x,
                            animation_desc.y,
                            width,
                            height,
                            reinterpret_cast<const uint16_t*>(frame_buf->data())
                        );

                        timer.frame_end();
                    }
                }

                break;
            }

            case data::AnimationMode::FromRAM: {
                for (int repeat = 0; repeat < state_desc.loop_count; repeat++) {
                    for (uint32_t frame_index = 0; frame_index < animation_desc.frame_count; frame_index++) {
                        timer.frame_start();

                        const uint8_t* frame_ptr = loaded_images[frame_index]->data();
                        if (animation_desc.upscale) {
                            integer_upscale(
                                reinterpret_cast<const uint16_t*>(loaded_images[frame_index]->data()),
                                reinterpret_cast<uint32_t*>(upscaled_fb.data()),
                                animation_desc.width,
                                animation_desc.height
                            );
                            frame_ptr = upscaled_fb.data();
                        }

                        upload_to_screen(
                            animation_desc.x,
                            animation_desc.y,
                            width,
                            height,
                            reinterpret_cast<const uint16_t*>(frame_ptr)
                        );

                        timer.frame_end();
                    }
                }

                break;
            }
        }
    }
}

void CharacterFSM::update_display(
    const image::SharedAllocatedImageData& image, const lv_image_dsc_t& desc, const bool upscale
) {
    LVGLLockGuard guard(0);
    current_image = image;
    current_descriptor = desc;

    lv_image_set_scale(image_obj, upscale ? 512 : 256);
    lv_image_set_src(image_obj, &current_descriptor);
    lv_obj_invalidate(lv_screen_active());
}

void CharacterFSM::set_ui_image(const data::StateImageVariant& variant) {
    auto busy_guard = get_busy_sl();
    ui_dirty = false;

    if (const auto* image_desc = std::get_if<data::StateImage>(&variant)) {
        if (image_desc->preload) {
            const auto& [dsc, ptr] = preloaded_data.image_data.at(image_desc->image_name);
            update_display(ptr, dsc, image_desc->upscale);
        } else {
            update_display(loaded_images[0], loaded_descriptors[0], image_desc->upscale);
        }
    } else if (const auto* anim_desc = std::get_if<data::StateAnimation>(&variant)) {
        const auto& animation = character_data.animations.at(anim_desc->name);

        play_animation(*anim_desc, animation);
        busy_guard.free();

        switch_state_unchecked(anim_desc->next_state);
    } else if (const auto* sequence_desc = std::get_if<data::StateSequence>(&variant)) {
        // Check if new frame is required
        if (esp_timer_get_time() > next_frame_time) {
            // Sequence is empty!
            if (sequence_desc->frames.empty()) {
                next_frame_time = INT64_MAX;
                return;
            }

            current_sequence_index = (current_sequence_index + 1) % sequence_desc->frames.size();

            const auto& frame = sequence_desc->frames.at(current_sequence_index);

            next_frame_time = esp_timer_get_time() + frame.duration_us;

            switch (sequence_desc->mode) {
                case data::SequenceLoadMode::Preload: {
                    const auto& [dsc, ptr] = preloaded_data.image_data.at(frame.image_name);

                    update_display(ptr, dsc, frame.upscale);

                    break;
                }

                case data::SequenceLoadMode::LoadAll: {
                    update_display(
                        loaded_images[current_sequence_index],
                        loaded_descriptors[current_sequence_index],
                        frame.upscale
                    );

                    break;
                }

                case data::SequenceLoadMode::LoadEach: {
                    const auto ready_frame_index = current_sequence_index % 2;
                    const auto offscreen_frame_index = (ready_frame_index + 1) % 2;

                    update_display(
                        loaded_images[ready_frame_index],
                        loaded_descriptors[ready_frame_index],
                        frame.upscale
                    );

                    ESP_LOGI(IMG_TAG, "Setting #%l to screen", current_sequence_index);

                    const auto next_sequence_index = (current_sequence_index + 1) % sequence_desc->frames.size();

                    const auto& offscreen_image = loaded_images[offscreen_frame_index];
                    const auto& seq_frame = sequence_desc->frames.at(next_sequence_index);

                    {
                        if (!seq_frame.image_exists(character_data)) {
                            return;
                        }

                        ESP_LOGI(TAG, "Writing frame into %x-%x for #%d", offscreen_image->start(), offscreen_image->end(), next_sequence_index);

                        seq_frame.load_image(character_data, offscreen_image->span());
                    }

                    loaded_descriptors[offscreen_frame_index] = data::make_image_dsc(
                        seq_frame.has_alpha, seq_frame.width, seq_frame.height, offscreen_image
                    );

                    break;
                }
            }
        }
    }
}

void CharacterFSM::tick() {
    if (!is_ready_sl()) return;

    {
        CriticalGuard guard(&spinlock);
        in_tick = true;
    }

    const auto now = esp_timer_get_time();
    const auto time_since_transition = now - last_transition_time;

    try {
        const auto [image, transitions] = get_current_state_sl();

        if (ui_dirty || (current_sequence_index != -1 && now > next_frame_time)) {
            set_ui_image(image);
        }

        update_cooking_progress_if_needed();
        address_queue();

        for (const auto& [next_state, trigger]: transitions) {
            if (const auto elapsed = std::get_if<data::StateTransitionElapsedTime>(&trigger)) {
                if (time_since_transition > elapsed->duration_us && !state_is_cooking) {
                    switch_state_sl(next_state);
                }
            }
        }
    } catch (std::out_of_range&) {
        ESP_LOGE(TAG, "no state!");
    }

    {
        CriticalGuard guard(&spinlock);
        in_tick = false;
    }
}

void fsm_task(void*) {
    TaskDeleteGuard task_guard{};
    ESP_LOGI(TAG, "FSM Task running!");

    character_names = data::list_characters();

    bp_characteristics->set_character_count(character_names);

    if (character_names.empty()) {
        ESP_LOGE(TAG, "There's no characters!");
    }

    auto selected_character_name = data::get_selected_character_name(character_names);
    if (!selected_character_name) {
        selected_character_name = character_names.front();
        data::select_character(character_names, *selected_character_name);
    }

    ESP_LOGI(TAG, "Loading '%s' character data...", selected_character_name->c_str());

    char_fsm.load_character_sl(*selected_character_name);

    ESP_LOGI(TAG, "Starting FSM...");

    char_fsm.create_ui();
    bp_characteristics->set_command_handler(bluetooth_command_handler);

    // ReSharper disable once CppDFAEndlessLoop
    while (char_fsm.alive) {
        vTaskDelay(TASK_INTERVAL);

        char_fsm.tick();
    }

    ESP_LOGI(TAG, "Returned from FSM Task");
}

void bp::start_fsm_task() {
    if (const portBASE_TYPE result = xTaskCreate(
            fsm_task,
            "CharFSM",
            TASK_STACK,
            nullptr,
            configMAX_PRIORITIES / 2,
            &fsm_task_handle
        );
        result != pdPASS) {
        ESP_LOGE(TAG, "Failed to start fsm task! %d", result);
    }
}
