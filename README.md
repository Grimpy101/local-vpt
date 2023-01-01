# VPT Local Rendering
Based on [VPT web application](https://github.com/terier/vpt) by Žiga Lesar

A command-line application for rendering images from volumetric data, written in Rust and using WebGPU.

## How to run
There are Linux binaries in the release section. The binary is run in command line.

## How to build
To build the script for the specific system, install Rust and Cargo, and run ``cargo build --release``. More about compiling rust applications can be found [here](https://doc.rust-lang.org/cargo/commands/cargo-build.html).

## How it works

1. The script reads all data from files into memory, sets variables and starts the pipeline.

2. It renders the scene. The renderer is a copy of Multiple Scattering Renderer as used in original VPT application. First, the reset pass is done to set light rays to their starting position and clear the image. Then, render pass is executed *n* times (*n* meaning the number of iterations as given by the `--iterations` option - see the Options section).

3. The resulting image is put through another render pass for tone mapping and gamma correction. Tone mapper used is the Artistic Tone Mapper as seen in original VPT application. It can be configured with three arguments as noted in Options section (see `--tones`, `--saturation`, and `gamma`).

4. Final image is written to file.

## Options
Script accepts settings and required data throught script arguments listed below:

* `--volume PATH` *(required)*: A string representing path to file with raw volumetric data (currently accepts only .raw format)
* `--volume-dimensions W H D` *(optional)*: Three integers representing width, height and depth of the volumetric texture (defaults to: authomatically calculated values)
* `--tf PATH` *(optional)*: A string representing path to file with transfer function texture (defaults to: [0,0,0,255, 1,0,0,255])
* `--camera-position X Y Z` *(optional)*: Three floats representing x,y,z coordinates of camera in the scene (defaults to: [-1.0, -1.0, 1.0])
* `--out-resolution W H` *(optional)*: A pair of integers representing resolution of output image (defaults to: [512, 512])
* `--output PATH` *(optional)*: A string representing path to the output image file (defaults to: output.ppm)
* `--steps I` *(optional)*: An integer representing number of iterations when calculating ray movements, on GPU. Used in renderer (defaults to: 100)
* `--anisotropy F` *(optional)*: A float representing anisotropy. Used in renderer (defaults to: 0.0)
* `--extinction F` *(optional)*: A float representing extinction. Used in renderer (defaults to: 100.0)
* `--bounces I` *(optional)*: An integer representing number of bounces per photon. Used in renderer (defaults to: 8.0)
* `--iterations I` *(optional)*: An integer representing number of iterations of rendering. This is different from steps in that this is the number of consecutive jobs on GPU (defaults to: 1)
* `--mvp-matrix F1 F2 F3 F4 F5 F6 F7 F8 F9 F10 F11 F12 F13 F14 F15 F16` *(optional)*: An array of floats representing inverse MVP transformation matrix to use for rendering. If not specified, it is calculated from camera position. The format of array is row-by-row, from left to right, operating on column vectors
* `--focal-length F` *(optional)*: A float representing distance of projection plane from camera origin (defaults to: 2.0)
* `--tones F F F` *(optional)*: Three floats representing low key, midtones, and high key, respectively, in range [0.0, 1.0]. Used in tone mapping (defaults to: [0.0, 0.5, 1.0])
* `--saturation F` *(optional)*: A float representing color saturation of the final visualization. Lower values mean more washed out colors. Used in tone mapping (defaults to: 1.0)
* `--gamma F` *(optional)*: A float representing gamma value to use in gamma correction. Higher values mean lighter dark regions. Used in tone mapping (defaults to: 2.2)

The supported formats for volume data files are:
- raw 3D texture array where each value is a single parameter as unsigned 8-bit integer

The supported formats for transfer function files are:
- 2D texture array where each value contains four parameters RGBA, each being an unsigned 8-bit integer

The program outputs an image in [PPM format version P3](https://en.wikipedia.org/wiki/Netpbm).