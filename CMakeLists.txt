cmake_minimum_required(VERSION 3.22)
project(oboromi LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Rust core via Corrosion
include(FetchContent)
FetchContent_Declare(
    Corrosion
    GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
    GIT_TAG v0.6
)
FetchContent_MakeAvailable(Corrosion)

corrosion_import_crate(MANIFEST_PATH ${CMAKE_SOURCE_DIR}/Cargo.toml)

# Qt6 GUI
# Configure with: cmake -B build -DCMAKE_PREFIX_PATH="C:/Qt/6.12.0/msvc2022_64" (or your path)
# Qt kit ABI must match Rust toolchain (MSVC vs MinGW)
find_package(Qt6 REQUIRED COMPONENTS Widgets Multimedia)
qt_standard_project_setup()

set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTOUIC ON)
set(CMAKE_AUTORCC ON)

qt_add_executable(oboromi-gui
    gui-qt/src/main.cpp
    gui-qt/src/AudioBackend.h
    gui-qt/src/AudioOutput.h
    gui-qt/src/AudioOutput.cpp
    gui-qt/src/DisplayWidget.h
    gui-qt/src/DisplayWidget.cpp
    gui-qt/src/MainWindow.h
    gui-qt/src/MainWindow.cpp
    gui-qt/src/RustBridge.h
    gui-qt/src/RustBridge.cpp
    gui-qt/resources.qrc
)

target_include_directories(oboromi-gui PRIVATE
    ${CMAKE_SOURCE_DIR}/core/include
)

target_link_libraries(oboromi-gui PRIVATE
    Qt6::Widgets
    Qt6::Multimedia
    oboromi_core
)

# system libs needed by rust staticlib
if(WIN32)
    target_link_libraries(oboromi-gui PRIVATE ws2_32 userenv ntdll bcrypt)
elseif(APPLE)
    target_link_libraries(oboromi_core INTERFACE "-framework IOKit" "-framework CoreFoundation")
else()
    target_link_libraries(oboromi_core INTERFACE udev)
endif()

if(WIN32)
    target_sources(oboromi-gui PRIVATE gui-qt/src/AudioWasapi.cpp)
    target_link_libraries(oboromi-gui PRIVATE ole32)
elseif(APPLE)
    target_sources(oboromi-gui PRIVATE gui-qt/src/AudioCoreAudio.cpp)
    target_link_libraries(oboromi-gui PRIVATE "-framework AudioToolbox")
else()
    find_package(PkgConfig)
    pkg_check_modules(PIPEWIRE IMPORTED_TARGET libpipewire-0.3)
    if(PIPEWIRE_FOUND)
        target_sources(oboromi-gui PRIVATE gui-qt/src/AudioPipeWire.cpp)
        target_compile_definitions(oboromi-gui PRIVATE OBOROMI_PIPEWIRE)
        target_link_libraries(oboromi-gui PRIVATE PkgConfig::PIPEWIRE)
    endif()
    find_package(ALSA)
    if(ALSA_FOUND)
        target_sources(oboromi-gui PRIVATE gui-qt/src/AudioAlsa.cpp)
        target_compile_definitions(oboromi-gui PRIVATE OBOROMI_ALSA)
        target_link_libraries(oboromi-gui PRIVATE ALSA::ALSA)
    endif()
endif()

set_target_properties(oboromi-gui PROPERTIES
    WIN32_EXECUTABLE ON
    OUTPUT_NAME "oboromi"
)

# Auto-deploy Qt DLLs on Windows
if(WIN32)
    get_target_property(_qmake_exe Qt6::qmake IMPORTED_LOCATION)
    get_filename_component(_qt_bin_dir "${_qmake_exe}" DIRECTORY)
    find_program(WINDEPLOYQT_EXECUTABLE windeployqt HINTS "${_qt_bin_dir}")

    if(WINDEPLOYQT_EXECUTABLE)
        add_custom_command(TARGET oboromi-gui POST_BUILD
            COMMAND "${WINDEPLOYQT_EXECUTABLE}" --no-translations "$<TARGET_FILE:oboromi-gui>"
            COMMENT "Deploy Qt DLLs"
        )
    endif()
endif()
