#pragma once
#include <span>
#include <variant>

#include "NimBLEDevice.h"

extern "C" {
struct bp_client_command_packet {
    uint8_t magic;
    uint8_t op;
    char data[200];
};

struct bp_client_response_packet {
    bool success;
    uint8_t op;
    char data[200];
};
}


namespace bp {
    constexpr auto BT_DEVICE_NAME = "BP Board";

    using BluetoothPairRequestCallback = void (*)(NimBLEConnInfo, uint32_t);

    constexpr uint16_t MAX_BLE_TRANSFER = 200;

    constexpr auto CHARACTER_SVC_UUID = "c4aa52a4-467e-413f-9559-419eb1a367a7";

    constexpr auto CURRENT_MODE_CHR_UUID = "00000001-467e-413f-9559-419eb1a367a7";
    constexpr auto CHARACTER_NAME_CHR_UUID = "00000002-467e-413f-9559-419eb1a367a7";
    constexpr auto CHARACTER_SPECIES_CHR_UUID = "00000003-467e-413f-9559-419eb1a367a7";
    constexpr auto ACTION_COUNT_CHR_UUID = "00000004-467e-413f-9559-419eb1a367a7";
    constexpr auto COMMAND_CHR_UUID = "00000010-467e-413f-9559-419eb1a367a7";
    constexpr auto RESPONSE_CHR_UUID = "00000011-467e-413f-9559-419eb1a367a7";

    constexpr auto SCREEN_SVC_UUID = "230521b4-d8c4-4e35-9b91-6327de387d77";

    constexpr auto BACKLIGHT_CHR_UUID = "00000001-d8c4-4e35-9b91-6327de387d77";

    void init_bluetooth(BluetoothPairRequestCallback);
    void respond_pair(const NimBLEConnInfo&, bool);

    constexpr uint8_t COMMAND_MAGIC_NUMBER = 242;

    struct ClientCommandResponse {
        bool success;
        std::variant<std::string, std::vector<char>> response;
    };

    using CommandHandler = ClientCommandResponse (*)(uint8_t op, std::span<char, 200> data);

    class BPCharacteristics {
        NimBLECharacteristic* mode_chr;
        NimBLECharacteristic* character_name_chr;
        NimBLECharacteristic* character_species_chr;
        NimBLECharacteristic* action_count_chr;
        NimBLECharacteristic* command_chr;
        NimBLECharacteristic* response_chr;

        NimBLECharacteristic* backlight_chr;

        class BLECommandHandler final : public NimBLECharacteristicCallbacks {
            BPCharacteristics* parent;

        public:
            explicit BLECommandHandler(BPCharacteristics* parent);
            CommandHandler handler = nullptr;
            void onWrite(NimBLECharacteristic* pCharacteristic, NimBLEConnInfo& connInfo) override;
        };

        BLECommandHandler command_handler;

        explicit BPCharacteristics(NimBLEService* char_svc, NimBLEService* scr_svc);

    public:
        void set_character_info(const std::string& name, const std::string& species, std::size_t action_count) const;
        void set_command_handler(CommandHandler command_handler);

        friend void init_bluetooth(BluetoothPairRequestCallback);
    };
}

extern bp::BPCharacteristics* bp_characteristics;
