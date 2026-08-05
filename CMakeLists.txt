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
find_package(Qt6 REQUIRED COMPONENTS Widgets)
qt_standard_project_setup()

set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTOUIC ON)
set(CMAKE_AUTORCC ON)

qt_add_executable(oboromi-gui
    gui-qt/src/main.cpp
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
    oboromi_core
)

# Windows system libs needed by rust staticlib
if(WIN32)
    target_link_libraries(oboromi-gui PRIVATE ws2_32 userenv ntdll bcrypt)
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
