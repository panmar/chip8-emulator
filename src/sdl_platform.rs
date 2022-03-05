#![windows_subsystem = "console"]
// #![windows_subsystem = "windows"]

extern crate sdl2;

use std::time::{Duration, Instant};

use crate::chip8::{Emulator, SCREEN_HEIGHT, SCREEN_WIDTH};
use sdl2::{
    audio::{AudioCallback, AudioDevice, AudioSpecDesired},
    event::Event,
    keyboard::Keycode,
    pixels::Color,
    rect::Rect,
    render::Canvas,
    video::Window,
    Sdl,
};
use std::collections::HashSet;

pub struct SDLPlatform {
    context: Sdl,
    canvas: Canvas<Window>,
    pending_close: bool,
    audio: AudioDevice<SquareWave>,
}

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

struct Timer {
    timer: Instant,
}

impl Timer {
    fn new() -> Timer {
        Timer {
            timer: Instant::now(),
        }
    }

    fn tick(&mut self) -> Duration {
        let elapsed_time = self.timer.elapsed();
        self.timer = Instant::now();
        elapsed_time
    }
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        for x in out.iter_mut() {
            if self.phase >= 0.0 && self.phase < 0.5 {
                *x = self.volume;
            } else {
                *x = -self.volume;
            }
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

impl SDLPlatform {
    pub fn new() -> SDLPlatform {
        let context = sdl2::init().unwrap();
        let video = context.video().unwrap();
        let audio = context.audio().unwrap();

        let desired_spec = AudioSpecDesired {
            freq: Some(44100),
            channels: Some(1),
            samples: None,
        };

        let audio_device = audio
            .open_playback(None, &desired_spec, |spec| SquareWave {
                phase_inc: 440.0 / spec.freq as f32,
                phase: 0.0,
                volume: 0.25,
            })
            .unwrap();

        let window = video
            .window("CHIP-8 emulator", SCREEN_WIDTH * 20, SCREEN_HEIGHT * 20)
            .position_centered()
            .build()
            .unwrap();
        let canvas = window.into_canvas().build().unwrap();

        SDLPlatform {
            context,
            canvas,
            pending_close: false,
            audio: audio_device,
        }
    }

    pub fn run(&mut self, emulator: &mut Emulator) {
        let mut update_timer = Timer::new();
        while !self.pending_close {
            self.update(emulator, update_timer.tick());
            self.draw(emulator);
        }
    }

    fn update(&mut self, emulator: &mut Emulator, elapsed_time: Duration) {
        self.handle_input(emulator);
        emulator.step(elapsed_time);

        if emulator.cpu.sound_timer > 0 {
            self.audio.resume();
        } else {
            self.audio.pause();
        }
    }

    // NOTE(panmar): Use more convenient QWERTY keyboard mapping
    // 1 2 3 C                 1 2 3 4
    // 4 5 6 D      ====>      Q W E R
    // 7 8 9 E      ====>      A S D F
    // A 0 B F                 Z X C V
    fn handle_input(&mut self, emulator: &mut Emulator) {
        let mut event_pump = self.context.event_pump().unwrap();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => self.pending_close = true,
                _ => {}
            }
        }
        let pressed_keys: HashSet<Keycode> = event_pump
            .keyboard_state()
            .pressed_scancodes()
            .filter_map(Keycode::from_scancode)
            .collect();
        emulator.input.fill(false);
        for keycode in pressed_keys {
            match keycode {
                Keycode::Num1 => emulator.input[1] = true,
                Keycode::Num2 => emulator.input[2] = true,
                Keycode::Num3 => emulator.input[3] = true,
                Keycode::Q => emulator.input[4] = true,
                Keycode::W => emulator.input[5] = true,
                Keycode::E => emulator.input[6] = true,
                Keycode::A => emulator.input[7] = true,
                Keycode::S => emulator.input[8] = true,
                Keycode::D => emulator.input[9] = true,
                Keycode::Z => emulator.input[0xA] = true,
                Keycode::X => emulator.input[0] = true,
                Keycode::C => emulator.input[0xB] = true,
                Keycode::Num4 => emulator.input[0xC] = true,
                Keycode::R => emulator.input[0xD] = true,
                Keycode::F => emulator.input[0xE] = true,
                Keycode::V => emulator.input[0xF] = true,
                _ => {}
            };
        }
    }

    fn draw(&mut self, emulator: &Emulator) {
        self.canvas.set_draw_color(Color::RGB(0, 0, 0));
        self.canvas.clear();

        self.canvas.set_draw_color(Color::RGB(255, 255, 255));
        let pixel_size = 20u32;

        let padding = 2;
        for pixel in emulator.active_pixels.iter() {
            self.canvas
                .fill_rect(Rect::new(
                    pixel_size as i32 * pixel.0 as i32,
                    pixel_size as i32 * pixel.1 as i32,
                    pixel_size - 2 * padding,
                    pixel_size - 2 * padding,
                ))
                .unwrap();
        }

        self.canvas.present();
    }
}
