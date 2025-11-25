#pragma once
#include "sd_protocol_types.h"

extern sdmmc_card_t* bp_sdcard_ptr;

namespace bp {
    constexpr auto SDCARD_MOUNT_POINT = "/sdcard";

    esp_err_t init_sdcard();
    void sdcard_fail_screen(esp_err_t error);
}