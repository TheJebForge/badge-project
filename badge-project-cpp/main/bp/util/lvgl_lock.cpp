#include "lvgl_lock.hpp"

#include "esp_lvgl_port.h"

LVGLLockGuard::LVGLLockGuard(const uint32_t timeout) {
    lvgl_port_lock(timeout);
}

LVGLLockGuard::~LVGLLockGuard() {
    lvgl_port_unlock();
}
