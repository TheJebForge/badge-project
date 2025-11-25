#include "display.hpp"

#include "esp_check.h"
#include "esp_lcd_panel_ops.h"
#include "esp_lcd_st7796.h"
#include "driver/i2c.h"
#include "esp_lcd_touch_cst816s.h"
#include "esp_lvgl_port.h"
#include "driver/i2c_master.h"

constexpr gpio_num_t DISPLAY_GPIO_IM0 = GPIO_NUM_47;
constexpr gpio_num_t DISPLAY_GPIO_IM1 = GPIO_NUM_48;

constexpr gpio_num_t DISPLAY_GPIO_SCLK = GPIO_NUM_40;
constexpr gpio_num_t DISPLAY_GPIO_MOSI = GPIO_NUM_45;
constexpr gpio_num_t DISPLAY_GPIO_RST = GPIO_NUM_NC;
constexpr gpio_num_t DISPLAY_GPIO_DC = GPIO_NUM_41;
constexpr gpio_num_t DISPLAY_GPIO_CS = GPIO_NUM_42;

constexpr i2c_port_t TOUCH_I2C_NUM = I2C_NUM_0;
constexpr uint32_t TOUCH_I2C_CLK_HZ = 400000;
constexpr gpio_num_t TOUCH_I2C_SCL = GPIO_NUM_3;
constexpr gpio_num_t TOUCH_I2C_SDA = GPIO_NUM_1;
constexpr gpio_num_t TOUCH_GPIO_INT = GPIO_NUM_4;

constexpr spi_host_device_t DISPLAY_SPI_NUM = SPI3_HOST;
constexpr uint32_t DISPLAY_PIXEL_CLK_HZ = 80 * 1000 * 1000;
constexpr uint8_t DISPLAY_CMD_BITS = 8;
constexpr uint8_t DISPLAY_PARAM_BITS = 8;
constexpr esp_lcd_color_space_t DISPLAY_COLOR_SPACE = LCD_RGB_ELEMENT_ORDER_BGR;
constexpr uint32_t DISPLAY_BITS_PER_PIXEL = 16;
constexpr uint32_t DISPLAY_DRAW_BUFF_HEIGHT = 180;
constexpr bool DISPLAY_DRAW_BUFF_DOUBLE = false;

static auto TAG = "display_init";

/* LCD IO and panel */
esp_lcd_panel_io_handle_t bp_disp_lcd_io = nullptr;
esp_lcd_panel_handle_t bp_disp_lcd_panel = nullptr;
esp_lcd_touch_handle_t bp_disp_touch_handle = nullptr;

/* LVGL display and touch */
lv_display_t* bp_lvgl_disp = nullptr;
lv_indev_t* bp_lvgl_touch_indev = nullptr;

static const st7796_lcd_init_cmd_t lcd_init_cmds[] = {
    // {cmd, { data }, data_size, delay_ms}
    {0x11, (uint8_t []){0x00}, 0, 120}, // 0x11 command, no data, 120ms delay

    {0xF0, (uint8_t []){0xC3}, 1, 0}, // 0xF0 command with data 0xC3
    {0xF0, (uint8_t []){0x96}, 1, 0}, // 0xF0 command with data 0x96

    {0x36, (uint8_t []){0x28}, 1, 0}, // 0x36 command with data 0x28
    {0x3A, (uint8_t []){0x55}, 1, 0}, // 0x3A command with data 0x55

    {0xB4, (uint8_t []){0x01}, 1, 0}, // 0xB4 command with data 0x01 (1-dot Inversion)
    {0xB7, (uint8_t []){0xC6}, 1, 0}, // 0xB7 command with data 0xC6

    {0xC0, (uint8_t []){0x80, 0x04}, 2, 0}, // 0xC0 command with data 0x80, 0x04
    {0xC1, (uint8_t []){0x13}, 1, 0}, // 0xC1 command with data 0x13 (VOP=4.5V)
    {0xC5, (uint8_t []){0xA7}, 1, 0}, // 0xC2 command with data 0xA7
    {0xC5, (uint8_t []){0x16}, 1, 0}, // 0xC5 command with data 0x16

    {0xE8, (uint8_t []){0x40, 0x8a, 0x00, 0x00, 0x29, 0x19, 0xA5, 0x33}, 8, 0}, // 0xE8 command with 8 data bytes

    {0xE0, (uint8_t []){0xF0, 0x19, 0x20, 0x10, 0x11, 0x0A, 0x46, 0x44, 0x57, 0x09, 0x1A, 0x1B, 0x2A, 0x2D}, 14, 0},
    // 0xE0 command with 14 data bytes

    {0xE1, (uint8_t []){0xF0, 0x12, 0x1A, 0x0A, 0x0C, 0x18, 0x45, 0x44, 0x56, 0x3F, 0x15, 0x11, 0x24, 0x26}, 14, 0},
    // 0xE1 command with 14 data bytes

    {0xF0, (uint8_t []){0x3C}, 1, 0}, // 0xF0 command with data 0x3C
    {0xF0, (uint8_t []){0x69}, 1, 0}, // 0xF0 command with data 0x69

    {0x21, (uint8_t []){0x00}, 0, 0}, // 0x21 command, no data
    {0x29, (uint8_t []){0x00}, 0, 50}, // 0x29 command, no data, 50ms delay

    {0x2C, (uint8_t []){0x00}, 0, 0},
};

esp_err_t bp::init_display() {
    esp_err_t ret = ESP_OK;

    {
        constexpr gpio_config_t mode_backlight_config{
            .pin_bit_mask = 1ULL << DISPLAY_GPIO_BACKLIGHT
                            | 1ULL << DISPLAY_GPIO_IM0
                            | 1ULL << DISPLAY_GPIO_IM1,
            .mode = GPIO_MODE_OUTPUT
        };
        ESP_ERROR_CHECK(gpio_config(&mode_backlight_config));

        // Set display to SPI mode
        ESP_ERROR_CHECK(gpio_set_level(DISPLAY_GPIO_IM0, true));
        ESP_ERROR_CHECK(gpio_set_level(DISPLAY_GPIO_IM1, true));

        // Set backlight to default state
        ESP_ERROR_CHECK(gpio_set_level(DISPLAY_GPIO_BACKLIGHT, DISPLAY_DEFAULT_BACKLIGHT));

        ESP_LOGD(TAG, "Initializing SPI bus");

        constexpr spi_bus_config_t spi_bus_config{
            .mosi_io_num = DISPLAY_GPIO_MOSI,
            .miso_io_num = GPIO_NUM_NC,
            .sclk_io_num = DISPLAY_GPIO_SCLK,
            .quadwp_io_num = GPIO_NUM_NC,
            .quadhd_io_num = GPIO_NUM_NC,
            .max_transfer_sz = DISPLAY_WIDTH * DISPLAY_HEIGHT * sizeof(uint16_t)
        };
        ESP_RETURN_ON_ERROR(spi_bus_initialize(DISPLAY_SPI_NUM, &spi_bus_config, SPI_DMA_CH_AUTO), TAG,
                            "Failed to init SPI");

        ESP_LOGD(TAG, "Initializing panel IO");

        constexpr esp_lcd_panel_io_spi_config_t io_spi_config{
            .cs_gpio_num = DISPLAY_GPIO_CS,
            .dc_gpio_num = DISPLAY_GPIO_DC,
            .spi_mode = 0,
            .pclk_hz = DISPLAY_PIXEL_CLK_HZ,
            .trans_queue_depth = 10,
            .lcd_cmd_bits = DISPLAY_CMD_BITS,
            .lcd_param_bits = DISPLAY_PARAM_BITS,
        };
        ESP_GOTO_ON_ERROR(esp_lcd_new_panel_io_spi(DISPLAY_SPI_NUM, &io_spi_config, &bp_disp_lcd_io), err, TAG,
                          "Failed to init panel IO");

        st7796_vendor_config_t vendor_config{
            .init_cmds = lcd_init_cmds,
            .init_cmds_size = sizeof(lcd_init_cmds) / sizeof(st7796_lcd_init_cmd_t)
        };

        const esp_lcd_panel_dev_config_t panel_config{
            .reset_gpio_num = DISPLAY_GPIO_RST,
            .color_space = DISPLAY_COLOR_SPACE,
            .data_endian = LCD_RGB_DATA_ENDIAN_BIG,
            .bits_per_pixel = DISPLAY_BITS_PER_PIXEL,
            .vendor_config = &vendor_config
        };
        ESP_GOTO_ON_ERROR(esp_lcd_new_panel_st7796(bp_disp_lcd_io, &panel_config, &bp_disp_lcd_panel), err, TAG,
                          "Failed to init display driver");

        esp_lcd_panel_reset(bp_disp_lcd_panel);
        esp_lcd_panel_init(bp_disp_lcd_panel);
        esp_lcd_panel_mirror(bp_disp_lcd_panel, true, false);
        esp_lcd_panel_invert_color(bp_disp_lcd_panel, true);
        esp_lcd_panel_disp_on_off(bp_disp_lcd_panel, true);

        return ret;
    }

err:
    if (bp_disp_lcd_panel) {
        esp_lcd_panel_del(bp_disp_lcd_panel);
        bp_disp_lcd_panel = nullptr;
    }

    if (bp_disp_lcd_io) {
        esp_lcd_panel_io_del(bp_disp_lcd_io);
        bp_disp_lcd_io = nullptr;
    }

    spi_bus_free(DISPLAY_SPI_NUM);
    return ret;
}

esp_err_t bp::init_touchscreen() {
    constexpr i2c_config_t i2c_config{
        .mode = I2C_MODE_MASTER,
        .sda_io_num = TOUCH_I2C_SDA,
        .scl_io_num = TOUCH_I2C_SCL,
        .sda_pullup_en = GPIO_PULLUP_DISABLE,
        .scl_pullup_en = GPIO_PULLUP_DISABLE,
        .master{
            .clk_speed = TOUCH_I2C_CLK_HZ
        },
    };
    ESP_RETURN_ON_ERROR(i2c_param_config(TOUCH_I2C_NUM, &i2c_config), TAG,
                        "Failed to configure Touch I2C");
    ESP_RETURN_ON_ERROR(i2c_driver_install(TOUCH_I2C_NUM, i2c_config.mode, 0, 0, 0), TAG,
                        "Failed to initialize Touch I2C");

    constexpr esp_lcd_touch_config_t touch_config{
        .x_max = DISPLAY_WIDTH,
        .y_max = DISPLAY_HEIGHT,
        .rst_gpio_num = GPIO_NUM_NC,
        .int_gpio_num = TOUCH_GPIO_INT,
        .levels{
            .reset = 0,
            .interrupt = 0,
        },
        .flags{
            .swap_xy = 0,
            .mirror_x = 0,
            .mirror_y = 0
        }
    };

    esp_lcd_panel_io_handle_t touch_io_handle = nullptr;
    constexpr esp_lcd_panel_io_i2c_config_t touch_io_config{
        .dev_addr = 0x2e,
        .control_phase_bytes = 1,
        .dc_bit_offset = 0,
        .lcd_cmd_bits = 8,
        .flags =
        {
            .disable_control_phase = 1,
        },
    };

    ESP_RETURN_ON_ERROR(esp_lcd_new_panel_io_i2c_v1(TOUCH_I2C_NUM, &touch_io_config, &touch_io_handle), TAG,
                        "Failed to init Touch IO");
    return esp_lcd_touch_new_i2c_cst816s(touch_io_handle, &touch_config, &bp_disp_touch_handle);
}

esp_err_t bp::init_lvgl() {
    constexpr lvgl_port_cfg_t lvgl_cfg = ESP_LVGL_PORT_INIT_CONFIG();
    ESP_RETURN_ON_ERROR(lvgl_port_init(&lvgl_cfg), TAG, "Failed to init LVGL");

    const lvgl_port_display_cfg_t display_cfg{
        .io_handle = bp_disp_lcd_io,
        .panel_handle = bp_disp_lcd_panel,
        .buffer_size = DISPLAY_WIDTH * DISPLAY_DRAW_BUFF_HEIGHT,
        .double_buffer = DISPLAY_DRAW_BUFF_DOUBLE,
        .hres = DISPLAY_WIDTH,
        .vres = DISPLAY_HEIGHT,
        .monochrome = false,
        .rotation = {
            .swap_xy = false,
            .mirror_x = true,
            .mirror_y = false,
        },
        .color_format = LV_COLOR_FORMAT_RGB565,
        .flags = {
            .buff_dma = true,
            .swap_bytes = true,
        }
    };
    bp_lvgl_disp = lvgl_port_add_disp(&display_cfg);

    const lvgl_port_touch_cfg_t touch_cfg{
        .disp = bp_lvgl_disp,
        .handle = bp_disp_touch_handle
    };
    bp_lvgl_touch_indev = lvgl_port_add_touch(&touch_cfg);

    return ESP_OK;
}
