# Aquarium

This repository contains code for evolutionary simulation of computer-like microbes. This project was hugely inspired by [foo52ru](https://www.youtube.com/channel/UCP1JsJgeNs86oqLGnjfGo9Q) and other similar works.

## Building & Running

Project is split in multiple modules, in order to clone it type:
```
git clone --recurse-submodules https://github.com/HectorHW/aquarium.git
```

The backend is written in Rust, React is used for frontend. In order to build the project, you will need `cargo` and `npm`. To issue build, simply invoke included `build` or `build.bat` scripts according to your platform. Alternatively, you can perform all building steps manually:

1. Clone repository with all dependencies
2. Switch to frontend project directory: `cd aquarium-front`
3. Install dependencies required for building frontend by issuing `npm i`
4. Build production version of frontend by typing `npm run build`. This will create `build` directory with all frontend files that will be later served.
5. Switch back to project root: `cd ..`
6. Build backend with cargo: `cargo build --release` and later execute produced binary from project root. Alternatively, issue `cargo run --release` to build (when necessary) and immediately run the server.

## Simulation mechanics

The world has width of 100 and height of 50 squares (hardcoded in [main.rs](src/main.rs), but can be changed if necessary) and is closed in a ring meaning that entity that travels over the right edge will appear on the left. Depending on the depth, bacteria have different amouts of available **sunlight** (which can be used for photosynthesis) and **minerals** (which can be used for energy generation and are accumulated automatically, but are limited in storage per cell).

Every bacteria is a computer consisting of **16 registers** some of which store information about the world (like depth) or bacterium itself (eg. size) and **256 commands**. Commands are executed sequentially (except for jump instruction), when last instruction is run, instruction pointer jumps back to first command. 

Commands are split into **observing** (which usually change bacterium's registers and/or instruction pointer) and **acting** (like producing energy or interacting with the world). Bacterium may execute up to 16 *observing* commands per tick and finish when it encounters *acting* command or when limit of 16 commands is reached. For list of commands see `OpCode` enum in [code.rs](src/cells/code.rs).

Bacteria require energy to operate. When the energy drops to zero, they die leaving dead body which can later be eaten by other bacteria. Energy can be aquired in different ways:

* Photosynthesis at the top part of the world
* Using minerals which are accumulated when bacteria is in the bottom part of the world
* Eating other bacteria. Eating requires extra energy and uses probabalystic mechanic: probability of sucessfully eating other bacterium is computed as: `own_mass / (own_mass + other_mass)`. Cells are rewarded half the victim's mass on sucessfull eating.
* Getting energy (and minerals) from other bacteria if they share it. This in theory, combined with ability to see genetic difference of neighboring bacteria, allows creation of simple multicellular organisms (which was in fact observed trice at least).
  
In order to prevent appearance of never-dying, non-breeding species aging mechanism was introduced. With some small probability (1/1000 right now) bacterium's code may break (by randomly changing single instruction) on every simulation tick.

## Interaction with the simulation

Simulation also offers simple web frontend with multiple buttons that can be used to alter the time flow or content of simulation.

![aquarium_web](https://user-images.githubusercontent.com/29350307/156882481-95306f45-d69b-41cc-93e0-bce8428ffcf5.png)

* Synced/Unsynced button allows to switch between update modes. In synced mode every simulation tick corresponds to one display update (Please note that this hugely slows down the simulation). In unsynced mode server is polled every 500ms. This allows the server to run simulation with less interuption and perform close to maximum tick rate, but results in jumpy, non-smooth display behavior.
* Paused/Unpaused button allows to stop the simulation. This may be useful for taking screenshots or observing cell state.
* Spawn menu allows to spawn cells:
  * Green which will spawn cells with program of 256 photosynthesize instructions. This will most likely result in quick cell growth.
  * Random which will randomly generate programs. Please note that chance of not dying in this case is rather small, but this way should represent the beginning of life in a more realistic way.
* Reset which will clear the field.
* Load which allows to load previously saved field state file from your computer provided that you have the same world settings.
* Save which allows to save all cells to your computer for later loading or sharing.

Heatmap-like coloring represents energy level of cells. Additional view modes may be implemented in the future.

It is also possible to view particular cell's state and code direcly. In order to do that, pause the simulation (otherwise you most likely will miss the desired cell) and click on the square corresponding to the cell you want to view. A new page containing information about cell's minerals, energy, instruction pointer, registers and code will appear.