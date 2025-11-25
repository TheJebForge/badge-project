#include "frame_timer.hpp"

#include <algorithm>

#include "esp_log.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h"


FrameTimer::FrameTimer(const int64_t interval) {
    frame_interval = interval;
    last_wait_time = esp_timer_get_time();
    frame_start_time = last_wait_time;
}

void FrameTimer::frame_start() {
    frame_start_time = esp_timer_get_time();
}

void FrameTimer::frame_end() {
    const auto now = esp_timer_get_time();

    if (const auto time_to_wait = frame_interval - (now - frame_start_time);
        time_to_wait > 0 || last_wait_time > 2000000) {
        last_wait_time = now;
        vTaskDelay(std::max(time_to_wait / 1000, 30LL) / portTICK_PERIOD_MS);
    }
}

