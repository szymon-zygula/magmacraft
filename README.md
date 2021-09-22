# Magmacraft
Magmacraft is an experimental renderer written in Rust and Vulkan API using ash bindingins.
As of now, it can load vertex and fragment shaders and use vertex constants.
As Vulkan API is very verbose,
getting this modest functionality required writing over 4300 lines of code.

## Compiling
To compile the project, you need `cargo` and
something like `glslc` to compile glsl shaders into SPIR-V format.

## Running
To properly run, the application requires installed Vulkan validation layers.
For some reason, the dwm window manager causes Magmacraft to crash.
