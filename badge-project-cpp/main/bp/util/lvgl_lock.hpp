#pragma once
#include <cstdint>

class LVGLLockGuard {
public:
    explicit LVGLLockGuard(uint32_t timeout);

    ~LVGLLockGuard();
};
