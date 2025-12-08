#include "image.hpp"

#include <utility>

#include "esp_log.h"
#include "bp/util/critical.hpp"

namespace bp::image {
    EXT_RAM_BSS_ATTR uint8_t raw_image_storage[IMAGE_STORAGE_SIZE];

    portMUX_TYPE image_spinlock = portMUX_INITIALIZER_UNLOCKED;
    ImageDataAllocator allocator{};

    AllocatedImageData::AllocatedImageData(
        const std::size_t start,
        const std::size_t end
    ) : _valid(true),
        _start(start),
        _end(end) {
    }

    AllocatedImageData::~AllocatedImageData() {
        _valid = false;
    }

    AllocatedImageData::AllocatedImageData(
        AllocatedImageData&& other
    ) noexcept : _valid(other._valid),
                 _start(other._start),
                 _end(other._end) {
        other._valid = false;
    }

    AllocatedImageData& AllocatedImageData::operator=(AllocatedImageData&& other) noexcept {
        if (this == &other)
            return *this;
        _valid = other._valid;
        other._valid = false;
        _start = other._start;
        _end = other._end;
        return *this;
    }

    uint8_t* AllocatedImageData::data() const {
        if (!_valid) throw invalid_access();

        return raw_image_storage + _start;
    }

    std::span<uint8_t> AllocatedImageData::span() const {
        if (!_valid) throw invalid_access();
        return std::span{raw_image_storage + _start, raw_image_storage + _end};
    }

    void ImageDataAllocator::clear_expired() {
        std::erase_if(_allocations, [](const WeakAllocatedImageData& weak_alloc) {
            return weak_alloc.expired();
        });
    }

    std::size_t ImageDataAllocator::largest_block_size_sl() const {
        CriticalGuard guard(&image_spinlock);

        std::size_t block_start = 0;
        std::size_t largest_size = 0;

        for (const auto& potential_alloc : _allocations) {
            if (const auto alloc = potential_alloc.lock(); alloc && alloc->_valid) {
                if (const auto current_size = alloc->_start - block_start; current_size > largest_size) {
                    largest_size = current_size;
                }

                block_start = alloc->_end + 1;
            }
        }

        if (const auto trailing_size = IMAGE_STORAGE_SIZE - block_start; trailing_size > largest_size) {
            return trailing_size;
        }

        return largest_size;
    }

    struct OccupiedSpace {
        std::size_t start;
        std::size_t end;

        OccupiedSpace() = default;

        explicit OccupiedSpace(const AllocatedImageData& allocation)
            : start(allocation.start()),
              end(allocation.end()) {
        }

        friend bool operator<(const OccupiedSpace& lhs, const OccupiedSpace& rhs) {
            if (lhs.end < rhs.start) {
                return true;
            }

            if (rhs.end < lhs.start) {
                return false;
            }

            return true;
        }

        friend bool operator<=(const OccupiedSpace& lhs, const OccupiedSpace& rhs) {
            return rhs >= lhs;
        }

        friend bool operator>(const OccupiedSpace& lhs, const OccupiedSpace& rhs) {
            return rhs < lhs;
        }

        friend bool operator>=(const OccupiedSpace& lhs, const OccupiedSpace& rhs) {
            return !(lhs < rhs);
        }
    };

    std::optional<std::size_t> ImageDataAllocator::find_space(const std::size_t size) const {
        std::vector<OccupiedSpace> occupied{};
        occupied.reserve(_allocations.size());

        for (const auto& potential_alloc : _allocations) {
            if (const auto allocation = potential_alloc.lock()) {
                occupied.emplace_back(*allocation.get());
            }
        }

        std::sort(occupied.begin(), occupied.end());

        std::size_t block_start = 0;

        for (const auto& occlusion : occupied) {
            if (const auto current_size = occlusion.start - block_start; current_size > size) {
                return block_start;
            }

            block_start = occlusion.end + 1;
        }

        if (const auto current_size = IMAGE_STORAGE_SIZE - block_start; current_size > size) {
            return block_start;
        }

        return std::nullopt;
    }

    std::optional<SharedAllocatedImageData> ImageDataAllocator::allocate_image_data_sl(const std::size_t size) {
        CriticalGuard guard(&image_spinlock);

        clear_expired();

        const auto memory_pos = find_space(size);
        if (!memory_pos) {
            return std::nullopt;
        }

        auto ptr = std::make_shared<AllocatedImageData>(
            memory_pos.value(), memory_pos.value() + size - 1
        );
        _allocations.emplace_back(ptr);

        return ptr;
    }

    const char* invalid_access::what() const noexcept {
        return "invalid access of allocated image data";
    }
}
