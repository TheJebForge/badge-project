#include "error.hpp"

#include <lvgl.h>

#include "lvgl_lock.hpp"

static lv_obj_t* create_error_screen(const char* title, const char* subtitle) {
    lv_obj_t* screen = lv_obj_create(nullptr);
    lv_obj_remove_flag(screen, LV_OBJ_FLAG_SCROLLABLE);

    lv_obj_t* container = lv_obj_create(screen);
    lv_obj_set_layout(container, LV_LAYOUT_FLEX);
    lv_obj_set_flex_flow(container, LV_FLEX_FLOW_COLUMN);
    lv_obj_set_flex_align(container, LV_FLEX_ALIGN_CENTER, LV_FLEX_ALIGN_CENTER, LV_FLEX_ALIGN_CENTER);
    lv_obj_set_style_pad_row(container, 1, 0);
    lv_obj_align(container, LV_ALIGN_CENTER, 0, 0);
    lv_obj_set_width(container, lv_pct(100));
    lv_obj_set_style_border_width(container, 0, 0);

    lv_obj_t* title_obj = lv_label_create(container);
    lv_obj_set_style_text_font(title_obj, &lv_font_montserrat_24, 0);
    lv_label_set_text(title_obj, title);

    lv_obj_t* subtitle_obj = lv_label_create(container);
    lv_label_set_text(subtitle_obj, subtitle);

    return screen;
}

void bp::error_screen(const char* title, const char* subtitle) {
    LVGLLockGuard guard(0);
    lv_obj_t* screen = create_error_screen(title, subtitle);
    lv_screen_load_anim(screen, LV_SCR_LOAD_ANIM_NONE, 0, 0, true);
}

void bp::temporary_error_screen(const char* title, const char* subtitle, const uint32_t delay_ms) {
    LVGLLockGuard guard(0);
    lv_obj_t* old_screen = lv_screen_active();
    lv_obj_t* screen = create_error_screen(title, subtitle);
    lv_screen_load(screen);
    lv_screen_load_anim(old_screen, LV_SCR_LOAD_ANIM_FADE_IN, 500, delay_ms, true);
}
