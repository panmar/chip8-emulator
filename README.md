# chip8-emulator
A [CHIP-8](https://en.wikipedia.org/wiki/CHIP-8) emulator written in Rust.

<p align="center"><img src="sample_game.jpg" alt="example" width="75%"/></p>

## Keyboard mapping

Classic CHIP-8 keyboard (0-A) is mapped into the following QWERTY layout:

<table>
    <tbody>
        <tr><td>1</td><td>2</td><td>3</td><td>4</td>
        <tr><td>Q</td><td>W</td><td>E</td><td>R</td>
        <tr><td>A</td><td>S</td><td>D</td><td>F</td>
        <tr><td>Z</td><td>X</td><td>C</td><td>V</td>
    </tr>
    </tbody>
</table>

## Run
```
cargo run -- <filepath-to-rom>
```
## Tests
```
cargo test
```
## Usage
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
## Useful links
* http://devernay.free.fr/hacks/chip8/C8TECH10.HTM#8xy2
* https://tobiasvl.github.io/blog/write-a-chip-8-emulator/#timers
* http://johnearnest.github.io/Octo/docs/XO-ChipSpecification.html
