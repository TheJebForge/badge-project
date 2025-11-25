#pragma once

#include "esp_lcd_touch.h"
#include "esp_lcd_types.h"
#include "soc/gpio_num.h"
#include "lvgl.h"

extern esp_lcd_panel_io_handle_t bp_disp_lcd_io;
extern esp_lcd_panel_handle_t bp_disp_lcd_panel;
extern esp_lcd_touch_handle_t bp_disp_touch_handle;

extern lv_display_t* bp_lvgl_disp;
extern lv_indev_t* bp_lvgl_touch_indev;

namespace bp {
    constexpr uint16_t DISPLAY_WIDTH = 320;
    constexpr uint16_t DISPLAY_HEIGHT = 480;
    constexpr gpio_num_t DISPLAY_GPIO_BACKLIGHT = GPIO_NUM_13;
    constexpr bool DISPLAY_DEFAULT_BACKLIGHT = true;

    esp_err_t init_display();

    esp_err_t init_touchscreen();

    esp_err_t init_lvgl();
}