#pragma once

#include "freertos/FreeRTOS.h"

class CriticalGuard {
    portMUX_TYPE* ptr;
public:
    explicit CriticalGuard(portMUX_TYPE* spinlock_ptr);
    ~CriticalGuard();
};
