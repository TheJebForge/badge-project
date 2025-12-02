#pragma once

#include "freertos/FreeRTOS.h"

class TaskDeleteGuard {
public:
    explicit TaskDeleteGuard() = default;
    ~TaskDeleteGuard() {
        vTaskDelete(nullptr);
    }
};
