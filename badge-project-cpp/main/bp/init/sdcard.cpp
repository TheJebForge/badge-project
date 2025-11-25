#include "sdcard.hpp"

#include "esp_vfs_fat.h"
#include "bp/util/error.hpp"
#include "driver/sdmmc_default_configs.h"
#include "driver/sdmmc_host.h"

constexpr int SDCARD_FREQ_KHZ = 40000;
constexpr uint8_t SDCARD_PIN_WIDTH = 4; // How many pins to use
constexpr gpio_num_t SDCARD_GPIO_SCLK = GPIO_NUM_14;
constexpr gpio_num_t SDCARD_GPIO_CMD = GPIO_NUM_17;
constexpr gpio_num_t SDCARD_GPIO_D0 = GPIO_NUM_16;
constexpr gpio_num_t SDCARD_GPIO_D1 = GPIO_NUM_18;
constexpr gpio_num_t SDCARD_GPIO_D2 = GPIO_NUM_15;
constexpr gpio_num_t SDCARD_GPIO_D3 = GPIO_NUM_21;

static auto TAG = "sdcard_init";

sdmmc_card_t* bp_sdcard_ptr = nullptr;
esp_err_t bp::init_sdcard() {
    esp_err_t ret = ESP_OK;

    esp_vfs_fat_sdmmc_mount_config_t mount_config {
        .format_if_mount_failed = false,
        .max_files = 5,
        .allocation_unit_size = 64 * 1024,
    };

    sdmmc_card_t* card;

    sdmmc_host_t host = SDMMC_HOST_DEFAULT();
    host.max_freq_khz = SDCARD_FREQ_KHZ;

    sdmmc_slot_config_t slot_config = SDMMC_SLOT_CONFIG_DEFAULT();
    slot_config.width = SDCARD_PIN_WIDTH;

    slot_config.clk = SDCARD_GPIO_SCLK;
    slot_config.cmd = SDCARD_GPIO_CMD;
    slot_config.d0 = SDCARD_GPIO_D0;
    slot_config.d1 = SDCARD_GPIO_D1;
    slot_config.d2 = SDCARD_GPIO_D2;
    slot_config.d3 = SDCARD_GPIO_D3;

    slot_config.flags |= SDMMC_SLOT_FLAG_INTERNAL_PULLUP;

    ESP_LOGI(TAG, "Attempting to mount the SD Card");
    ret = esp_vfs_fat_sdmmc_mount(SDCARD_MOUNT_POINT, &host, &slot_config, &mount_config, &card);

    if (ret != ESP_OK) {
        if (ret == ESP_FAIL) {
            ESP_LOGE(TAG, "Failed to mount filesystem. "
                     "If you want the card to be formatted, set the EXAMPLE_FORMAT_IF_MOUNT_FAILED menuconfig option.");
        } else {
            ESP_LOGE(TAG, "Failed to initialize the card (%s). "
                     "Make sure SD card lines have pull-up resistors in place.", esp_err_to_name(ret));
        }
        return ret;
    }
    ESP_LOGI(TAG, "Filesystem mounted");

    bp_sdcard_ptr = card;

    return ret;
}

void bp::sdcard_fail_screen(const esp_err_t error) {
    error_screen("Failed to mount SDCard", esp_err_to_name(error));
}
