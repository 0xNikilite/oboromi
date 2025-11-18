# Architecture Overview

`oboromi` is designed as a modular emulator foundation. This document outlines the high-level architecture.

## Core Components

### 1. Core (`core/`)
The heart of the emulator. It handles:
- **CPU Emulation**: Wraps Unicorn Engine for ARM64 execution.
- **Memory Management**: Manages the emulated RAM and memory mapping.
- **Loader**: Handles loading of binaries (future).

### 2. GUI (`gui/`)
The frontend interface.
- Built with `eframe` (egui framework).
- Visualizes CPU state, memory, and test results.
- Runs the main loop and drives the core emulation.

## Data Flow

1.  **Initialization**: The GUI initializes the `UnicornCPU` instance.
2.  **Execution**: The GUI triggers execution steps (single step or run).
3.  **Visualization**: After each step/frame, the GUI queries the Core for register states and memory contents to display.

## Key Dependencies

-   **Unicorn Engine**: For CPU emulation.
-   **egui/eframe**: For the graphical user interface.
-   **anyhow**: For error handling.
