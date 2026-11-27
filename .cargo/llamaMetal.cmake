# Cargo sets this feature variable in the llama-cpp-sys-2 build script and its child processes.
if(PROJECT_NAME STREQUAL "llama.cpp" AND APPLE AND NOT CMAKE_SYSTEM_NAME STREQUAL "watchOS")
    if(DEFINED ENV{CARGO_FEATURE_METAL})
        set(GGML_METAL ON CACHE BOOL "Enable Apple Metal" FORCE)
    else()
        set(GGML_METAL OFF CACHE BOOL "Enable Apple Metal" FORCE)
    endif()
endif()
