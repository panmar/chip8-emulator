use rand::Rng;
use std::fs;
use std::mem;
use std::time::Duration;
use std::time::Instant;

pub const SCREEN_WIDTH: u32 = 64;
pub const SCREEN_HEIGHT: u32 = 32;

const MEMORY_SIZE: usize = 4096;

pub struct Emulator {
    cpu: Cpu,
    memory: [u8; MEMORY_SIZE],
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

pub trait Platform {
    fn clear_display(&mut self);
    fn draw_pixels(&mut self, pixels: &[(u32, u32)]) -> bool;
    fn is_key_pressed(&self, key: Key) -> bool;
    fn get_key_pressed(&self) -> Option<Key>;
    fn update(&mut self);
    fn draw(&mut self);
    fn pending_close(&self) -> bool;
    fn play_sound(&mut self);
    fn stop_sound(&mut self);
}

#[derive(Debug)]
pub enum Key {
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    A,
    B,
    C,
    D,
    E,
    F,
}

impl Key {
    pub fn value(&self) -> u8 {
        match *self {
            Key::Num0 => 0,
            Key::Num1 => 1,
            Key::Num2 => 2,
            Key::Num3 => 3,
            Key::Num4 => 4,
            Key::Num5 => 5,
            Key::Num6 => 6,
            Key::Num7 => 7,
            Key::Num8 => 8,
            Key::Num9 => 9,
            Key::A => 10,
            Key::B => 11,
            Key::C => 12,
            Key::D => 13,
            Key::E => 14,
            Key::F => 15,
        }
    }
}

impl From<u8> for Key {
    fn from(orig: u8) -> Self {
        match orig {
            0 => return Key::Num0,
            1 => return Key::Num1,
            2 => return Key::Num2,
            3 => return Key::Num3,
            4 => return Key::Num4,
            5 => return Key::Num5,
            6 => return Key::Num6,
            7 => return Key::Num7,
            8 => return Key::Num8,
            9 => return Key::Num9,
            10 => return Key::A,
            11 => return Key::B,
            12 => return Key::C,
            13 => return Key::D,
            14 => return Key::E,
            15 => return Key::F,
            _ => return Key::F,
        };
    }
}

#[rustfmt::skip]
enum Instruction {
    ClearDisplay,
    Return,
    Jump { address: u16 },
    Call { address: u16 },
    CondRegEqConstant { register: usize, constant: u8 },
    CondRegNotEqConstant { register: usize, constant: u8 },
    CondRegEqReg { register_lhs: usize, register_rhs: usize },
    AssignConstToReg { register: usize, constant: u8 },
    AddConstToReg { register: usize, constant: u8 },
    AssignRegToReg { register_lhs: usize, register_rhs: usize },
    BitwiseOr { register_lhs: usize, register_rhs: usize },
    BitwiseAnd { register_lhs: usize, register_rhs: usize },
    BitwiseXor { register_lhs: usize, register_rhs: usize },
    AddRegToReg { register_lhs: usize, register_rhs: usize },
    SubReg2FromReg1 { register_lhs: usize, register_rhs: usize },
    BitwiseShrBy1 { register: usize },
    SubReg1FromReg2 { register_lhs: usize, register_rhs: usize },
    BitwiseShlBy1 { register: usize },
    CondRegNotEqReg { register_lhs: usize, register_rhs: usize },
    SetAddress { address: u16 },
    JumpWithV0Offset { address: u16 },
    BitwiseAndWithRand { register: usize, constant: u8 },
    DisplaySprite { register_x: usize, register_y: usize, n_bytes: usize },
    CondKeyPressed { register: usize },
    CondKeyNotPressed { register: usize },
    AssignDelayTimerToReg { register: usize },
    AwaitAndSetKeyPress { register: usize },
    SetDelayTimer { register: usize },
    SetSoundTimer { register: usize },
    AddRegToAddressWithoutCarry { register: usize },
    AssignFontSpriteToAddress { register: usize },
    StoreRegBcd { register: usize },
    SaveRegisters { last_register: usize },
    LoadRegisters { last_register: usize },

    Unknown { opcode: u16 },
}

impl Instruction {
    fn parse(opcode: u16) -> Instruction {
        let hex_digits: [u8; 4] = [
            ((opcode & 0xf000) >> 12) as u8,
            ((opcode & 0x0f00) >> 8) as u8,
            ((opcode & 0x00f0) >> 4) as u8,
            (opcode & 0x000f) as u8,
        ];

        use Instruction::*;
        match hex_digits {
            [0x0, 0, 0xE, 0] => ClearDisplay,
            [0x0, 0, 0xE, 0xE] => Return,
            [0x1, _, _, _] => Jump {
                address: opcode & 0x0fff,
            },
            [0x2, _, _, _] => Call {
                address: opcode & 0x0fff,
            },
            [0x3, register, _, _] => CondRegEqConstant {
                register: register as usize,
                constant: (opcode & 0x00ff) as u8,
            },
            [0x4, register, _, _] => CondRegNotEqConstant {
                register: register as usize,
                constant: (opcode & 0x00ff) as u8,
            },
            [0x5, register_lhs, register_rhs, 0] => CondRegEqReg {
                register_lhs: register_lhs as usize,
                register_rhs: register_rhs as usize,
            },
            [0x6, register, _, _] => AssignConstToReg {
                register: register as usize,
                constant: (opcode & 0x00ff) as u8,
            },
            [0x7, register, _, _] => AddConstToReg {
                register: register as usize,
                constant: (opcode & 0x00ff) as u8,
            },
            [0x8, register_lhs, register_rhs, 0] => AssignRegToReg {
                register_lhs: register_lhs as usize,
                register_rhs: register_rhs as usize,
            },
            [0x8, register_lhs, register_rhs, 1] => BitwiseOr {
                register_lhs: register_lhs as usize,
                register_rhs: register_rhs as usize,
            },
            [0x8, register_lhs, register_rhs, 2] => BitwiseAnd {
                register_lhs: register_lhs as usize,
                register_rhs: register_rhs as usize,
            },
            [0x8, register_lhs, register_rhs, 3] => BitwiseXor {
                register_lhs: register_lhs as usize,
                register_rhs: register_rhs as usize,
            },
            [0x8, register_lhs, register_rhs, 4] => AddRegToReg {
                register_lhs: register_lhs as usize,
                register_rhs: register_rhs as usize,
            },
            [0x8, register_lhs, register_rhs, 5] => SubReg2FromReg1 {
                register_lhs: register_lhs as usize,
                register_rhs: register_rhs as usize,
            },
            [0x8, register, _, 6] => BitwiseShrBy1 {
                register: register as usize,
            },
            [0x8, register_lhs, register_rhs, 7] => SubReg1FromReg2 {
                register_lhs: register_lhs as usize,
                register_rhs: register_rhs as usize,
            },
            [0x8, register, _, 0xE] => BitwiseShlBy1 {
                register: register as usize,
            },
            [0x9, register_lhs, register_rhs, 0x0] => CondRegNotEqReg {
                register_lhs: register_lhs as usize,
                register_rhs: register_rhs as usize,
            },
            [0xA, _, _, _] => SetAddress {
                address: opcode & 0x0fff,
            },
            [0xB, _, _, _] => JumpWithV0Offset {
                address: opcode & 0x0fff,
            },
            [0xC, register, _, _] => BitwiseAndWithRand {
                register: register as usize,
                constant: (opcode & 0x00ff) as u8,
            },
            [0xD, register_x, register_y, n_bytes] => DisplaySprite {
                register_x: register_x as usize,
                register_y: register_y as usize,
                n_bytes: n_bytes as usize,
            },
            [0xE, register, 0x9, 0xE] => CondKeyPressed {
                register: register as usize,
            },
            [0xE, register, 0xA, 0x1] => CondKeyNotPressed {
                register: register as usize,
            },
            [0xF, register, 0x0, 0x7] => AssignDelayTimerToReg {
                register: register as usize,
            },
            [0xF, register, 0x0, 0xA] => AwaitAndSetKeyPress {
                register: register as usize,
            },
            [0xF, register, 0x1, 0x5] => SetDelayTimer {
                register: register as usize,
            },
            [0xF, register, 0x1, 0x8] => SetSoundTimer {
                register: register as usize,
            },
            [0xF, register, 0x1, 0xE] => AddRegToAddressWithoutCarry {
                register: register as usize,
            },
            [0xF, register, 0x2, 0x9] => AssignFontSpriteToAddress {
                register: register as usize,
            },
            [0xF, register, 0x3, 0x3] => StoreRegBcd {
                register: register as usize,
            },
            [0xF, register, 0x5, 0x5] => SaveRegisters {
                last_register: register as usize,
            },
            [0xF, register, 0x6, 0x5] => LoadRegisters {
                last_register: register as usize,
            },
            _ => Unknown { opcode },
        }
    }
}

impl Emulator {
    pub fn new(platform: Box<dyn Platform>) -> Emulator {
        let mut cpu: Cpu = unsafe { mem::zeroed() };
        cpu.stack_index = -1;
        let mut memory: [u8; MEMORY_SIZE] = unsafe { mem::zeroed() };
        Emulator::load_font_data(&mut memory);
        Emulator {
            cpu,
            memory,
            platform,
        }
    }

    fn load_font_data(memory: &mut [u8]) {
        // "0"
        memory[0x0000 + 0] = 0xF0;
        memory[0x0000 + 2] = 0x90;
        memory[0x0000 + 4] = 0x90;
        memory[0x0000 + 6] = 0x90;
        memory[0x0000 + 8] = 0xF0;

        // "1"
        memory[0x000A + 0] = 0x20;
        memory[0x000A + 2] = 0x60;
        memory[0x000A + 4] = 0x20;
        memory[0x000A + 6] = 0x20;
        memory[0x000A + 8] = 0x70;

        // "2"
        memory[0x0014 + 0] = 0xF0;
        memory[0x0014 + 2] = 0x10;
        memory[0x0014 + 4] = 0xF0;
        memory[0x0014 + 6] = 0x80;
        memory[0x0014 + 8] = 0xF0;

        // "3"
        memory[0x001E + 0] = 0xF0;
        memory[0x001E + 2] = 0x10;
        memory[0x001E + 4] = 0xF0;
        memory[0x001E + 6] = 0x10;
        memory[0x001E + 8] = 0xF0;

        // "4"
        memory[0x0028 + 0] = 0x90;
        memory[0x0028 + 2] = 0x90;
        memory[0x0028 + 4] = 0xF0;
        memory[0x0028 + 6] = 0x10;
        memory[0x0028 + 8] = 0x10;

        // "5"
        memory[0x0032 + 0] = 0xF0;
        memory[0x0032 + 2] = 0x80;
        memory[0x0032 + 4] = 0xF0;
        memory[0x0032 + 6] = 0x10;
        memory[0x0032 + 8] = 0xF0;

        // "6"
        memory[0x003C + 0] = 0xF0;
        memory[0x003C + 2] = 0x80;
        memory[0x003C + 4] = 0xF0;
        memory[0x003C + 6] = 0x90;
        memory[0x003C + 8] = 0xF0;

        // "7"
        memory[0x0046 + 0] = 0xF0;
        memory[0x0046 + 2] = 0x10;
        memory[0x0046 + 4] = 0x20;
        memory[0x0046 + 6] = 0x40;
        memory[0x0046 + 8] = 0x40;

        // "8"
        memory[0x0050 + 0] = 0xF0;
        memory[0x0050 + 2] = 0x90;
        memory[0x0050 + 4] = 0xF0;
        memory[0x0050 + 6] = 0x90;
        memory[0x0050 + 8] = 0xF0;

        // "9"
        memory[0x005A + 0] = 0xF0;
        memory[0x005A + 2] = 0x90;
        memory[0x005A + 4] = 0xF0;
        memory[0x005A + 6] = 0x10;
        memory[0x005A + 8] = 0xF0;

        // "A"
        memory[0x0064 + 0] = 0xF0;
        memory[0x0064 + 2] = 0x90;
        memory[0x0064 + 4] = 0xF0;
        memory[0x0064 + 6] = 0x90;
        memory[0x0064 + 8] = 0x90;

        // "B"
        memory[0x006E + 0] = 0xE0;
        memory[0x006E + 2] = 0x90;
        memory[0x006E + 4] = 0xE0;
        memory[0x006E + 6] = 0x90;
        memory[0x006E + 8] = 0xE0;

        // "C"
        memory[0x0078 + 0] = 0xF0;
        memory[0x0078 + 2] = 0x80;
        memory[0x0078 + 4] = 0x80;
        memory[0x0078 + 6] = 0x80;
        memory[0x0078 + 8] = 0xF0;

        // "D"
        memory[0x0082 + 0] = 0xE0;
        memory[0x0082 + 2] = 0x90;
        memory[0x0082 + 4] = 0x90;
        memory[0x0082 + 6] = 0x90;
        memory[0x0082 + 8] = 0xE0;

        // "E"
        memory[0x008C + 0] = 0xF0;
        memory[0x008C + 2] = 0x80;
        memory[0x008C + 4] = 0xF0;
        memory[0x008C + 6] = 0x80;
        memory[0x008C + 8] = 0xF0;

        // "F"
        memory[0x0096 + 0] = 0xF0;
        memory[0x0096 + 2] = 0x80;
        memory[0x0096 + 4] = 0xF0;
        memory[0x0096 + 6] = 0x80;
        memory[0x0096 + 8] = 0x80;
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
        fn execute_if_elapsed(
            emulator: &mut Emulator,
            func: &dyn Fn(&mut Emulator),
            elapsed_time: &mut Instant,
            elapsed_time_limit: Duration,
        ) {
            if elapsed_time.elapsed() > elapsed_time_limit {
                func(emulator);
                *elapsed_time = Instant::now();
            }
        }
        let mut cpu_step_timer = Instant::now();
        let mut render_step_timer = Instant::now();
        let mut sound_timer = Instant::now();
        let mut delay_timer = Instant::now();
        while !self.platform.pending_close() {
            self.platform.update();
            execute_if_elapsed(
                self,
                &Emulator::emulation_step,
                &mut cpu_step_timer,
                Duration::from_millis(2),
            );

            execute_if_elapsed(
                self,
                &Emulator::sound_update,
                &mut sound_timer,
                Duration::from_micros(16666),
            );

            execute_if_elapsed(
                self,
                &Emulator::timer_update,
                &mut delay_timer,
                Duration::from_micros(16666),
            );

            execute_if_elapsed(
                self,
                &Emulator::draw,
                &mut render_step_timer,
                Duration::from_micros(16666),
            );
        }
    }

    fn sound_update(&mut self) {
        if self.cpu.sound_timer > 0 {
            self.platform.play_sound();
            self.cpu.sound_timer = self.cpu.sound_timer - 1;
        } else {
            self.platform.stop_sound();
        }
    }

    fn timer_update(&mut self) {
        if self.cpu.delay_timer > 0 {
            self.cpu.delay_timer = self.cpu.delay_timer - 1;
        }
    }

    fn draw(&mut self) {
        self.platform.draw();
    }

    fn execute(&mut self, instruction: Instruction) {
        let cpu = &mut self.cpu;
        let memory = &mut self.memory;
        let platform = &mut self.platform;

        use Instruction::*;
        match instruction {
            ClearDisplay => platform.clear_display(),
            Return => {
                cpu.program_counter = cpu.stack[cpu.stack_index as usize];
                cpu.stack_index -= 1;
            }
            Jump { address } => cpu.program_counter = address,
            Call { address } => {
                cpu.stack_index += 1;
                cpu.stack[cpu.stack_index as usize] = cpu.program_counter;
                cpu.program_counter = address;
            }
            CondRegEqConstant { register, constant } => {
                if cpu.registers[register] == constant {
                    cpu.program_counter += 2;
                }
            }
            CondRegNotEqConstant { register, constant } => {
                if cpu.registers[register] != constant {
                    cpu.program_counter += 2;
                }
            }
            CondRegEqReg {
                register_lhs,
                register_rhs,
            } => {
                if cpu.registers[register_lhs] == cpu.registers[register_rhs] {
                    cpu.program_counter += 2;
                }
            }
            AssignConstToReg { register, constant } => cpu.registers[register] = constant,
            AddConstToReg { register, constant } => {
                let result = cpu.registers[register].overflowing_add(constant);
                match result {
                    (number, _) => {
                        cpu.registers[register] = number;
                    }
                }
            }
            AssignRegToReg {
                register_lhs,
                register_rhs,
            } => cpu.registers[register_lhs] = cpu.registers[register_rhs],
            BitwiseOr {
                register_lhs,
                register_rhs,
            } => cpu.registers[register_lhs] |= cpu.registers[register_rhs],
            BitwiseAnd {
                register_lhs,
                register_rhs,
            } => cpu.registers[register_lhs] &= cpu.registers[register_rhs],
            BitwiseXor {
                register_lhs,
                register_rhs,
            } => cpu.registers[register_lhs] ^= cpu.registers[register_rhs],
            AddRegToReg {
                register_lhs,
                register_rhs,
            } => {
                let result =
                    cpu.registers[register_lhs].overflowing_add(cpu.registers[register_rhs]);
                match result {
                    (sum, overflow) => {
                        cpu.registers[register_lhs as usize] = sum;
                        cpu.registers[0xF] = overflow as u8;
                    }
                }
            }
            SubReg2FromReg1 {
                register_lhs,
                register_rhs,
            } => {
                let result =
                    cpu.registers[register_lhs].overflowing_sub(cpu.registers[register_rhs]);
                match result {
                    (sub, overflow) => {
                        cpu.registers[register_lhs] = sub;
                        cpu.registers[0xF] = overflow as u8;
                    }
                }
            }
            BitwiseShrBy1 { register } => {
                cpu.registers[0xF] = cpu.registers[register] % 2;
                cpu.registers[register] /= 2;
            }
            SubReg1FromReg2 {
                register_lhs,
                register_rhs,
            } => {
                let result =
                    cpu.registers[register_rhs].overflowing_sub(cpu.registers[register_lhs]);
                match result {
                    (sub, overflow) => {
                        cpu.registers[register_lhs] = sub;
                        cpu.registers[0xF] = overflow as u8;
                    }
                }
            }
            BitwiseShlBy1 { register } => {
                let result = cpu.registers[register].overflowing_mul(2);
                match result {
                    (mul, overflow) => {
                        cpu.registers[register] = mul;
                        cpu.registers[0xF] = overflow as u8;
                    }
                }
            }
            CondRegNotEqReg {
                register_lhs,
                register_rhs,
            } => {
                if cpu.registers[register_lhs] != cpu.registers[register_rhs] {
                    cpu.program_counter += 2;
                }
            }
            SetAddress { address } => cpu.register_i = address,
            JumpWithV0Offset { address } => cpu.program_counter = cpu.registers[0] as u16 + address,
            BitwiseAndWithRand { register, constant } => {
                let mut rng = rand::thread_rng();
                let random_number: u8 = rng.gen();
                cpu.registers[register] = constant & random_number;
            }
            DisplaySprite {
                register_x,
                register_y,
                n_bytes,
            } => {
                let origin_x = cpu.registers[register_x] as u32;
                let origin_y = cpu.registers[register_y] as u32;
                let mut pixels = Vec::new();
                for i in 0..n_bytes {
                    let data = memory[cpu.register_i as usize + i as usize];
                    if data & 0b10000000 != 0 {
                        pixels.push((origin_x, origin_y + i as u32));
                    }
                    if data & 0b01000000 != 0 {
                        pixels.push((origin_x + 1, origin_y + i as u32));
                    }
                    if data & 0b00100000 != 0 {
                        pixels.push((origin_x + 2, origin_y + i as u32));
                    }
                    if data & 0b00010000 != 0 {
                        pixels.push((origin_x + 3, origin_y + i as u32));
                    }
                    if data & 0b00001000 != 0 {
                        pixels.push((origin_x + 4, origin_y + i as u32));
                    }
                    if data & 0b00000100 != 0 {
                        pixels.push((origin_x + 5, origin_y + i as u32));
                    }
                    if data & 0b00000010 != 0 {
                        pixels.push((origin_x + 6, origin_y + i as u32));
                    }
                    if data & 0b00000001 != 0 {
                        pixels.push((origin_x + 7, origin_y + i as u32));
                    }
                }

                let mut xored = false;
                if !pixels.is_empty() {
                    xored = platform.draw_pixels(&pixels);
                }

                if xored {
                    cpu.registers[0xF] = 1;
                } else {
                    cpu.registers[0xF] = 0;
                }
            }
            CondKeyPressed { register } => {
                let key: Key = cpu.registers[register].into();
                if self.platform.is_key_pressed(key) {
                    cpu.program_counter += 2;
                }
            }
            CondKeyNotPressed { register } => {
                let key: Key = cpu.registers[register].into();
                if !self.platform.is_key_pressed(key) {
                    cpu.program_counter += 2;
                }
            }
            AssignDelayTimerToReg { register } => cpu.registers[register] = cpu.delay_timer,
            AwaitAndSetKeyPress { register } => match self.platform.get_key_pressed() {
                Some(keycode) => cpu.registers[register] = keycode.value(),
                _ => cpu.program_counter -= 2,
            },
            SetDelayTimer { register } => cpu.delay_timer = cpu.registers[register],
            SetSoundTimer { register } => cpu.sound_timer = cpu.registers[register],
            AddRegToAddressWithoutCarry { register } => {
                cpu.register_i += cpu.registers[register] as u16
            }
            AssignFontSpriteToAddress { register } => {
                let character = cpu.registers[register];
                cpu.register_i = match character {
                    0 => 0x0000,
                    1 => 0x000A,
                    2 => 0x0014,
                    3 => 0x001E,
                    4 => 0x0028,
                    5 => 0x0032,
                    6 => 0x003C,
                    7 => 0x0046,
                    8 => 0x0050,
                    9 => 0x005A,
                    0xA => 0x0064,
                    0xB => 0x006E,
                    0xC => 0x0078,
                    0xD => 0x0082,
                    0xE => 0x008C,
                    0xF => 0x0096,
                    _ => cpu.register_i,
                }
            }
            StoreRegBcd { register } => {
                let mut value = cpu.registers[register];
                memory[(cpu.register_i + 2) as usize] = value % 10;
                value /= 10;
                memory[(cpu.register_i + 1) as usize] = value % 10;
                value /= 10;
                memory[(cpu.register_i + 0) as usize] = value % 10;
            }
            SaveRegisters { last_register } => {
                for i in 0..=last_register {
                    memory[cpu.register_i as usize + i] = cpu.registers[i];
                }
            }
            LoadRegisters { last_register } => {
                for i in 0..=last_register {
                    cpu.registers[i] = memory[cpu.register_i as usize + i];
                }
            }

            Unknown { opcode } => {
                println!("Unknown instruction: {:#06x}", opcode)
            }
        }
    }

    fn emulation_step(&mut self) {

        let opcode = self.fetch_opcode().unwrap();
        let instruction = Instruction::parse(opcode);

        // print!(
        //     "[{:#06x}] instruction: {:#06x}",
        //     self.cpu.program_counter - 2, opcode
        // );
        // print!(" ");
        // for i in 0..0xf {
        //     print!("r[{}]={} ", i, self.cpu.registers[i as usize]);
        // }
        // println!();

        self.execute(instruction);
    }

    fn fetch_opcode(&mut self) -> Option<u16> {
        let opcode = u16::from_be_bytes([
            self.memory[self.cpu.program_counter as usize],
            self.memory[(self.cpu.program_counter + 1) as usize],
        ]);

        self.cpu.program_counter += 2;
        return Some(opcode);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_hex::assert_eq_hex;

    #[test]
    #[rustfmt::skip]
    fn should_convert_from_instruction_to_opcode() {
        use Instruction::*;
        assert_eq_hex!(ClearDisplay.to_opcode(), 0x00E0);
        assert_eq_hex!(Return.to_opcode(), 0x00EE);
        assert_eq_hex!(Jump{address: 0x04F1}.to_opcode(), 0x14F1);
        assert_eq_hex!(Call{address: 0x07AB}.to_opcode(), 0x27AB);
        assert_eq_hex!(CondRegEqConstant{register: 0xA, constant: 0xC3}.to_opcode(), 0x3AC3);
        assert_eq_hex!(CondRegNotEqConstant{register: 1, constant: 0x23}.to_opcode(), 0x4123);
        assert_eq_hex!(CondRegEqReg{register_lhs: 0xA, register_rhs: 0xD}.to_opcode(), 0x5AD0);
        assert_eq_hex!(AssignConstToReg{register: 7, constant: 0xAF}.to_opcode(), 0x67AF);
        assert_eq_hex!(AddConstToReg{register: 0xC, constant: 0x42}.to_opcode(), 0x7C42);
        assert_eq_hex!(AssignRegToReg{register_lhs: 0x9, register_rhs: 0x3}.to_opcode(), 0x8930);
        assert_eq_hex!(BitwiseOr{register_lhs: 0x5, register_rhs: 0xF}.to_opcode(), 0x85F1);
        assert_eq_hex!(BitwiseAnd{register_lhs: 0x5, register_rhs: 0xF}.to_opcode(), 0x85F2);
        assert_eq_hex!(BitwiseXor{register_lhs: 0x5, register_rhs: 0xF}.to_opcode(), 0x85F3);
        assert_eq_hex!(AddRegToReg{register_lhs: 0x6, register_rhs: 0x0}.to_opcode(), 0x8604);
        assert_eq_hex!(SubReg2FromReg1{register_lhs: 0xA, register_rhs: 0xB}.to_opcode(), 0x8AB5);

        // TODO(panmar): There is some ambiguity about this instruction;
        // Y register (3rd digit) seems to be unused
        assert_eq_hex!(BitwiseShrBy1{register: 0x9}.to_opcode(), 0x8906);

        assert_eq_hex!(SubReg1FromReg2{register_lhs: 0xA, register_rhs: 0xB}.to_opcode(), 0x8AB7);

        // TODO(panmar): There is some ambiguity about this instruction;
        // Y register (3rd digit) seems to be unused
        assert_eq_hex!(BitwiseShlBy1{register: 0x9}.to_opcode(), 0x890E);

        assert_eq_hex!(CondRegNotEqReg{register_lhs: 0xA, register_rhs: 0xB}.to_opcode(), 0x9AB0);
        assert_eq_hex!(SetAddress{address: 0x123}.to_opcode(), 0xA123);
        assert_eq_hex!(JumpWithV0Offset{address: 0x123}.to_opcode(), 0xB123);
        assert_eq_hex!(BitwiseAndWithRand{register: 0xA, constant: 0xB4}.to_opcode(), 0xCAB4);
        assert_eq_hex!(DisplaySprite{register_x: 0xA, register_y: 0xB, n_bytes: 9}.to_opcode(), 0xDAB9);
        assert_eq_hex!(CondKeyPressed{register: 0x5}.to_opcode(), 0xE59E);
        assert_eq_hex!(CondKeyNotPressed{register: 0x5}.to_opcode(), 0xE5A1);
        assert_eq_hex!(AssignDelayTimerToReg{register: 0x5}.to_opcode(), 0xF507);
        assert_eq_hex!(AwaitAndSetKeyPress{register: 0x5}.to_opcode(), 0xF50A);
        assert_eq_hex!(SetDelayTimer{register: 0x3}.to_opcode(), 0xF315);
        assert_eq_hex!(SetSoundTimer{register: 0x3}.to_opcode(), 0xF318);
        assert_eq_hex!(AddRegToAddressWithoutCarry{register: 0x5}.to_opcode(), 0xF51E);
        assert_eq_hex!(AssignFontSpriteToAddress{register: 0x5}.to_opcode(), 0xF529);
        assert_eq_hex!(StoreRegBcd{register: 0x7}.to_opcode(), 0xF733);
        assert_eq_hex!(SaveRegisters{last_register: 0x7}.to_opcode(), 0xF755);
        assert_eq_hex!(LoadRegisters{last_register: 0x7}.to_opcode(), 0xF765);
    }

    struct TestPlatform {}

    impl Platform for TestPlatform {
        fn clear_display(&mut self) {}
        fn draw_pixels(&mut self, _pixels: &[(u32, u32)]) -> bool {
            false
        }
        fn is_key_pressed(&self, _key: Key) -> bool {
            false
        }
        fn get_key_pressed(&self) -> Option<Key> {
            None
        }
        fn update(&mut self) {}
        fn draw(&mut self) {}
        fn pending_close(&self) -> bool {
            false
        }
        fn play_sound(&mut self) {}
        fn stop_sound(&mut self) {}
    }

    impl Emulator {
        fn load_program_from_instructions(&mut self, instructions: &Vec<Instruction>) {
            let mut data: Vec<u8> = Vec::new();
            for instruction in instructions {
                let opcode = instruction.to_opcode();
                data.push(((opcode & 0xFF00) >> 8) as u8);
                data.push((opcode & 0x00FF) as u8);
            }
            self.load_program_from_data(&data);
        }
    }

    impl Instruction {
        fn to_opcode(&self) -> u16 {
            use Instruction::*;
            let opcode = match self {
                ClearDisplay => 0x00E0,
                Return => 0x00EE,
                Jump { address } => 0x1000 | address,
                Call { address } => 0x2000 | address,
                CondRegEqConstant { register, constant } => {
                    0x3000 | ((*register as u16) << 8) | (*constant as u16)
                }
                CondRegNotEqConstant { register, constant } => {
                    0x4000 | ((*register as u16) << 8) | (*constant as u16)
                }
                CondRegEqReg {
                    register_lhs,
                    register_rhs,
                } => 0x5000 | ((*register_lhs as u16) << 8) | ((*register_rhs as u16) << 4),
                AssignConstToReg { register, constant } => {
                    0x6000 | ((*register as u16) << 8) | (*constant as u16)
                }
                AddConstToReg { register, constant } => {
                    0x7000 | ((*register as u16) << 8) | (*constant as u16)
                }
                AssignRegToReg {
                    register_lhs,
                    register_rhs,
                } => 0x8000 | ((*register_lhs as u16) << 8) | ((*register_rhs as u16) << 4),
                BitwiseOr {
                    register_lhs,
                    register_rhs,
                } => 0x8001 | ((*register_lhs as u16) << 8) | ((*register_rhs as u16) << 4),
                BitwiseAnd {
                    register_lhs,
                    register_rhs,
                } => 0x8002 | ((*register_lhs as u16) << 8) | ((*register_rhs as u16) << 4),
                BitwiseXor {
                    register_lhs,
                    register_rhs,
                } => 0x8003 | ((*register_lhs as u16) << 8) | ((*register_rhs as u16) << 4),
                AddRegToReg {
                    register_lhs,
                    register_rhs,
                } => 0x8004 | ((*register_lhs as u16) << 8) | ((*register_rhs as u16) << 4),
                SubReg2FromReg1 {
                    register_lhs,
                    register_rhs,
                } => 0x8005 | ((*register_lhs as u16) << 8) | ((*register_rhs as u16) << 4),
                BitwiseShrBy1 { register } => 0x8006 | ((*register as u16) << 8),
                SubReg1FromReg2 {
                    register_lhs,
                    register_rhs,
                } => 0x8007 | ((*register_lhs as u16) << 8) | ((*register_rhs as u16) << 4),
                BitwiseShlBy1 { register } => 0x800E | ((*register as u16) << 8),
                CondRegNotEqReg {
                    register_lhs,
                    register_rhs,
                } => 0x9000 | ((*register_lhs as u16) << 8) | ((*register_rhs as u16) << 4),
                SetAddress { address } => 0xA000 | address,
                JumpWithV0Offset { address } => 0xB000 | address,
                BitwiseAndWithRand { register, constant } => {
                    0xC000 | ((*register as u16) << 8) | *constant as u16
                }
                DisplaySprite {
                    register_x,
                    register_y,
                    n_bytes,
                } => {
                    0xD000
                        | ((*register_x as u16) << 8)
                        | ((*register_y as u16) << 4)
                        | *n_bytes as u16
                }
                CondKeyPressed { register } => 0xE09E | ((*register as u16) << 8),
                CondKeyNotPressed { register } => 0xE0A1 | ((*register as u16) << 8),
                AssignDelayTimerToReg { register } => 0xF007 | ((*register as u16) << 8),
                AwaitAndSetKeyPress { register } => 0xF00A | ((*register as u16) << 8),
                SetDelayTimer { register } => 0xF015 | ((*register as u16) << 8),
                SetSoundTimer { register } => 0xF018 | ((*register as u16) << 8),
                AddRegToAddressWithoutCarry { register } => 0xF01E | ((*register as u16) << 8),
                AssignFontSpriteToAddress { register } => 0xF029 | ((*register as u16) << 8),
                StoreRegBcd { register } => 0xF033 | ((*register as u16) << 8),
                SaveRegisters { last_register } => 0xF055 | ((*last_register as u16) << 8),
                LoadRegisters { last_register } => 0xF065 | ((*last_register as u16) << 8),

                Unknown { opcode } => *opcode,
            };
            return opcode;
        }
    }

    #[test]
    #[ignore]
    #[rustfmt::skip]
    fn should_display_sprite_font() {
        let mut emulator = Emulator::new(Box::new(TestPlatform {}));
        use Instruction::*;
        emulator.load_program_from_instructions(&vec![
            ClearDisplay,
            AssignConstToReg { register: 0, constant: 0xA },
            AssignFontSpriteToAddress { register: 0x0 },
            AssignConstToReg { register: 1, constant: 0xA },
            AssignConstToReg { register: 2, constant: 0x5 },
            DisplaySprite { register_x: 0x1, register_y: 0x2, n_bytes: 0xA },
        ]);
        emulator.run();
    }
}
