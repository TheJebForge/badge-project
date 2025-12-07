#pragma once
#include <sdkconfig.h>
#include <memory>
#include <vector>
#include <freertos/FreeRTOS.h>

namespace bp::image {
    constexpr uint32_t IMAGE_STORAGE_SIZE = CONFIG_IMAGE_STATIC_STORAGE_SIZE * 1000;
    extern uint8_t raw_image_storage[IMAGE_STORAGE_SIZE];
    extern portMUX_TYPE image_spinlock;

    class ImageDataAllocator;

    class AllocatedImageData;
    using ImageDeallocator = void(*)(const AllocatedImageData&);

    class AllocatedImageData {
        bool _valid;
        std::size_t _start;
        std::size_t _end;

        friend ImageDataAllocator;
    public:
        explicit AllocatedImageData(std::size_t start, std::size_t end);
        ~AllocatedImageData();

        AllocatedImageData(const AllocatedImageData& other) = delete;

        AllocatedImageData(AllocatedImageData&& other) noexcept;

        AllocatedImageData& operator=(const AllocatedImageData& other) = delete;

        AllocatedImageData& operator=(AllocatedImageData&& other) noexcept;

        [[nodiscard]] uint8_t* data() const;
        [[nodiscard]] std::span<uint8_t> span() const;

        [[nodiscard]] std::size_t start() const {
            return _start;
        }

        [[nodiscard]] std::size_t end() const {
            return _end;
        }

        [[nodiscard]] std::size_t len() const {
            return _end - _start + 1;
        }

        [[nodiscard]] bool valid() const {
            return _valid;
        }

        explicit operator bool() const noexcept {
            return _valid;
        }
    };

    using SharedAllocatedImageData = std::shared_ptr<AllocatedImageData>;
    using WeakAllocatedImageData = std::weak_ptr<AllocatedImageData>;

    class ImageDataAllocator {
        std::vector<WeakAllocatedImageData> _allocations{};

        void clear_expired();
        /// Returns end of the found occlusion, or std::nullopt if there wasn't any
        [[nodiscard]] std::optional<std::size_t> check_occlusions(std::size_t start, std::size_t len) const;
        [[nodiscard]] std::optional<std::size_t> find_space(std::size_t size) const;
    public:
        [[nodiscard]] std::size_t largest_block_size_sl() const;

        std::optional<SharedAllocatedImageData> allocate_image_data_sl(std::size_t size);
    };

    class invalid_access final : public std::exception {
    public:
        explicit invalid_access() {}
        [[nodiscard]] const char* what() const noexcept override;
    };

    extern ImageDataAllocator allocator;
}
