use rand::Rng;
use std::fs;
use std::mem;
use std::time::Duration;

pub const SCREEN_WIDTH: u32 = 64;
pub const SCREEN_HEIGHT: u32 = 32;

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
        let mut memory: [u8; 4096] = unsafe { mem::zeroed() };
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
        while !self.platform.pending_close() {
            self.platform.update();
            self.emulation_step();
            self.platform.draw();
            ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
        }
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
        let opcode = u16::from_be_bytes([
            self.memory[self.cpu.program_counter as usize],
            self.memory[(self.cpu.program_counter + 1) as usize],
        ]);

        let instruction = Instruction::parse(opcode);

        // print!(
        //     "[{:#06x}] instruction: {:#06x}",
        //     self.cpu.program_counter, opcode
        // );
        // print!(" ");
        // for i in 0..0xf {
        //     print!("r[{}]={} ", i, self.cpu.registers[i as usize]);
        // }
        // println!();

        // TODO(panmar): This is a hack, probably should be inside execution
        self.cpu.program_counter += 2;

        self.execute(instruction);

        if self.cpu.sound_timer > 0 {
            self.platform.play_sound();
            self.cpu.sound_timer = self.cpu.sound_timer - 1;
        } else {
            self.platform.stop_sound();
        }

        if self.cpu.delay_timer > 0 {
            self.cpu.delay_timer = self.cpu.delay_timer - 1;
        }
    }
}
