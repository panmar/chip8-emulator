# chip8-emulator
A [CHIP-8](https://en.wikipedia.org/wiki/CHIP-8) emulator written in Rust.

<p align="center"><img src="sample_game.jpg" alt="example" width="50%"/></p>

## Example
```rust
mod chip8;
mod sdl_platform;

pub fn main() {   
    let mut emulator = chip8::Emulator::new();
    emulator.load_program_from_file("example.ch8");
    let mut platform = sdl_platform::SDLPlatform::new();
    platform.run(&mut emulator);
}
```
