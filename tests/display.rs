use chip8_emulator::sdl_platform::SDLPlatform;
use chip8_emulator::chip8::Emulator;

#[test]
#[ignore]
fn should_display_font() {
    let platform = SDLPlatform::new();
    let mut emulator = Emulator::new(Box::new(platform));
    emulator.load_program_from_data(&vec!{
        0x00, 0xE0,
        0x60, 0x0F,
        0xF0, 0x29,
        0xD2, 0x2A,
    });
    emulator.run();
}