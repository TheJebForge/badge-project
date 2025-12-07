#include "esp_err.h"
#include "esp_log.h"
#include "esp_lvgl_port.h"
#include "bp/fsm.hpp"
#include "bp/init/bluetooth.hpp"
#include "bp/data/character.hpp"
#include "bp/init/display.hpp"
#include "bp/init/sdcard.hpp"
#include "bp/util/lvgl_lock.hpp"
#include "misc/lv_event_private.h"

constexpr auto TAG = "app_main";

static std::optional<NimBLEConnInfo> current_request = std::nullopt;
static lv_obj_t* current_request_dialog = nullptr;
static bool YES = true;
static bool NO = false;

// ReSharper disable once CppParameterMayBeConstPtrOrRef
void process_pair_request(lv_event_t* event) {
    if (event->code == LV_EVENT_CLICKED) {
        if (!current_request.has_value()) return;

        const bool confirm = *static_cast<bool*>(event->user_data);
        bp::respond_pair(current_request.value(), confirm);

        if (current_request_dialog) {
            lv_obj_delete(current_request_dialog);
            current_request_dialog = nullptr;
        }
    }
}

void bluetooth_pair_request(NimBLEConnInfo info, uint32_t pin) {
    current_request = info;

    LVGLLockGuard guard(0);

    current_request_dialog = lv_obj_create(lv_screen_active());
    lv_obj_set_layout(current_request_dialog, LV_LAYOUT_FLEX);
    lv_obj_set_flex_flow(current_request_dialog, LV_FLEX_FLOW_COLUMN);
    lv_obj_set_flex_align(current_request_dialog, LV_FLEX_ALIGN_CENTER, LV_FLEX_ALIGN_CENTER, LV_FLEX_ALIGN_CENTER);
    lv_obj_set_style_pad_row(current_request_dialog, 1, 0);
    lv_obj_align(current_request_dialog, LV_ALIGN_CENTER, 0, 0);
    lv_obj_set_width(current_request_dialog, lv_pct(100));
    lv_obj_set_style_border_width(current_request_dialog, 0, 0);
    lv_obj_set_style_bg_color(current_request_dialog, lv_color_hex(0x444444), 0);

    lv_obj_t* title_obj = lv_label_create(current_request_dialog);
    lv_obj_set_style_text_font(title_obj, &lv_font_montserrat_24, 0);
    lv_label_set_text(title_obj, "Pair Request");

    lv_obj_t* subtitle_obj = lv_label_create(current_request_dialog);
    lv_label_set_text(subtitle_obj, std::format("Pin: {}", pin).c_str());

    lv_obj_t* row = lv_obj_create(current_request_dialog);
    lv_obj_set_layout(row, LV_LAYOUT_FLEX);
    lv_obj_set_flex_flow(row, LV_FLEX_FLOW_ROW);
    lv_obj_set_flex_align(row, LV_FLEX_ALIGN_CENTER, LV_FLEX_ALIGN_CENTER, LV_FLEX_ALIGN_CENTER);
    lv_obj_set_style_pad_column(row, 5, 0);
    lv_obj_set_height(row, 40);
    lv_obj_set_width(row, lv_pct(100));
    lv_obj_set_style_border_width(row, 0, 0);
    lv_obj_set_style_bg_color(row, lv_color_hex(0x444444), 0);

    lv_obj_t* no_button = lv_button_create(row);
    lv_obj_add_event_cb(no_button, process_pair_request, LV_EVENT_CLICKED, &NO);

    lv_obj_t* no_button_label = lv_label_create(no_button);
    lv_obj_center(no_button_label);
    lv_label_set_text(no_button_label, "Cancel");

    lv_obj_t* yes_button = lv_button_create(row);
    lv_obj_add_event_cb(yes_button, process_pair_request, LV_EVENT_CLICKED, &YES);

    lv_obj_t* yes_button_label = lv_label_create(yes_button);
    lv_obj_center(yes_button_label);
    lv_label_set_text(yes_button_label, "Confirm");
}

extern "C" void app_main(void) {
    ESP_ERROR_CHECK(bp::init_display());
    ESP_ERROR_CHECK(bp::init_touchscreen());
    ESP_ERROR_CHECK(bp::init_lvgl());

    LVGLLockGuard guard(0);

    lv_display_set_theme(nullptr, lv_theme_mono_init(nullptr, true, &lv_font_montserrat_16));
    lv_screen_load_anim(lv_obj_create(nullptr), LV_SCR_LOAD_ANIM_NONE, 0, 0, true);

    if (esp_err_t sdcard_error; (sdcard_error = bp::init_sdcard()) != ESP_OK) {
        bp::sdcard_fail_screen(sdcard_error);
        return;
    }

    bp::init_bluetooth(bluetooth_pair_request);

    // lv_obj_t* container = lv_obj_create(lv_screen_active());
    // lv_obj_set_layout(container, LV_LAYOUT_FLEX);
    // lv_obj_set_flex_flow(container, LV_FLEX_FLOW_COLUMN);
    // lv_obj_set_flex_align(container, LV_FLEX_ALIGN_CENTER, LV_FLEX_ALIGN_CENTER, LV_FLEX_ALIGN_CENTER);
    // lv_obj_set_style_pad_row(container, 1, 0);
    // lv_obj_align(container, LV_ALIGN_CENTER, 0, 0);
    // lv_obj_set_width(container, lv_pct(100));
    // lv_obj_set_style_border_width(container, 0, 0);

    // for (const auto& char_name : bp::data::list_characters()) {
    //     ESP_LOGI(TAG, "found: %s", char_name.c_str());
    //     const auto character = bp::data::load_character_data(char_name);
    //     ESP_LOGI(TAG, "character name: %s, species: %s, default state: %s",
    //         character.name.c_str(), character.species.c_str(), character.default_state.c_str());
    //
    //     for (const auto& [name, state] : character.states) {
    //         ESP_LOGI(TAG, "has state: %s", name.c_str());
    //
    //         if (std::holds_alternative<bp::data::StateImage>(state.image)) {
    //             auto& img = std::get<bp::data::StateImage>(state.image);
    //             ESP_LOGI(TAG, "image state! name: %s, width %d, height %d", img.image_name.c_str(), img.width, img.height);
    //         } else if (std::holds_alternative<bp::data::StateAnimation>(state.image)) {
    //             auto& anim = std::get<bp::data::StateAnimation>(state.image);
    //             ESP_LOGI(TAG, "anim state! name: %s, next state %s", anim.name.c_str(), anim.next_state.c_str());
    //         }
    //
    //         for (const auto& [next_state, _] : state.transitions) {
    //             ESP_LOGI(TAG, "can transition to %s", next_state.c_str());
    //         }
    //     }
    //
    //     for (const auto& [name, anim] : character.animations) {
    //         ESP_LOGI(TAG, "has animation: %s", name.c_str());
    //     }
    //
    //     for (const auto& [name, action] : character.actions) {
    //         ESP_LOGI(TAG, "has action: %s with display '%s'", name.c_str(), action.display.c_str());
    //     }
    //
    //     try {
    //         const auto preloaded = bp::data::preload_data(character);
    //
    //         for (const auto& [image_name, _] : preloaded.image_data) {
    //             ESP_LOGI(TAG, "image preloaded: %s", image_name.c_str());
    //         }
    //
    //         for (const auto& [anim_name, frames] : preloaded.animation_frames) {
    //             ESP_LOGI(TAG, "animation preloaded: %s, with %d frames", anim_name.c_str(), frames.size());
    //         }
    //
    //         heap_caps_print_heap_info(MALLOC_CAP_DEFAULT);
    //         heap_caps_print_heap_info(MALLOC_CAP_SPIRAM);
    //     } catch (bp::data::data_exception& exception) {
    //         ESP_LOGI(TAG, "caught data exception! %s", exception.what());
    //     } catch (std::exception& exception) {
    //         ESP_LOGI(TAG, "caught other exception! %s", exception.what());
    //     }
    // }
    //
    // lv_obj_t* label = lv_label_create(lv_screen_active());
    // lv_label_set_text(label, "kekw");
    // lv_obj_align(label, LV_ALIGN_CENTER, 0, 0);

    heap_caps_print_heap_info(MALLOC_CAP_DEFAULT);
    bp::start_fsm_task();
}

// old code
//
//     lvgl_port_lock(0);
//
// #define IMAGE_FILE_SIZE (0xC800)
//     static u_int16_t* animation_data;
//     animation_data = malloc(IMAGE_FILE_SIZE);
//
//     static u_int32_t* upscaled_data;
//     upscaled_data = malloc(0x32000);
//
//     for (int i = 1; i <= 405; i++) {
//         const int64_t time_start = esp_timer_get_time();
//
//         char filename[64];
//         sprintf(filename, "/sdcard/out%d.png.bin", i);
//
//         int fd = open(filename, O_RDONLY, 0);
//         if (fd < 0) {
//             ESP_LOGE(TAG, "Failed to find image file");
//             return;
//         }
//
//         const size_t len = read(fd, animation_data, IMAGE_FILE_SIZE);
//         if (len < IMAGE_FILE_SIZE) {
//             ESP_LOGE(TAG, "Couldn't read all bytes from image file");
//             return;
//         }
//         close(fd);
//
//         upscale(animation_data, upscaled_data, 160, 160);
//
//         ESP_ERROR_CHECK(esp_lcd_panel_draw_bitmap(bp_disp_lcd_panel, 0, 0, 320, 80, upscaled_data));
//         ESP_ERROR_CHECK(esp_lcd_panel_draw_bitmap(bp_disp_lcd_panel, 0, 80, 320, 160, upscaled_data + 160 * 80));
//         ESP_ERROR_CHECK(esp_lcd_panel_draw_bitmap(bp_disp_lcd_panel, 0, 160, 320, 240, upscaled_data + 160 * 160));
//         ESP_ERROR_CHECK(esp_lcd_panel_draw_bitmap(bp_disp_lcd_panel, 0, 240, 320, 320, upscaled_data + 160 * 240));
//
//         const int64_t time_took = esp_timer_get_time() - time_start;
//         const int64_t time_to_wait = 103000 - time_took;
//
//         if (time_to_wait > 0) {
//             vTaskDelay(time_to_wait / 1000 / portTICK_PERIOD_MS);
//         }
//     }
//     lvgl_port_unlock();
//
//     lv_label_create(0);
//     lv_obj_invalidate(lv_screen_active());
//     lvgl_port_unlock();

// int64_t time_since_wait = esp_system_get_time();
// DIR* dirp = opendir("/sdcard");
//
// struct dirent* dp;
// while (dirp) {
//     if (esp_system_get_time() - time_since_wait > 50000) {
//         ESP_LOGI(TAG, "waiting");
//         vTaskDelay(10 / portTICK_PERIOD_MS);
//         time_since_wait = esp_system_get_time();
//     }
//
//     if ((dp = readdir(dirp)) != NULL) {
//         ESP_LOGI(TAG, "/sdcard/%s", dp->d_name);
//     } else {
//         closedir(dirp);
//         dirp = NULL;
//     }
// }

// printf("hello\nimma buzz\n");
//
// const ledc_timer_config_t buzzer_timer = {
//     .speed_mode = LEDC_LOW_SPEED_MODE,
//     .duty_resolution = 9,
//     .timer_num = LEDC_TIMER_0,
//     .freq_hz = 440,
//     .clk_cfg = LEDC_USE_XTAL_CLK
// };
// ESP_ERROR_CHECK(ledc_timer_config(&buzzer_timer));
//
// const ledc_channel_config_t buzzer_channel = {
//     .speed_mode = LEDC_LOW_SPEED_MODE,
//     .channel = LEDC_CHANNEL_0,
//     .timer_sel = LEDC_TIMER_0,
//     .intr_type = LEDC_INTR_DISABLE,
//     .gpio_num = 38,
//     .duty = 0,
//     .hpoint = 0,
// };
// ESP_ERROR_CHECK(ledc_channel_config(&buzzer_channel));
//
// ESP_ERROR_CHECK(ledc_fade_func_install(ESP_INTR_FLAG_INTRDISABLED));
//
//
// for (int i = 0; i < NOTES; i++) {
//     ledc_set_freq(LEDC_LOW_SPEED_MODE, LEDC_TIMER_0, freq[i]);
//     ledc_set_duty_and_update(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_0, 2, 0);
//     vTaskDelay(MAX(len[i], 30) / portTICK_PERIOD_MS);
//     ledc_set_duty_and_update(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_0, 0, 0);
//     vTaskDelay(MAX(delay[i], 30) / portTICK_PERIOD_MS);
//
//     // ledc_set_duty_and_update(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_0, i, 0);
//     // vTaskDelay(20 / portTICK_PERIOD_MS);
// }
//
// printf("Restarting now.\n");
// fflush(stdout);
// esp_restart();