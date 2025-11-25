#pragma once
#include <cstdint>


class FrameTimer {
    int64_t last_wait_time;
    int64_t frame_interval;
    int64_t frame_start_time;
public:
    explicit FrameTimer(int64_t interval);
    void frame_start();
    void frame_end();
};
