#![windows_subsystem = "console"]
// #![windows_subsystem = "windows"]

extern crate sdl2;

use crate::chip8::{Platform, SCREEN_HEIGHT, SCREEN_WIDTH};
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
