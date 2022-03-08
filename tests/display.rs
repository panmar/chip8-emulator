use chip8_emulator::chip8::Emulator;
use chip8_emulator::sdl_platform::SDLPlatform;

#[test]
#[rustfmt::skip]
#[ignore]
fn should_display_font() {
    let mut emulator = Emulator::new();
    emulator.load_program_from_data(&vec!{
        0x00, 0xE0,
        0x60, 0x0F,
        0xF0, 0x29,
        0xD2, 0x2A,
    });
    let mut platform = SDLPlatform::new();
    platform.run(&mut emulator);
}
