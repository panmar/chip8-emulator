# chip8-emulator
A [CHIP-8](https://en.wikipedia.org/wiki/CHIP-8) emulator written in Rust.

## Example
```rust
mod chip8;
mod sdl_platform;

use crate::sdl_platform::SDLPlatform;

pub fn main() {
    let platform = SDLPlatform::new();
    let mut emulator = chip8::Emulator::new(Box::new(platform));
    emulator.load_program_from_file("example.rom");
    emulator.run();
}
```
