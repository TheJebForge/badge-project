#pragma once

#include <esp_heap_caps.h>

#include "esp_log.h"
#include "sdkconfig.h"

template <class T>
class PSRAMAllocator : std::allocator<T>
{
public:
    using value_type = T;

    PSRAMAllocator() noexcept = default;

    template <class U> constexpr explicit PSRAMAllocator(const PSRAMAllocator<U>&) noexcept
    {
    }

    [[nodiscard]] value_type* allocate(const std::size_t n)
    {
#if CONFIG_SPIRAM

        if (auto p = static_cast<value_type*>(heap_caps_malloc(n * sizeof(value_type), MALLOC_CAP_SPIRAM)))
        {
            return p;
        }
#endif // CONFIG_SPIRAM

        // If unable to allocate on PSRAM, allocate on default heap
        if (auto p2 = static_cast<value_type*>(heap_caps_malloc(n * sizeof(value_type), MALLOC_CAP_DEFAULT)))
        {
            return p2;
        }

        throw std::bad_alloc();
    }

    void deallocate(value_type* p, std::size_t) noexcept
    {
        heap_caps_free(p);
    }
};

template <class T, class U>
bool operator==(const PSRAMAllocator<T>&, const PSRAMAllocator<U>&)
{
    return true;
}

template <class T, class U>
bool operator!=(const PSRAMAllocator<T>& x, const PSRAMAllocator<U>& y)
{
    return !(x == y);
}