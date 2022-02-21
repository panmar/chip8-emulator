use rand::Rng;
use std::fs;
use std::mem;
use std::time::Duration;

pub const SCREEN_WIDTH: u32 = 64;
pub const SCREEN_HEIGHT: u32 = 32;

pub trait Platform {
    fn clear_display(&mut self);
    fn draw_pixel(&mut self, x: u32, y: u32);
    fn update(&mut self);
    fn pending_close(&self) -> bool;
}

pub struct Emulator {
    cpu: Cpu,
    memory: [u8; 4096],
    platform: Box<dyn Platform>,
}

struct Cpu {
    registers: [u8; 16],
    register_i: u16,
    program_counter: u16,
    stack: [u16; 16],
    stack_index: i8,
    delay_timer: u8,
    sound_timer: u8,
}

impl Emulator {
    pub fn new(platform: Box<dyn Platform>) -> Emulator {
        let mut cpu: Cpu = unsafe { mem::zeroed() };
        cpu.stack_index = -1;
        let memory: [u8; 4096] = unsafe { mem::zeroed() };
        Emulator {
            cpu,
            memory,
            platform,
        }
    }

    pub fn load_program_from_file(&mut self, filepath: &str) {
        self.load_program_from_data(&fs::read(filepath).unwrap());
    }

    pub fn load_program_from_data(&mut self, data: &Vec<u8>) {
        let mut i = 512;
        for p in data {
            self.memory[i] = *p;
            i += 1;
        }

        self.cpu.program_counter = 512;
    }

    pub fn run(&mut self) {
        while !self.platform.pending_close() {
            self.platform.update();
            self.emulation_step();
            ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
        }
    }

    fn emulation_step(&mut self) {
        let cpu = &mut self.cpu;
        let memory = &mut self.memory;
        let platform = &mut self.platform;

        let operation = u16::from_be_bytes([
            memory[cpu.program_counter as usize],
            memory[(cpu.program_counter + 1) as usize],
        ]);

        print!(
            "[{:#06x}] instruction: {:#06x}",
            cpu.program_counter, operation
        );

        print!(" ");

        for i in 0..0xf {
            print!("r[{}]={} ", i, cpu.registers[i as usize]);
        }
        println!();

        let instruction: [u8; 4] = [
            ((operation & 0xf000) >> 12) as u8,
            ((operation & 0x0f00) >> 8) as u8,
            ((operation & 0x00f0) >> 4) as u8,
            (operation & 0x000f) as u8,
        ];

        cpu.program_counter += 2;

        match instruction {
            [0x0, 0, 0xE, 0] => {
                platform.clear_display();
            }
            [0x0, 0, 0xE, 0xE] => {
                cpu.program_counter = cpu.stack[cpu.stack_index as usize];
                cpu.stack_index -= 1;
            }
            [0x0, _, _, _] => {}
            [0x1, _, _, _] => {
                cpu.program_counter = operation & 0x0fff;
            }
            [0x2, _, _, _] => {
                cpu.stack_index += 1;
                cpu.stack[cpu.stack_index as usize] = cpu.program_counter;
                cpu.program_counter = operation & 0x0fff;
            }
            [0x3, register, _, _] => {
                if cpu.registers[register as usize] == (operation & 0x00ff) as u8 {
                    cpu.program_counter += 2;
                }
            }
            [0x4, register, _, _] => {
                if cpu.registers[register as usize] != (operation & 0x00ff) as u8 {
                    cpu.program_counter += 2;
                }
            }
            [0x5, register_x, register_y, 0] => {
                if cpu.registers[register_x as usize] == cpu.registers[register_y as usize] {
                    cpu.program_counter += 2;
                }
            }
            [0x6, register, _, _] => {
                cpu.registers[register as usize] = (operation & 0x00ff) as u8;
            }
            [0x7, register, _, _] => {
                // state.cpu_registers[register as usize] += (operation & 0x00ff) as u8;
                let result =
                    cpu.registers[register as usize].overflowing_add((operation & 0x00ff) as u8);
                match result {
                    (number, _) => {
                        cpu.registers[register as usize] = number;
                    }
                }
            }
            [0x8, register_x, register_y, 0] => {
                cpu.registers[register_x as usize] = cpu.registers[register_y as usize];
            }
            [0x8, register_x, register_y, 1] => {
                cpu.registers[register_x as usize] |= cpu.registers[register_y as usize];
            }
            [0x8, register_x, register_y, 2] => {
                cpu.registers[register_x as usize] &= cpu.registers[register_y as usize];
            }
            [0x8, register_x, register_y, 3] => {
                cpu.registers[register_x as usize] ^= cpu.registers[register_y as usize];
            }
            [0x8, register_x, register_y, 4] => {
                let result = cpu.registers[register_x as usize]
                    .overflowing_add(cpu.registers[register_y as usize]);
                match result {
                    (number, overflow) => {
                        cpu.registers[register_x as usize] = number;
                        cpu.registers[0xF] = overflow as u8;
                    }
                }
            }
            [0x8, register_x, register_y, 5] => {
                let result = cpu.registers[register_x as usize]
                    .overflowing_sub(cpu.registers[register_y as usize]);
                match result {
                    (number, overflow) => {
                        cpu.registers[register_x as usize] = number;
                        cpu.registers[0xF] = overflow as u8;
                    }
                }
            }
            [0x8, register_x, _, 6] => {
                cpu.registers[0xF] = cpu.registers[register_x as usize] % 2;
                cpu.registers[register_x as usize] /= 2;
            }
            [0x8, register_x, register_y, 7] => {
                let result = cpu.registers[register_y as usize]
                    .overflowing_sub(cpu.registers[register_x as usize]);
                match result {
                    (number, overflow) => {
                        cpu.registers[register_x as usize] = number;
                        cpu.registers[0xF] = overflow as u8;
                    }
                }
            }
            [0x8, register_x, _, 0xE] => {
                let result = cpu.registers[register_x as usize].overflowing_mul(2);
                match result {
                    (number, overflow) => {
                        cpu.registers[register_x as usize] = number;
                        cpu.registers[0xF] = overflow as u8;
                    }
                }
            }
            [0x9, register_x, register_y, 0x0] => {
                if cpu.registers[register_x as usize] != cpu.registers[register_y as usize] {
                    cpu.program_counter += 2;
                }
            }
            [0xA, _, _, _] => {
                cpu.register_i = operation & 0x0fff;
            }
            [0xB, _, _, _] => {
                cpu.program_counter = cpu.registers[0] as u16 + (operation & 0x0fff);
            }
            [0xC, register_x, _, _] => {
                let value: u8 = (operation & 0x00ff) as u8;
                let mut rng = rand::thread_rng();
                let random_number: u8 = rng.gen();
                cpu.registers[register_x as usize] = value & random_number;
            }
            [0xD, register_x, register_y, n] => {
                println!(
                    "Display {} bytes from {} at ({}, {})",
                    n,
                    cpu.register_i,
                    cpu.registers[register_x as usize],
                    cpu.registers[register_y as usize]
                );
                let origin_x = cpu.registers[register_x as usize] as u32;
                let origin_y = cpu.registers[register_y as usize] as u32;
                for i in 0..n {
                    let data = memory[cpu.register_i as usize + i as usize];
                    if data & 0b10000000 != 0 {
                        platform.draw_pixel(origin_x, origin_y + i as u32);
                    }
                    if data & 0b01000000 != 0 {
                        platform.draw_pixel(origin_x + 1, origin_y + i as u32);
                    }
                    if data & 0b00100000 != 0 {
                        platform.draw_pixel(origin_x + 2, origin_y + i as u32);
                    }
                    if data & 0b00010000 != 0 {
                        platform.draw_pixel(origin_x + 3, origin_y + i as u32);
                    }
                    if data & 0b00001000 != 0 {
                        platform.draw_pixel(origin_x + 4, origin_y + i as u32);
                    }
                    if data & 0b00000100 != 0 {
                        platform.draw_pixel(origin_x + 5, origin_y + i as u32);
                    }
                    if data & 0b00000010 != 0 {
                        platform.draw_pixel(origin_x + 6, origin_y + i as u32);
                    }
                    if data & 0b00000001 != 0 {
                        platform.draw_pixel(origin_x + 7, origin_y + i as u32);
                    }
                }
            }
            [0xE, register_x, 0x9, 0xE] => {}
            [0xE, register_x, 0xA, 0x1] => {}
            [0xF, register_x, 0x0, 0x7] => {
                cpu.registers[register_x as usize] = cpu.delay_timer;
            }
            [0xF, register_x, 0x0, 0xA] => {}
            [0xF, register_x, 0x1, 0x5] => {
                cpu.delay_timer = cpu.registers[register_x as usize];
            }
            [0xF, register_x, 0x1, 0x8] => {
                cpu.sound_timer = cpu.registers[register_x as usize];
            }
            [0xF, register_x, 0x1, 0xE] => {
                cpu.register_i += cpu.registers[register_x as usize] as u16;
            }
            [0xF, register_x, 0x2, 0x9] => {
                println!("FONT !!!!!");
            }
            [0xF, register_x, 0x3, 0x3] => {
                let mut value = cpu.registers[register_x as usize];
                memory[(cpu.register_i + 2) as usize] = value % 10;
                value /= 10;
                memory[(cpu.register_i + 1) as usize] = value % 10;
                value /= 10;
                memory[(cpu.register_i + 0) as usize] = value % 10;
            }
            [0xF, register_x, 0x5, 0x5] => {
                for i in 0..register_x {
                    let offset = i as usize;
                    memory[cpu.register_i as usize + offset] = cpu.registers[offset];
                }
            }
            [0xF, register_x, 0x6, 0x5] => {
                for i in 0..register_x {
                    let offset = i as usize;
                    cpu.registers[offset] = memory[cpu.register_i as usize + offset];
                }
            }

            _ => {}
        }
    }
}
