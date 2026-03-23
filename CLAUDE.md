# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Pixel Plumber is a game built in [Processing](https://processing.org/) (Java-based creative coding framework). The player navigates through procedurally generated drain levels, clearing debris to guide water flow while avoiding toxic acid contamination.

## Running the Project

Open `Pixel_Plumber/Pixel_Plumber.pde` in the Processing IDE and run it. There is no build system, test suite, or linter — Processing handles compilation and execution.

The game window is 1035x780 pixels with a 15px grid cell size.

## Architecture

All source files are in `Pixel_Plumber/` as `.pde` files (Processing's Java dialect). Processing concatenates all `.pde` files in a sketch folder, so all classes share a single global namespace.

**Entry point:** `Pixel_Plumber.pde` — defines global constants (grid dimensions, element/state enums), `setup()`, `draw()`, and input handlers that delegate to `Game`.

**Class hierarchy:**
- `Game` — state machine (title/play/death/controls screens), manages game loop, input routing, and UI panel
- `Level` — procedural level generation via cellular automata, fluid simulation (`update()`), shot collision detection
- `Player` — physics-based movement with velocity/friction, collision response against solid pixels
- `Shot` — projectiles fired by the player; left-click destroys dirt, right-click places dirt
- `Pixel` — single grid cell with state (`gas`/`liquid`/`solid`) and element (`air`/`water`/`acid`/`dirt`/`rock`)
- `Pipe` — entry/exit pipes; exit pipe activates when water reaches it, triggering level completion

**Key systems:**
- **Cellular automata:** Level generation uses birth/death rules to create natural-looking caves from dirt, rock, and acid layers that are combined via overlay
- **Fluid simulation:** Water and acid fall with gravity and spread laterally in `Level.update()`, processed each frame on a temp grid to avoid order-dependent artifacts
- **Health mechanic:** Water contacting acid decreases health; reaching 0 triggers death

## Global Constants

Element and state constants are defined as globals in `Pixel_Plumber.pde` (e.g., `air=0`, `water=1`, `acid=2`, `dirt=3`, `rock=4`; `gas=0`, `liquid=1`, `solid=2`). These are referenced across all classes.

## Assets

- `data/silkscreen.ttf` — UI font
- `data/controls.png` — controls screen image
