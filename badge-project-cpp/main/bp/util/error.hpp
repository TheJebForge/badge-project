#pragma once
#include <cstdint>

namespace bp {
    void error_screen(const char* title, const char* subtitle);
    void temporary_error_screen(const char* title, const char* subtitle, uint32_t delay_ms);
}
