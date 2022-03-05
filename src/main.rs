mod chip8;
mod sdl_platform;

use crate::{chip8::Emulator, sdl_platform::SDLPlatform};
use std::{env, process::exit};

pub fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!(
            "Wrong number of arguments: expected {}, given {}",
            1,
            args.len() - 1
        );
        exit(1);
    }

    let mut emulator = Emulator::new();
    emulator.load_program_from_file(&args[1]);
    let mut platform = SDLPlatform::new();
    platform.run(&mut emulator);
}
