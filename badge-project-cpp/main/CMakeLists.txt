set(PROJECT_SOURCES
        bp/data/character.cpp
        bp/init/bluetooth.cpp
        bp/init/sdcard.cpp
        bp/init/display.cpp
        bp/util/critical.cpp
        bp/util/lvgl_lock.cpp
        bp/util/error.cpp
        bp/util/frame_timer.cpp
        bp/fsm.cpp
        badge-project.cpp)

idf_component_register(SRCS ${PROJECT_SOURCES} INCLUDE_DIRS ".")
target_compile_options(${COMPONENT_LIB} PRIVATE -std=gnu++23 -Wno-missing-field-initializers)