#include "bluetooth.hpp"

#include "display.hpp"
#include "esp_log.h"
#include "nvs_flash.h"

constexpr auto TAG = "bluetooth_init";

class NameCallback final : public NimBLECharacteristicCallbacks {
public:
    void onStatus(NimBLECharacteristic* pCharacteristic, const int code) override {
        ESP_LOGI(TAG, "%s status %d", pCharacteristic->getUUID().toString().c_str(), code);
    }

    void onWrite(NimBLECharacteristic* pCharacteristic, NimBLEConnInfo& connInfo) override {
        ESP_LOGI(
            TAG, "%s been written, new value %s", pCharacteristic->getUUID().toString().c_str(),
            pCharacteristic->getValue().c_str()
        );
    }
};

class ServerCallback final : public NimBLEServerCallbacks {
    bp::BluetoothPairRequestCallback callback_;

public:
    explicit ServerCallback(const bp::BluetoothPairRequestCallback callback) {
        callback_ = callback;
    }

    void onConfirmPassKey(NimBLEConnInfo& connInfo, const uint32_t pin) override {
        callback_(connInfo, pin);
    }
};

class DeviceCallback final : public NimBLEDeviceCallbacks {
public:
    int onStoreStatus(ble_store_status_event* event, void* arg) override {
        ESP_LOGI(TAG, "store error! %d", event->event_code);
        return -1;
    }
};

bp::BPCharacteristics* bp_characteristics;

void bp::init_bluetooth(const BluetoothPairRequestCallback callback) {
    NimBLEDevice::init(BT_DEVICE_NAME);
    NimBLEDevice::setSecurityAuth(true, true, true);
    NimBLEDevice::setSecurityIOCap(BLE_HS_IO_DISPLAY_YESNO);

    static DeviceCallback device_callback{};
    NimBLEDevice::setDeviceCallbacks(&device_callback);

    NimBLEServer* gatt_server = NimBLEDevice::createServer();
    static ServerCallback server_callback{callback};
    gatt_server->setCallbacks(&server_callback);
    gatt_server->advertiseOnDisconnect(true);

    NimBLEService* character_service = gatt_server->createService(CHARACTER_SVC_UUID);
    NimBLEService* screen_service = gatt_server->createService(SCREEN_SVC_UUID);

    static BPCharacteristics chars(character_service, screen_service);
    bp_characteristics = &chars;

    character_service->start();
    screen_service->start();

    NimBLEAdvertising* advertizing = NimBLEDevice::getAdvertising();
    advertizing->addServiceUUID(CHARACTER_SVC_UUID);
    advertizing->setName(BT_DEVICE_NAME);
    advertizing->start();
}

void bp::respond_pair(const NimBLEConnInfo& info, const bool confirm) {
    NimBLEDevice::injectConfirmPasskey(info, confirm);
}

bp::BPCharacteristics::BLECommandHandler::BLECommandHandler(BPCharacteristics* parent) : parent(parent) {
}

void bp::BPCharacteristics::BLECommandHandler::onWrite(
    NimBLECharacteristic* pCharacteristic, NimBLEConnInfo& connInfo
) {
    if (!handler) return;

    bp_client_command_packet command{};

    const auto chr_value = pCharacteristic->getValue();
    std::copy(
        chr_value.begin(),
        chr_value.size() > sizeof(bp_client_command_packet)
            ? chr_value.begin() + sizeof(bp_client_command_packet) - 2
            : chr_value.end(),
        reinterpret_cast<char*>(&command)
    );

    // Magic number check, so I know we're talking the correct language, and it's not just garbage data
    if (command.magic != COMMAND_MAGIC_NUMBER) return;

    const auto [success, response] = handler(
        command.op, std::span{command.data}
    );

    bp_client_response_packet response_packet{
        success,
        command.op,
        {}
    };

    if (const auto* resp_str = std::get_if<std::string>(&response)) {
        resp_str->copy(response_packet.data, sizeof(response_packet.data) - 1);
    } else if (const auto* resp_vec = std::get_if<std::vector<char>>(&response)) {
        std::copy(
            resp_vec->begin(),
            resp_vec->size() > sizeof(response_packet.data)
                ? resp_vec->begin() + sizeof(response_packet.data)
                : resp_vec->end(),
            reinterpret_cast<char*>(&response_packet.data)
        );
    }

    auto _ = parent->response_chr->indicate(response_packet);
}

class BacklightToggler final : public NimBLECharacteristicCallbacks {
public:
    void onWrite(NimBLECharacteristic* pCharacteristic, NimBLEConnInfo& connInfo) override {
        const auto chr_value = pCharacteristic->getValue();

        bool new_value = false;
        memcpy(&new_value, chr_value.data(), sizeof(bool));

        bp::set_backlight_state(new_value);
    }
};

bp::BPCharacteristics::BPCharacteristics(NimBLEService* char_svc, NimBLEService* scr_svc) : command_handler(this) {
    mode_chr = char_svc->createCharacteristic(
        CURRENT_MODE_CHR_UUID,
        READ | READ_ENC | WRITE | WRITE_ENC
    );
    character_name_chr = char_svc->createCharacteristic(
        CHARACTER_NAME_CHR_UUID,
        READ | READ_ENC
    );
    character_species_chr = char_svc->createCharacteristic(
        CHARACTER_SPECIES_CHR_UUID,
        READ | READ_ENC
    );
    action_count_chr = char_svc->createCharacteristic(
        ACTION_COUNT_CHR_UUID,
        READ | READ_ENC
    );
    character_count_chr = char_svc->createCharacteristic(
        CHARACTER_COUNT_CHR_UUID,
        READ | READ_ENC
    );
    command_chr = char_svc->createCharacteristic(
        COMMAND_CHR_UUID,
        WRITE | WRITE_ENC
    );
    response_chr = char_svc->createCharacteristic(
        RESPONSE_CHR_UUID,
        READ | READ_ENC | INDICATE
    );

    command_chr->setCallbacks(&this->command_handler);

    backlight_chr = scr_svc->createCharacteristic(
        BACKLIGHT_CHR_UUID,
        READ | READ_ENC | WRITE | WRITE_ENC
    );

    static BacklightToggler toggler{};
    backlight_chr->setCallbacks(&toggler);
    backlight_chr->setValue(get_backlight_state());
}

void bp::BPCharacteristics::set_character_count(const std::vector<std::string>& names) const {
    character_count_chr->setValue(names.size());
}

void bp::BPCharacteristics::set_character_info(
    const std::string& name,
    const std::string& species,
    const std::size_t action_count
) const {
    mode_chr->setValue(0);
    character_name_chr->setValue(name.c_str());
    character_species_chr->setValue(species.c_str());
    action_count_chr->setValue(action_count);
}

void bp::BPCharacteristics::set_command_handler(const CommandHandler command_handler) {
    this->command_handler.handler = command_handler;
}
