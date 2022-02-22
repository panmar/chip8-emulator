#![windows_subsystem = "console"]
// #![windows_subsystem = "windows"]

extern crate sdl2;

use crate::chip8::{Platform, SCREEN_HEIGHT, SCREEN_WIDTH};
use sdl2::{
    event::Event, keyboard::Keycode, pixels::Color, rect::Rect, render::Canvas, video::Window, Sdl,
};

pub struct SDLPlatform {
    context: Sdl,
    canvas: Canvas<Window>,
    pending_close: bool,
}

impl SDLPlatform {
    pub fn new() -> SDLPlatform {
        let context = sdl2::init().unwrap();
        let video_subsystem = context.video().unwrap();
        let window = video_subsystem
            .window("CHIP-8 emulator", SCREEN_WIDTH * 20, SCREEN_HEIGHT * 20)
            .position_centered()
            .build()
            .unwrap();
        let canvas = window.into_canvas().build().unwrap();
        SDLPlatform {
            context,
            canvas,
            pending_close: false,
        }
    }
}

impl Platform for SDLPlatform {
    fn clear_display(&mut self) {
        self.canvas.set_draw_color(Color::RGB(0, 0, 0));
        self.canvas.clear();
        self.canvas.present();
    }

    fn draw_pixel(&mut self, x: u32, y: u32) {
        self.canvas.set_draw_color(Color::RGB(255, 255, 255));
        let pixel_size = 20u32;
        self.canvas
            .fill_rect(Rect::new(
                pixel_size as i32 * x as i32,
                pixel_size as i32 * y as i32,
                pixel_size,
                pixel_size,
            ))
            .unwrap();
        self.canvas.present();
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

    fn pending_close(&self) -> bool {
        self.pending_close
    }
}
