#![windows_subsystem = "console"]
// #![windows_subsystem = "windows"]

extern crate sdl2;

use crate::chip8::{Key, Platform, SCREEN_HEIGHT, SCREEN_WIDTH};
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
    drawn_pixels: HashSet<(u32, u32)>,
    pressed_keys: HashSet<Keycode>,
}

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
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
            drawn_pixels: HashSet::new(),
            pressed_keys: HashSet::new(),
        }
    }

    fn draw_pixel_no_present(&mut self, x: u32, y: u32) -> bool {
        let mut xored = false;
        if self.drawn_pixels.contains(&(x, y)) {
            self.drawn_pixels.remove(&(x, y));
            xored = true;
        } else {
            self.drawn_pixels.insert((x, y));
        }

        return xored;
    }
}

impl Platform for SDLPlatform {
    fn clear_display(&mut self) {
        self.drawn_pixels.clear();
    }

    fn draw_pixels(&mut self, pixels: &[(u32, u32)]) -> bool {
        let mut xored = false;
        for pixel in pixels {
            xored |= self.draw_pixel_no_present(pixel.0, pixel.1);
        }
        return xored;
    }

    fn is_key_pressed(&self, key: Key) -> bool {
        // NOTE(panmar): Use more convenient QWERTY keyboard mapping
        // 1 2 3 C                 1 2 3 4
        // 4 5 6 D      ====>      Q W E R
        // 7 8 9 E      ====>      A S D F
        // A 0 B F                 Z X C V
        for code in self.pressed_keys.iter() {
            let code_as_i32 = match code {
                Keycode::Num1 => 1,
                Keycode::Num2 => 2,
                Keycode::Num3 => 3,
                Keycode::Q => 4,
                Keycode::W => 5,
                Keycode::E => 6,
                Keycode::A => 7,
                Keycode::S => 8,
                Keycode::D => 9,
                Keycode::Z => 0xA,
                Keycode::X => 0,
                Keycode::C => 0xB,
                Keycode::Num4 => 0xC,
                Keycode::R => 0xD,
                Keycode::F => 0xE,
                Keycode::V => 0xF,
                _ => 42,
            };
            if code_as_i32 == key.value() {
                return true;
            }
        }
        return false;
    }

    fn get_key_pressed(&self) -> Option<Key> {
        for code in self.pressed_keys.iter() {
            match code {
                Keycode::Num0 => return Some(Key::Num0),
                Keycode::Num1 => return Some(Key::Num1),
                Keycode::Num2 => return Some(Key::Num2),
                Keycode::Num3 => return Some(Key::Num3),
                Keycode::Num4 => return Some(Key::Num4),
                Keycode::Num5 => return Some(Key::Num5),
                Keycode::Num6 => return Some(Key::Num6),
                Keycode::Num7 => return Some(Key::Num7),
                Keycode::Num8 => return Some(Key::Num8),
                Keycode::Num9 => return Some(Key::Num9),
                Keycode::A => return Some(Key::A),
                Keycode::B => return Some(Key::B),
                Keycode::C => return Some(Key::C),
                Keycode::D => return Some(Key::D),
                Keycode::E => return Some(Key::E),
                Keycode::F => return Some(Key::F),
                _ => return Option::None,
            };
        }
        return Option::None;
    }

    fn update(&mut self) {
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

        self.pressed_keys = event_pump
            .keyboard_state()
            .pressed_scancodes()
            .filter_map(Keycode::from_scancode)
            .collect();

        // println!("{:?}", self.pressed_keys);
    }

    fn draw(&mut self) {
        self.canvas.set_draw_color(Color::RGB(0, 0, 0));
        self.canvas.clear();

        self.canvas.set_draw_color(Color::RGB(255, 255, 255));
        let pixel_size = 20u32;

        for pixel in self.drawn_pixels.iter() {
            self.canvas
                .fill_rect(Rect::new(
                    pixel_size as i32 * pixel.0 as i32,
                    pixel_size as i32 * pixel.1 as i32,
                    pixel_size,
                    pixel_size,
                ))
                .unwrap();
        }

        self.canvas.present();
    }

    fn pending_close(&self) -> bool {
        self.pending_close
    }

    fn play_sound(&mut self) {
        self.audio.resume();
    }

    fn stop_sound(&mut self) {
        self.audio.pause();
    }
}
