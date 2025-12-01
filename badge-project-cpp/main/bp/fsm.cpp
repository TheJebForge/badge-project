#include "fsm.hpp"

#include "esp_lcd_panel_ops.h"
#include "esp_log.h"
#include "data/character.hpp"
#include "freertos/FreeRTOS.h"
#include "esp_timer.h"
#include "init/bluetooth.hpp"
#include "init/display.hpp"
#include "misc/lv_event_private.h"
#include "util/critical.hpp"
#include "util/frame_timer.hpp"
#include "util/lvgl_lock.hpp"
#include "init/bluetooth.hpp"

constexpr auto TAG = "char_fsm";
constexpr uint32_t TASK_STACK = 0x2000;
constexpr UBaseType_t TASK_PRIORITY = configMAX_PRIORITIES / 2;
constexpr TickType_t TASK_INTERVAL = 100 / portTICK_PERIOD_MS;

static TaskHandle_t fsm_task_handle;

class CharacterFSM {
    bool ready = false;
    bool busy = false;

    bp::data::Character character_data;
    bp::data::PreloadedData preloaded_data;

    std::string current_state;
    int64_t last_transition_time = 0;
    std::size_t current_sequence_index = -1;
    int64_t next_frame_time = 0;

    portMUX_TYPE spinlock = portMUX_INITIALIZER_UNLOCKED;

    lv_img_dsc_t allocated_image_dsc{};
    bp::data::ImageDataVec allocated_image;

    lv_obj_t* screen_obj = nullptr;
    lv_obj_t* char_name_obj = nullptr;
    lv_obj_t* image_obj = nullptr;
    bool ui_dirty = false;

    class BusyLock {
        CharacterFSM* parent;

    public:
        explicit BusyLock(CharacterFSM* parent);
        ~BusyLock();
    };

    bool is_busy_sl();
    BusyLock get_busy_sl();
    void wait_until_free_sl();
    void play_animation(
        const bp::data::StateAnimation& state_desc, const bp::data::Animation& animation_desc
    ) const;
    void set_ui_image(const bp::data::StateImageVariant& variant);

public:
    void create_ui();
    bool is_ready_sl();
    void load_character_sl(const std::string& name);
    bp::data::State get_current_state_sl();
    void switch_state_sl(const std::string& next_state);
    bool invoke_action_sl(const std::string& action_id);
    void mark_dirty();
    void tick();

    friend bp::ClientCommandResponse bluetooth_command_handler(uint8_t, std::span<char, 200>);
};

static CharacterFSM char_fsm;

enum class ClientCommandType : uint8_t {
    GetAction,
    GetActionDisplayName,
    InvokeAction
};

bp::ClientCommandResponse bluetooth_command_handler(
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
        if (std::holds_alternative<bp::data::StateTransitionClicked>(trigger)) {
            instance->switch_state_sl(next_state);
        }
    }
}

void CharacterFSM::create_ui() {
    {
        LVGLLockGuard guard(0);
        screen_obj = lv_obj_create(nullptr);

        lv_obj_t* header = lv_obj_create(screen_obj);
        lv_obj_set_size(header, bp::DISPLAY_WIDTH, bp::DISPLAY_HEIGHT - bp::DISPLAY_WIDTH);
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
        lv_obj_set_pos(image_button, 0, bp::DISPLAY_HEIGHT - bp::DISPLAY_WIDTH);
        lv_obj_set_size(image_button, bp::DISPLAY_WIDTH, bp::DISPLAY_WIDTH);
        lv_obj_add_event_cb(image_button, image_clicked, LV_EVENT_CLICKED, this);

        image_obj = lv_image_create(image_button);
        lv_obj_set_size(image_obj, bp::DISPLAY_WIDTH, bp::DISPLAY_WIDTH);
        lv_obj_center(image_obj);
        lv_image_set_antialias(image_obj, false);

        lv_screen_load(screen_obj);
    }

    mark_dirty();
}

bool CharacterFSM::is_ready_sl() {
    CriticalGuard guard(&spinlock);
    return ready;
}

void CharacterFSM::load_character_sl(const std::string& name) {
    wait_until_free_sl();

    {
        CriticalGuard guard(&spinlock);
        ready = false;
    }

    bp::data::load_character_data(character_data, name);
    bp::data::preload_data(preloaded_data, character_data);

    bp_characteristics->set_character_info(
        character_data.name,
        character_data.species,
        character_data.actions.size()
    );

    switch_state_sl(character_data.default_state);

    {
        CriticalGuard guard(&spinlock);
        ready = true;
    }
}

bp::data::State CharacterFSM::get_current_state_sl() {
    CriticalGuard guard{&spinlock};
    return character_data.states.at(current_state);
}

void CharacterFSM::switch_state_sl(const std::string& next_state) {
    ESP_LOGI(TAG, "Switching to '%s' state", next_state.c_str());

    {
        const int64_t now = esp_timer_get_time();
        CriticalGuard guard(&spinlock);
        current_state = next_state;
        last_transition_time = now;
        current_sequence_index = -1;
        next_frame_time = 0;
    }

    mark_dirty();
}

bool CharacterFSM::invoke_action_sl(const std::string& action_id) {
    try {
        const auto& [_, action] = char_fsm.character_data.actions.at(action_id);

        if (const auto* switch_state = std::get_if<bp::data::ActionSwitchState>(&action)) {
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

CharacterFSM::BusyLock::BusyLock(CharacterFSM* parent) : parent(parent) {
    CriticalGuard guard(&parent->spinlock);
    parent->busy = true;
}

CharacterFSM::BusyLock::~BusyLock() {
    CriticalGuard guard(&parent->spinlock);
    parent->busy = false;
}

bool CharacterFSM::is_busy_sl() {
    CriticalGuard guard(&spinlock);
    return busy;
}

CharacterFSM::BusyLock CharacterFSM::get_busy_sl() {
    return BusyLock(this);
}

void CharacterFSM::wait_until_free_sl() {
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
    const bp::data::StateAnimation& state_desc,
    const bp::data::Animation& animation_desc
) const {
    uint32_t width = animation_desc.width, height = animation_desc.height;

    bp::data::ImageDataVec upscaled_fb{};

    if (animation_desc.upscale) {
        width *= 2;
        height *= 2;

        upscaled_fb.resize(width * height * BYTES_PER_PIXEL);
    }

    FrameTimer timer{animation_desc.interval_us};

    if (state_desc.preload) {
        LVGLLockGuard guard(0);

        for (int repeat = 0; repeat < state_desc.loop_count; repeat++) {
            for (const auto& frame: preloaded_data.animation_frames.at(state_desc.name)) {
                timer.frame_start();

                auto* frame_buf = &frame;
                if (animation_desc.upscale) {
                    integer_upscale(
                        reinterpret_cast<const uint16_t*>(frame.data()),
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
    } else {
        // TODO: Implement other animation modes!
    }
}

void CharacterFSM::set_ui_image(const bp::data::StateImageVariant& variant) {
    ui_dirty = false;
    auto busy_guard = get_busy_sl();

    if (const auto* image_desc = std::get_if<bp::data::StateImage>(&variant)) {
        LVGLLockGuard guard(0);

        lv_image_set_scale(image_obj, image_desc->upscale ? 512 : 256);

        if (image_desc->preload) {
            const auto& [dsc, _] = preloaded_data.image_data.at(image_desc->image_name);
            lv_image_set_src(image_obj, &dsc);
        }
        // TODO: Implement dynamic loading (non-preloaded data)
    } else if (const auto* anim_desc = std::get_if<bp::data::StateAnimation>(&variant)) {
        const auto& animation = character_data.animations.at(anim_desc->name);

        play_animation(*anim_desc, animation);

        switch_state_sl(anim_desc->next_state);
    } else if (const auto* sequence_desc = std::get_if<bp::data::StateSequence>(&variant)) {
        // Check if new frame is required
        if (esp_timer_get_time() > next_frame_time) {
            // Sequence is empty!
            if (sequence_desc->frames.empty()) {
                next_frame_time = INT64_MAX;
                return;
            }

            current_sequence_index++;
            if (current_sequence_index >= sequence_desc->frames.size()) current_sequence_index = 0;

            const auto& frame = sequence_desc->frames.at(current_sequence_index);

            LVGLLockGuard guard(0);
            lv_image_set_scale(image_obj, frame.upscale ? 512 : 256);

            if (sequence_desc->mode == bp::data::SequenceLoadMode::Preload) {
                const auto& [dsc, _] = preloaded_data.image_data.at(frame.image_name);
                lv_image_set_src(image_obj, &dsc);
            }
            // TODO: Dynamic loading

            next_frame_time = esp_timer_get_time() + frame.duration_us;
        }
    }
}

void CharacterFSM::tick() {
    if (!is_ready_sl()) return;

    const auto now = esp_timer_get_time();
    const auto time_since_transition = now - last_transition_time;

    const auto [image, transitions] = get_current_state_sl();

    if (ui_dirty || (current_sequence_index != -1 && now > next_frame_time)) {
        set_ui_image(image);
    }

    for (const auto& [next_state, trigger]: transitions) {
        if (const auto elapsed = std::get_if<bp::data::StateTransitionElapsedTime>(&trigger)) {
            if (time_since_transition > elapsed->duration_us) {
                switch_state_sl(next_state);
            }
        }
    }
}

void fsm_task_code() {
    const auto characters = bp::data::list_characters();

    if (characters.empty()) {
        ESP_LOGE(TAG, "There's no characters!");
    }

    auto selected_character_name = bp::data::get_selected_character_name(characters);
    if (!selected_character_name) {
        selected_character_name = characters.front();
        bp::data::select_character(characters, *selected_character_name);
    }

    ESP_LOGI(TAG, "Loading '%s' character data...", selected_character_name->c_str());

    heap_caps_print_heap_info(MALLOC_CAP_DEFAULT);

    char_fsm.load_character_sl(*selected_character_name);

    ESP_LOGI(TAG, "Starting FSM...");

    char_fsm.create_ui();
    bp_characteristics->set_command_handler(bluetooth_command_handler);

    heap_caps_print_heap_info(MALLOC_CAP_DEFAULT);

    // ReSharper disable once CppDFAEndlessLoop
    while (true) {
        vTaskDelay(TASK_INTERVAL);

        char_fsm.tick();
    }
}


void fsm_task(void*) {
    ESP_LOGI(TAG, "FSM Task running!");

    fsm_task_code();

    ESP_LOGI(TAG, "Returned from FSM Task");
    vTaskDelete(fsm_task_handle);
}

void bp::start_fsm_task() {
    if (const portBASE_TYPE result = xTaskCreate(
            fsm_task,
            "CharFSM",
            0x1000,
            nullptr,
            configMAX_PRIORITIES / 2,
            &fsm_task_handle
        );
        result != pdPASS) {
        ESP_LOGE(TAG, "Failed to start fsm task! %d", result);
    }
}
