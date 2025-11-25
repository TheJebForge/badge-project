#include "critical.hpp"

CriticalGuard::CriticalGuard(portMUX_TYPE* spinlock_ptr) {
    ptr = spinlock_ptr;
    taskENTER_CRITICAL(spinlock_ptr);
}

CriticalGuard::~CriticalGuard() {
    taskEXIT_CRITICAL(ptr);
}
