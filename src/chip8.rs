use rand::Rng;
use std::collections::HashSet;
use std::fs;
use std::time::Duration;

pub const SCREEN_WIDTH: u32 = 64;
pub const SCREEN_HEIGHT: u32 = 32;

const MEMORY_SIZE: usize = 4096;

pub struct Emulator {
    pub cpu: Cpu,
    pub memory: [u8; MEMORY_SIZE],
    pub active_pixels: HashSet<(u32, u32)>,
    pub input: [bool; 16],
    cpu_timer: Duration,
    sound_timer: Duration,
    delay_timer: Duration,
}

pub struct Cpu {
    pub registers: [u8; 16],
    pub register_i: u16,
    pub program_counter: u16,
    pub stack: [u16; 16],
    pub stack_index: i8,
    pub delay_timer: u8,
    pub sound_timer: u8,
}

#[rustfmt::skip]
enum Instruction {
    ClearDisplay,
    Return,
    Jump { address: u16 },
    Call { address: u16 },
    SkipIfRegEqConstant { register: usize, constant: u8 },
    SkipIfRegNotEqConstant { register: usize, constant: u8 },
    SkipIfRegEqReg { register_lhs: usize, register_rhs: usize },
    SetRegToConstant { register: usize, constant: u8 },
    AddConstToReg { register: usize, constant: u8 },
    SetRegToReg { register_lhs: usize, register_rhs: usize },
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
    SkipIfKeyPressed { register: usize },
    SkipIfKeyNotPressed { register: usize },
    SetRegToDelayTimer { register: usize },
    AwaitAndSetKeyPress { register: usize },
    SetDelayTimer { register: usize },
    SetSoundTimer { register: usize },
    AddRegToAddressWithoutCarry { register: usize },
    SetAddressOfFontChar { register: usize },
    StoreRegBcd { register: usize },
    StoreRegisters { last_register: usize },
    LoadRegisters { last_register: usize },

    Unknown { opcode: u16 },
}

impl Instruction {
    fn decode(opcode: u16) -> Instruction {
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
            [0x3, register, _, _] => SkipIfRegEqConstant {
                register: register as usize,
                constant: (opcode & 0x00ff) as u8,
            },
            [0x4, register, _, _] => SkipIfRegNotEqConstant {
                register: register as usize,
                constant: (opcode & 0x00ff) as u8,
            },
            [0x5, register_lhs, register_rhs, 0] => SkipIfRegEqReg {
                register_lhs: register_lhs as usize,
                register_rhs: register_rhs as usize,
            },
            [0x6, register, _, _] => SetRegToConstant {
                register: register as usize,
                constant: (opcode & 0x00ff) as u8,
            },
            [0x7, register, _, _] => AddConstToReg {
                register: register as usize,
                constant: (opcode & 0x00ff) as u8,
            },
            [0x8, register_lhs, register_rhs, 0] => SetRegToReg {
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
            [0xE, register, 0x9, 0xE] => SkipIfKeyPressed {
                register: register as usize,
            },
            [0xE, register, 0xA, 0x1] => SkipIfKeyNotPressed {
                register: register as usize,
            },
            [0xF, register, 0x0, 0x7] => SetRegToDelayTimer {
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
            [0xF, register, 0x2, 0x9] => SetAddressOfFontChar {
                register: register as usize,
            },
            [0xF, register, 0x3, 0x3] => StoreRegBcd {
                register: register as usize,
            },
            [0xF, register, 0x5, 0x5] => StoreRegisters {
                last_register: register as usize,
            },
            [0xF, register, 0x6, 0x5] => LoadRegisters {
                last_register: register as usize,
            },
            _ => Unknown { opcode },
        }
    }

    #[allow(dead_code)]
    fn to_opcode(&self) -> u16 {
        use Instruction::*;
        let opcode = match self {
            ClearDisplay => 0x00E0,
            Return => 0x00EE,
            Jump { address } => 0x1000 | address,
            Call { address } => 0x2000 | address,
            SkipIfRegEqConstant { register, constant } => {
                0x3000 | ((*register as u16) << 8) | (*constant as u16)
            }
            SkipIfRegNotEqConstant { register, constant } => {
                0x4000 | ((*register as u16) << 8) | (*constant as u16)
            }
            SkipIfRegEqReg {
                register_lhs,
                register_rhs,
            } => 0x5000 | ((*register_lhs as u16) << 8) | ((*register_rhs as u16) << 4),
            SetRegToConstant { register, constant } => {
                0x6000 | ((*register as u16) << 8) | (*constant as u16)
            }
            AddConstToReg { register, constant } => {
                0x7000 | ((*register as u16) << 8) | (*constant as u16)
            }
            SetRegToReg {
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
                0xD000 | ((*register_x as u16) << 8) | ((*register_y as u16) << 4) | *n_bytes as u16
            }
            SkipIfKeyPressed { register } => 0xE09E | ((*register as u16) << 8),
            SkipIfKeyNotPressed { register } => 0xE0A1 | ((*register as u16) << 8),
            SetRegToDelayTimer { register } => 0xF007 | ((*register as u16) << 8),
            AwaitAndSetKeyPress { register } => 0xF00A | ((*register as u16) << 8),
            SetDelayTimer { register } => 0xF015 | ((*register as u16) << 8),
            SetSoundTimer { register } => 0xF018 | ((*register as u16) << 8),
            AddRegToAddressWithoutCarry { register } => 0xF01E | ((*register as u16) << 8),
            SetAddressOfFontChar { register } => 0xF029 | ((*register as u16) << 8),
            StoreRegBcd { register } => 0xF033 | ((*register as u16) << 8),
            StoreRegisters { last_register } => 0xF055 | ((*last_register as u16) << 8),
            LoadRegisters { last_register } => 0xF065 | ((*last_register as u16) << 8),

            Unknown { opcode } => *opcode,
        };
        return opcode;
    }
}

enum InstructionExecuteStatus {
    Complete,
    InProgress,
}

impl Emulator {
    pub fn new() -> Emulator {
        let mut emulator = Emulator {
            cpu: Cpu {
                registers: [0; 16],
                register_i: 0,
                program_counter: 512,
                stack: [0; 16],
                stack_index: -1,
                delay_timer: 0,
                sound_timer: 0,
            },
            memory: [0; MEMORY_SIZE],
            active_pixels: HashSet::new(),
            input: [false; 16],
            cpu_timer: Duration::MAX,
            sound_timer: Duration::ZERO,
            delay_timer: Duration::ZERO,
        };

        fn load_font_sprites(memory: &mut [u8; MEMORY_SIZE]) {
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

        load_font_sprites(&mut emulator.memory);
        emulator
    }

    #[allow(dead_code)]
    fn load_instructions(&mut self, instructions: Vec<Instruction>) {
        let mut data: Vec<u8> = Vec::new();
        for instruction in instructions {
            let opcode = instruction.to_opcode();
            data.push(((opcode & 0xFF00) >> 8) as u8);
            data.push((opcode & 0x00FF) as u8);
        }
        self.load_program_from_data(&data);
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

    pub fn step(&mut self, elapsed_time: Duration) {
        self.cpu_timer = self.cpu_timer.saturating_add(elapsed_time);
        self.delay_timer = self.delay_timer.saturating_add(elapsed_time);
        self.sound_timer = self.sound_timer.saturating_add(elapsed_time);

        if self.delay_timer >= Duration::from_millis(16) {
            self.cpu.delay_timer = self.cpu.delay_timer.saturating_sub(1);
            self.delay_timer = Duration::ZERO;
        }

        if self.sound_timer >= Duration::from_millis(16) {
            self.cpu.sound_timer = self.cpu.sound_timer.saturating_sub(1);
            self.sound_timer = Duration::ZERO;
        }

        if self.cpu_timer >= Duration::from_millis(2) {
            let opcode = self.fetch_opcode().unwrap();
            let instruction = Instruction::decode(opcode);
            match self.execute(instruction) {
                InstructionExecuteStatus::Complete => self.cpu_timer = Duration::ZERO,
                InstructionExecuteStatus::InProgress => {}
            }
        }
    }

    fn fetch_opcode(&mut self) -> Option<u16> {
        let opcode = u16::from_be_bytes([
            self.memory[self.cpu.program_counter as usize],
            self.memory[(self.cpu.program_counter + 1) as usize],
        ]);

        return Some(opcode);
    }

    fn execute(&mut self, instruction: Instruction) -> InstructionExecuteStatus {
        self.cpu.program_counter += 2;

        use Instruction::*;
        match instruction {
            ClearDisplay => {
                self.active_pixels.clear();
            }
            Return => {
                self.cpu.program_counter = self.cpu.stack[self.cpu.stack_index as usize];
                self.cpu.stack_index -= 1;
            }
            Jump { address } => self.cpu.program_counter = address,
            Call { address } => {
                self.cpu.stack_index += 1;
                self.cpu.stack[self.cpu.stack_index as usize] = self.cpu.program_counter;
                self.cpu.program_counter = address;
            }
            SkipIfRegEqConstant { register, constant } => {
                if self.cpu.registers[register] == constant {
                    self.cpu.program_counter += 2;
                }
            }
            SkipIfRegNotEqConstant { register, constant } => {
                if self.cpu.registers[register] != constant {
                    self.cpu.program_counter += 2;
                }
            }
            SkipIfRegEqReg {
                register_lhs,
                register_rhs,
            } => {
                if self.cpu.registers[register_lhs] == self.cpu.registers[register_rhs] {
                    self.cpu.program_counter += 2;
                }
            }
            SetRegToConstant { register, constant } => self.cpu.registers[register] = constant,
            AddConstToReg { register, constant } => {
                self.cpu.registers[register] = self.cpu.registers[register].wrapping_add(constant);
            }
            SetRegToReg {
                register_lhs,
                register_rhs,
            } => self.cpu.registers[register_lhs] = self.cpu.registers[register_rhs],
            BitwiseOr {
                register_lhs,
                register_rhs,
            } => self.cpu.registers[register_lhs] |= self.cpu.registers[register_rhs],
            BitwiseAnd {
                register_lhs,
                register_rhs,
            } => self.cpu.registers[register_lhs] &= self.cpu.registers[register_rhs],
            BitwiseXor {
                register_lhs,
                register_rhs,
            } => self.cpu.registers[register_lhs] ^= self.cpu.registers[register_rhs],
            AddRegToReg {
                register_lhs,
                register_rhs,
            } => {
                let result = self.cpu.registers[register_lhs]
                    .overflowing_add(self.cpu.registers[register_rhs]);
                match result {
                    (sum, overflow) => {
                        self.cpu.registers[register_lhs as usize] = sum;
                        self.cpu.registers[0xF] = overflow as u8;
                    }
                }
            }
            SubReg2FromReg1 {
                register_lhs,
                register_rhs,
            } => {
                let result = self.cpu.registers[register_lhs]
                    .overflowing_sub(self.cpu.registers[register_rhs]);
                match result {
                    (sub, overflow) => {
                        self.cpu.registers[register_lhs] = sub;
                        self.cpu.registers[0xF] = !overflow as u8;
                    }
                }
            }
            BitwiseShrBy1 { register } => {
                self.cpu.registers[0xF] = self.cpu.registers[register] % 2;
                self.cpu.registers[register] /= 2;
            }
            SubReg1FromReg2 {
                register_lhs,
                register_rhs,
            } => {
                let result = self.cpu.registers[register_rhs]
                    .overflowing_sub(self.cpu.registers[register_lhs]);
                match result {
                    (sub, overflow) => {
                        self.cpu.registers[register_lhs] = sub;
                        self.cpu.registers[0xF] = !overflow as u8;
                    }
                }
            }
            BitwiseShlBy1 { register } => {
                let result = self.cpu.registers[register].overflowing_mul(2);
                match result {
                    (mul, overflow) => {
                        self.cpu.registers[register] = mul;
                        self.cpu.registers[0xF] = overflow as u8;
                    }
                }
            }
            CondRegNotEqReg {
                register_lhs,
                register_rhs,
            } => {
                if self.cpu.registers[register_lhs] != self.cpu.registers[register_rhs] {
                    self.cpu.program_counter += 2;
                }
            }
            SetAddress { address } => self.cpu.register_i = address,
            JumpWithV0Offset { address } => {
                self.cpu.program_counter = self.cpu.registers[0] as u16 + address
            }
            BitwiseAndWithRand { register, constant } => {
                let mut rng = rand::thread_rng();
                let random_number: u8 = rng.gen();
                self.cpu.registers[register] = constant & random_number;
            }
            DisplaySprite {
                register_x,
                register_y,
                n_bytes,
            } => {
                let origin_x = self.cpu.registers[register_x] as u32 % SCREEN_WIDTH;
                let origin_y = self.cpu.registers[register_y] as u32 % SCREEN_HEIGHT;
                let mut pixels = Vec::new();
                for i in 0..n_bytes {
                    let sprite = self.memory[self.cpu.register_i as usize + i as usize];
                    let mut mask = 0b10000000;
                    for j in 0..8 {
                        let (pixel_x, pixel_y) = (origin_x + j, origin_y + i as u32);
                        if (pixel_x >= SCREEN_WIDTH) || (pixel_y >= SCREEN_HEIGHT) {
                            break;
                        }
                        if sprite & mask != 0 {
                            pixels.push((pixel_x, pixel_y));
                        }
                        mask >>= 1;
                    }
                }

                let mut xored = false;
                if !pixels.is_empty() {
                    xored = self.draw_pixels(&pixels);
                }

                if xored {
                    self.cpu.registers[0xF] = 1;
                } else {
                    self.cpu.registers[0xF] = 0;
                }
            }
            SkipIfKeyPressed { register } => {
                let key = self.cpu.registers[register];
                if self.input[key as usize] {
                    self.cpu.program_counter += 2;
                }
            }
            SkipIfKeyNotPressed { register } => {
                let key = self.cpu.registers[register];
                if !self.input[key as usize] {
                    self.cpu.program_counter += 2;
                }
            }
            SetRegToDelayTimer { register } => self.cpu.registers[register] = self.cpu.delay_timer,
            AwaitAndSetKeyPress { register } => {
                let mut key_pressed = false;
                for (i, input) in self.input.iter().enumerate() {
                    if *input {
                        self.cpu.registers[register] = i as u8;
                        key_pressed = true;
                        break;
                    }
                }
                if !key_pressed {
                    self.cpu.program_counter -= 2;
                    return InstructionExecuteStatus::InProgress;
                }
            }
            SetDelayTimer { register } => self.cpu.delay_timer = self.cpu.registers[register],
            SetSoundTimer { register } => self.cpu.sound_timer = self.cpu.registers[register],
            AddRegToAddressWithoutCarry { register } => {
                self.cpu.register_i += self.cpu.registers[register] as u16
            }
            SetAddressOfFontChar { register } => {
                let character = self.cpu.registers[register];
                self.cpu.register_i = match character {
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
                    _ => self.cpu.register_i,
                }
            }
            StoreRegBcd { register } => {
                let mut value = self.cpu.registers[register];
                self.memory[(self.cpu.register_i + 2) as usize] = value % 10;
                value /= 10;
                self.memory[(self.cpu.register_i + 1) as usize] = value % 10;
                value /= 10;
                self.memory[(self.cpu.register_i + 0) as usize] = value % 10;
            }
            StoreRegisters { last_register } => {
                for i in 0..=last_register {
                    self.memory[self.cpu.register_i as usize + i] = self.cpu.registers[i];
                }
            }
            LoadRegisters { last_register } => {
                for i in 0..=last_register {
                    self.cpu.registers[i] = self.memory[self.cpu.register_i as usize + i];
                }
            }

            Unknown { opcode } => {
                println!("Unknown instruction: {:#06x}", opcode)
            }
        }

        return InstructionExecuteStatus::Complete;
    }

    fn draw_pixels(&mut self, pixels: &[(u32, u32)]) -> bool {
        let mut xored = false;
        for pixel in pixels.iter() {
            if self.active_pixels.contains(pixel) {
                self.active_pixels.remove(pixel);
                xored = true;
            } else {
                self.active_pixels.insert(*pixel);
            }
        }
        return xored;
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
        assert_eq_hex!(SkipIfRegEqConstant{register: 0xA, constant: 0xC3}.to_opcode(), 0x3AC3);
        assert_eq_hex!(SkipIfRegNotEqConstant{register: 1, constant: 0x23}.to_opcode(), 0x4123);
        assert_eq_hex!(SkipIfRegEqReg{register_lhs: 0xA, register_rhs: 0xD}.to_opcode(), 0x5AD0);
        assert_eq_hex!(SetRegToConstant{register: 7, constant: 0xAF}.to_opcode(), 0x67AF);
        assert_eq_hex!(AddConstToReg{register: 0xC, constant: 0x42}.to_opcode(), 0x7C42);
        assert_eq_hex!(SetRegToReg{register_lhs: 0x9, register_rhs: 0x3}.to_opcode(), 0x8930);
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
        assert_eq_hex!(SkipIfKeyPressed{register: 0x5}.to_opcode(), 0xE59E);
        assert_eq_hex!(SkipIfKeyNotPressed{register: 0x5}.to_opcode(), 0xE5A1);
        assert_eq_hex!(SetRegToDelayTimer{register: 0x5}.to_opcode(), 0xF507);
        assert_eq_hex!(AwaitAndSetKeyPress{register: 0x5}.to_opcode(), 0xF50A);
        assert_eq_hex!(SetDelayTimer{register: 0x3}.to_opcode(), 0xF315);
        assert_eq_hex!(SetSoundTimer{register: 0x3}.to_opcode(), 0xF318);
        assert_eq_hex!(AddRegToAddressWithoutCarry{register: 0x5}.to_opcode(), 0xF51E);
        assert_eq_hex!(SetAddressOfFontChar{register: 0x5}.to_opcode(), 0xF529);
        assert_eq_hex!(StoreRegBcd{register: 0x7}.to_opcode(), 0xF733);
        assert_eq_hex!(StoreRegisters{last_register: 0x7}.to_opcode(), 0xF755);
        assert_eq_hex!(LoadRegisters{last_register: 0x7}.to_opcode(), 0xF765);
    }

    #[test]
    fn should_execute_clear_display() {
        use Instruction::*;

        // Given
        let mut emulator = Emulator::new();
        emulator.active_pixels.extend([(1, 1), (10, 15), (21, 30)]);

        // When
        emulator.execute(ClearDisplay);

        // Then
        assert_eq!(emulator.active_pixels.len(), 0);
    }

    #[test]
    fn should_execute_jump() {
        use Instruction::*;

        // Given
        let mut emulator = Emulator::new();

        // When
        emulator.execute(Jump { address: 0x123 });

        // Then
        assert_eq_hex!(emulator.cpu.program_counter, 0x123);
    }

    #[test]
    fn should_execute_call() {
        use Instruction::*;

        // Given
        let mut emulator = Emulator::new();
        let pc = emulator.cpu.program_counter;

        // When
        emulator.execute(Call { address: 0x123 });

        // Then
        assert_eq_hex!(emulator.cpu.program_counter, 0x123);
        assert_eq!(emulator.cpu.stack_index, 0);
        assert_eq!(
            emulator.cpu.stack[emulator.cpu.stack_index as usize],
            pc + 2
        );
    }

    #[test]
    fn should_execute_return() {
        use Instruction::*;

        // Given
        let mut emulator = Emulator::new();
        emulator.cpu.stack[0] = 0x123;
        emulator.cpu.stack_index = 0;

        // When
        emulator.execute(Return);

        // Then
        assert_eq_hex!(emulator.cpu.program_counter, 0x123);
        assert_eq!(emulator.cpu.stack_index, -1);
    }

    #[test]
    fn should_execute_skip_if_req_eq_constant() {
        use Instruction::*;

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.cpu.registers[0x3] = 0x7d;
            let pc = emulator.cpu.program_counter;

            // When
            emulator.execute(SkipIfRegEqConstant {
                register: 0x3,
                constant: 0x7d,
            });

            // Then
            assert_eq!(emulator.cpu.program_counter, pc + 4);
        }

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.cpu.registers[0x3] = 0x6c;
            let pc = emulator.cpu.program_counter;

            // When
            emulator.execute(SkipIfRegEqConstant {
                register: 0x3,
                constant: 0x7d,
            });

            // Then
            assert_eq!(emulator.cpu.program_counter, pc + 2);
        }
    }

    #[test]
    fn should_execute_skip_if_req_not_eq_constant() {
        use Instruction::*;

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.cpu.registers[0x3] = 0x7d;
            let pc = emulator.cpu.program_counter;

            // When
            emulator.execute(SkipIfRegNotEqConstant {
                register: 0x3,
                constant: 0x7d,
            });

            // Then
            assert_eq!(emulator.cpu.program_counter, pc + 2);
        }

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.cpu.registers[0x3] = 0x6c;
            let pc = emulator.cpu.program_counter;

            // When
            emulator.execute(SkipIfRegNotEqConstant {
                register: 0x3,
                constant: 0x7d,
            });

            // Then
            assert_eq!(emulator.cpu.program_counter, pc + 4);
        }
    }

    #[test]
    fn should_execute_skip_if_req_eq_req() {
        use Instruction::*;

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.cpu.registers[0x3] = 0x42;
            emulator.cpu.registers[0x5] = 0x42;
            let pc = emulator.cpu.program_counter;

            // When
            emulator.execute(SkipIfRegEqReg {
                register_lhs: 0x3,
                register_rhs: 0x5,
            });

            // Then
            assert_eq!(emulator.cpu.program_counter, pc + 4);
        }

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.cpu.registers[0x3] = 0x42;
            emulator.cpu.registers[0x5] = 0x71;
            let pc = emulator.cpu.program_counter;

            // When
            emulator.execute(SkipIfRegEqReg {
                register_lhs: 0x3,
                register_rhs: 0x5,
            });

            // Then
            assert_eq!(emulator.cpu.program_counter, pc + 2);
        }
    }

    #[test]
    fn should_execute_skip_if_key_pressed() {
        use Instruction::*;

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.input[0xA] = true;
            emulator.cpu.registers[0x3] = 0xA;
            let pc = emulator.cpu.program_counter;

            // When
            emulator.execute(SkipIfKeyPressed { register: 0x3 });

            // Then
            assert_eq!(emulator.cpu.program_counter, pc + 4);
        }

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.input[0xA] = false;
            emulator.cpu.registers[0x3] = 0xA;
            let pc = emulator.cpu.program_counter;

            // When
            emulator.execute(SkipIfKeyPressed { register: 0x3 });

            // Then
            assert_eq!(emulator.cpu.program_counter, pc + 2);
        }
    }

    #[test]
    fn should_execute_skip_if_not_key_pressed() {
        use Instruction::*;

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.input[0xA] = true;
            emulator.cpu.registers[0x3] = 0xA;
            let pc = emulator.cpu.program_counter;

            // When
            emulator.execute(SkipIfKeyNotPressed { register: 0x3 });

            // Then
            assert_eq!(emulator.cpu.program_counter, pc + 2);
        }

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.input[0xA] = false;
            emulator.cpu.registers[0x3] = 0xA;
            let pc = emulator.cpu.program_counter;

            // When
            emulator.execute(SkipIfKeyNotPressed { register: 0x3 });

            // Then
            assert_eq!(emulator.cpu.program_counter, pc + 4);
        }
    }

    #[test]
    fn should_execute_set_reg_to_constant() {
        use Instruction::*;

        // Given
        let mut emulator = Emulator::new();
        emulator.cpu.registers[0x4] = 0x42;

        // When
        emulator.execute(SetRegToConstant {
            register: 0x4,
            constant: 0xD7,
        });

        // Then
        assert_eq_hex!(emulator.cpu.registers[0x4], 0xD7);
    }

    #[test]
    fn should_execute_add_const_to_reg() {
        use Instruction::*;

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.cpu.registers[0x4] = 0x27;

            // When
            emulator.execute(AddConstToReg {
                register: 0x4,
                constant: 0xD7,
            });

            // Then
            assert_eq_hex!(emulator.cpu.registers[0x4], 0x27 + 0xD7);
            assert_eq_hex!(emulator.cpu.registers[0xF], 0);
        }

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.cpu.registers[0x4] = 0xff;

            // When
            emulator.execute(AddConstToReg {
                register: 0x4,
                constant: 0x01,
            });

            // Then
            assert_eq_hex!(emulator.cpu.registers[0x4], 0x0);
            assert_eq_hex!(emulator.cpu.registers[0xF], 0);
        }
    }

    #[test]
    fn should_execute_set_reg_to_reg() {
        use Instruction::*;

        // Given
        let mut emulator = Emulator::new();
        emulator.cpu.registers[0x3] = 0x42;
        emulator.cpu.registers[0xa] = 0xd5;

        // When
        emulator.execute(SetRegToReg {
            register_lhs: 0x3,
            register_rhs: 0xa,
        });

        // Then
        assert_eq_hex!(emulator.cpu.registers[0x3], 0xd5);
    }

    #[test]
    fn should_execute_bitwise_or() {
        use Instruction::*;

        // Given
        let mut emulator = Emulator::new();
        emulator.cpu.registers[0x3] = 0x42;
        emulator.cpu.registers[0xa] = 0xd5;

        // When
        emulator.execute(BitwiseOr {
            register_lhs: 0x3,
            register_rhs: 0xa,
        });

        // Then
        assert_eq_hex!(emulator.cpu.registers[0x3], 0x42 | 0xd5);
    }

    #[test]
    fn should_execute_bitwise_and() {
        use Instruction::*;

        // Given
        let mut emulator = Emulator::new();
        emulator.cpu.registers[0x3] = 0x42;
        emulator.cpu.registers[0xa] = 0xd5;

        // When
        emulator.execute(BitwiseAnd {
            register_lhs: 0x3,
            register_rhs: 0xa,
        });

        // Then
        assert_eq_hex!(emulator.cpu.registers[0x3], 0x42 & 0xd5);
    }

    #[test]
    fn should_execute_bitwise_xor() {
        use Instruction::*;

        // Given
        let mut emulator = Emulator::new();
        emulator.cpu.registers[0x3] = 0x42;
        emulator.cpu.registers[0xa] = 0xd5;

        // When
        emulator.execute(BitwiseXor {
            register_lhs: 0x3,
            register_rhs: 0xa,
        });

        // Then
        assert_eq_hex!(emulator.cpu.registers[0x3], 0x42 ^ 0xd5);
    }

    #[test]
    fn should_execute_add_reg_to_reg() {
        use Instruction::*;

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.cpu.registers[0x3] = 0x42;
            emulator.cpu.registers[0xa] = 0x65;

            // When
            emulator.execute(AddRegToReg {
                register_lhs: 0x3,
                register_rhs: 0xa,
            });

            // Then
            assert_eq_hex!(emulator.cpu.registers[0x3], 0x42 + 0x65);
            assert_eq!(emulator.cpu.registers[0xF], 0);
        }

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.cpu.registers[0x3] = 0xff;
            emulator.cpu.registers[0xa] = 0x1;

            // When
            emulator.execute(AddRegToReg {
                register_lhs: 0x3,
                register_rhs: 0xa,
            });

            // Then
            assert_eq_hex!(emulator.cpu.registers[0x3], 0x0);
            assert_eq!(emulator.cpu.registers[0xF], 1);
        }
    }

    #[test]
    fn should_execute_sub_reg2_from_reg1() {
        use Instruction::*;

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.cpu.registers[0x3] = 0x65;
            emulator.cpu.registers[0x4] = 0x42;

            // When
            emulator.execute(SubReg2FromReg1 {
                register_lhs: 0x3,
                register_rhs: 0x4,
            });

            // Then
            assert_eq_hex!(emulator.cpu.registers[0x3], 0x65 - 0x42);
            assert_eq!(emulator.cpu.registers[0xF], 1);
        }

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.cpu.registers[0x3] = 0x0;
            emulator.cpu.registers[0x4] = 0x4;

            // When
            emulator.execute(SubReg2FromReg1 {
                register_lhs: 0x3,
                register_rhs: 0x4,
            });

            // Then
            assert_eq_hex!(emulator.cpu.registers[0x3], 0xfc);
            assert_eq!(emulator.cpu.registers[0xF], 0);
        }
    }

    #[test]
    fn should_execute_sub_reg1_from_reg2() {
        use Instruction::*;

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.cpu.registers[0x3] = 0x42;
            emulator.cpu.registers[0x4] = 0x65;

            // When
            emulator.execute(SubReg1FromReg2 {
                register_lhs: 0x3,
                register_rhs: 0x4,
            });

            // Then
            assert_eq_hex!(emulator.cpu.registers[0x3], 0x65 - 0x42);
            assert_eq!(emulator.cpu.registers[0xF], 1);
        }

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.cpu.registers[0x3] = 0x7;
            emulator.cpu.registers[0x4] = 0x0;

            // When
            emulator.execute(SubReg1FromReg2 {
                register_lhs: 0x3,
                register_rhs: 0x4,
            });

            // Then
            assert_eq_hex!(emulator.cpu.registers[0x3], 0xf9);
            assert_eq!(emulator.cpu.registers[0xF], 0);
        }
    }

    #[test]
    fn should_execute_bitwise_shr_by_1() {
        use Instruction::*;

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.cpu.registers[0x3] = 0b11001101;

            // When
            emulator.execute(BitwiseShrBy1 { register: 0x3 });

            // Then
            assert_eq_hex!(emulator.cpu.registers[0x3], 0b1100110);
            assert_eq!(emulator.cpu.registers[0xF], 1);
        }

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.cpu.registers[0x3] = 0b10001110;

            // When
            emulator.execute(BitwiseShrBy1 { register: 0x3 });

            // Then
            assert_eq_hex!(emulator.cpu.registers[0x3], 0b1000111);
            assert_eq!(emulator.cpu.registers[0xF], 0);
        }
    }

    #[test]
    fn should_execute_bitwise_shl_by_1() {
        use Instruction::*;

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.cpu.registers[0x3] = 0b1101;

            // When
            emulator.execute(BitwiseShlBy1 { register: 0x3 });

            // Then
            assert_eq_hex!(emulator.cpu.registers[0x3], 0b11010);
            assert_eq!(emulator.cpu.registers[0xF], 0);
        }

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.cpu.registers[0x3] = 0b11001110;

            // When
            emulator.execute(BitwiseShlBy1 { register: 0x3 });

            // Then
            assert_eq_hex!(emulator.cpu.registers[0x3], 0b10011100);
            assert_eq!(emulator.cpu.registers[0xF], 1);
        }
    }

    #[test]
    fn should_execute_set_address() {
        use Instruction::*;

        // Given
        let mut emulator = Emulator::new();
        emulator.cpu.register_i = 0x0;

        // When
        emulator.execute(SetAddress { address: 0x456 });

        // Then
        assert_eq_hex!(emulator.cpu.register_i, 0x456);
    }

    #[test]
    fn should_execute_jump_with_v0_offset() {
        use Instruction::*;

        // Given
        let mut emulator = Emulator::new();
        emulator.cpu.registers[0] = 0xff;

        // When
        emulator.execute(JumpWithV0Offset { address: 0x456 });

        // Then
        assert_eq_hex!(
            emulator.cpu.program_counter,
            0x456 + emulator.cpu.registers[0] as u16
        );
    }

    #[test]
    fn should_execute_display_sprite_no_xor() {
        use Instruction::*;

        // Given
        let mut emulator = Emulator::new();
        emulator.cpu.registers[2] = 20;
        emulator.cpu.registers[3] = 10;
        emulator.cpu.register_i = 0x600;
        emulator.memory[0x600] = 0b10101010;
        emulator.memory[0x601] = 0b00111010;

        // When
        emulator.execute(DisplaySprite {
            register_x: 2,
            register_y: 3,
            n_bytes: 2,
        });

        // Then
        assert_eq!(emulator.active_pixels.len(), 8);
        assert!(emulator.active_pixels.contains(&(20, 10)));
        assert!(emulator.active_pixels.contains(&(22, 10)));
        assert!(emulator.active_pixels.contains(&(24, 10)));
        assert!(emulator.active_pixels.contains(&(26, 10)));
        assert!(emulator.active_pixels.contains(&(22, 11)));
        assert!(emulator.active_pixels.contains(&(23, 11)));
        assert!(emulator.active_pixels.contains(&(24, 11)));
        assert!(emulator.active_pixels.contains(&(26, 11)));
        assert_eq!(emulator.cpu.registers[0xF], 0);
    }

    #[test]
    fn should_execute_display_sprite_xor() {
        use Instruction::*;

        // Given
        let mut emulator = Emulator::new();
        emulator.cpu.registers[2] = 20;
        emulator.cpu.registers[3] = 10;
        emulator.cpu.register_i = 0x600;
        emulator.memory[0x600] = 0b10101010;
        emulator.active_pixels.insert((22, 10));
        emulator.active_pixels.insert((26, 10));

        // When
        emulator.execute(DisplaySprite {
            register_x: 2,
            register_y: 3,
            n_bytes: 1,
        });

        // Then
        assert_eq!(emulator.active_pixels.len(), 2);
        assert!(emulator.active_pixels.contains(&(20, 10)));
        assert!(emulator.active_pixels.contains(&(24, 10)));
        assert_eq!(emulator.cpu.registers[0xF], 1);
    }

    #[test]
    fn should_execute_display_sprite_near_edge() {
        use Instruction::*;

        // Given
        let mut emulator = Emulator::new();
        emulator.cpu.registers[2] = (SCREEN_WIDTH - 3) as u8;
        emulator.cpu.registers[3] = (SCREEN_HEIGHT - 1) as u8;
        emulator.cpu.register_i = 0x600;
        emulator.memory[0x600] = 0b10101010;
        emulator.memory[0x601] = 0b01101011;

        // When
        emulator.execute(DisplaySprite {
            register_x: 2,
            register_y: 3,
            n_bytes: 2,
        });

        // Then
        assert_eq!(emulator.active_pixels.len(), 2);
        assert!(emulator
            .active_pixels
            .contains(&(SCREEN_WIDTH - 3, SCREEN_HEIGHT - 1)));
        assert!(emulator
            .active_pixels
            .contains(&(SCREEN_WIDTH - 1, SCREEN_HEIGHT - 1)));
        assert_eq!(emulator.cpu.registers[0xF], 0);
    }

    #[test]
    fn should_execute_display_sprite_wrap() {
        use Instruction::*;

        // Given
        let mut emulator = Emulator::new();
        emulator.cpu.registers[2] = (7 * SCREEN_WIDTH + 5) as u8;
        emulator.cpu.registers[3] = (2 * SCREEN_HEIGHT + 10) as u8;
        emulator.cpu.register_i = 0x600;
        emulator.memory[0x600] = 0b10000010;
        emulator.memory[0x601] = 0b01001001;

        // When
        emulator.execute(DisplaySprite {
            register_x: 2,
            register_y: 3,
            n_bytes: 2,
        });

        // Then
        assert_eq!(emulator.active_pixels.len(), 5);
        assert!(emulator.active_pixels.contains(&(5, 10)));
        assert!(emulator.active_pixels.contains(&(11, 10)));
        assert!(emulator.active_pixels.contains(&(6, 11)));
        assert!(emulator.active_pixels.contains(&(9, 11)));
        assert!(emulator.active_pixels.contains(&(12, 11)));
        assert_eq!(emulator.cpu.registers[0xF], 0);
    }

    #[test]
    fn should_execute_set_reg_to_delay_timer() {
        use Instruction::*;

        // Given
        let mut emulator = Emulator::new();
        emulator.cpu.delay_timer = 42;

        // When
        emulator.execute(SetRegToDelayTimer { register: 0x3 });

        // Then
        assert_eq_hex!(emulator.cpu.registers[0x3], 42);
    }

    #[test]
    fn should_execute_await_and_set_key_press() {
        use Instruction::*;

        // Given
        let mut emulator = Emulator::new();
        emulator.input[0xC] = true;

        // When
        emulator.execute(AwaitAndSetKeyPress { register: 0x3 });

        // Then
        assert_eq_hex!(emulator.cpu.registers[0x3], 0xC);
    }

    #[test]
    fn should_execute_await_and_set_key_press_with_no_delay() {
        use Instruction::*;

        // Given
        let mut emulator = Emulator::new();

        // When
        emulator.input[0xC] = false;
        emulator.load_instructions(vec![AwaitAndSetKeyPress { register: 0x3 }]);
        emulator.step(Duration::from_nanos(1));
        emulator.input[0xC] = true;
        emulator.step(Duration::from_nanos(1));

        // Then
        assert_eq_hex!(emulator.cpu.registers[0x3], 0xC);
    }

    #[test]
    fn should_execute_await_and_set_delay_timer() {
        use Instruction::*;

        // Given
        let mut emulator = Emulator::new();
        emulator.cpu.registers[0x3] = 0x7d;

        // When
        emulator.execute(SetDelayTimer { register: 0x3 });

        // Then
        assert_eq!(emulator.cpu.delay_timer, 0x7d);
    }

    #[test]
    fn should_execute_await_and_set_sound_timer() {
        use Instruction::*;

        // Given
        let mut emulator = Emulator::new();
        emulator.cpu.registers[0x3] = 0x7d;

        // When
        emulator.execute(SetSoundTimer { register: 0x3 });

        // Then
        assert_eq!(emulator.cpu.sound_timer, 0x7d);
    }

    #[test]
    fn should_execute_add_reg_to_address_without_carry() {
        use Instruction::*;

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.cpu.register_i = 0xd79;
            emulator.cpu.registers[0x3] = 0x7d;

            // When
            emulator.execute(AddRegToAddressWithoutCarry { register: 0x3 });

            // Then
            assert_eq_hex!(emulator.cpu.register_i, 0xd79 + 0x7d);
            assert_eq!(emulator.cpu.registers[0xF], 0);
        }

        {
            // Given
            let mut emulator = Emulator::new();
            emulator.cpu.register_i = 0xf79;
            emulator.cpu.registers[0x3] = 0x7d;

            // When
            emulator.execute(AddRegToAddressWithoutCarry { register: 0x3 });

            // Then
            assert_eq_hex!(emulator.cpu.register_i, 0xf79 + 0x7d);
            assert_eq!(emulator.cpu.registers[0xF], 0);
        }
    }

    #[test]
    fn should_execute_store_reg_bcd() {
        use Instruction::*;

        // Given
        let mut emulator = Emulator::new();
        emulator.cpu.registers[0x3] = 196;
        emulator.cpu.register_i = 0x765;

        // When
        emulator.execute(StoreRegBcd { register: 0x3 });

        // Then
        assert_eq!(emulator.memory[emulator.cpu.register_i as usize + 0], 1);
        assert_eq!(emulator.memory[emulator.cpu.register_i as usize + 1], 9);
        assert_eq!(emulator.memory[emulator.cpu.register_i as usize + 2], 6);
    }

    #[test]
    fn should_execute_store_registers() {
        use Instruction::*;

        // Given
        let mut emulator = Emulator::new();
        emulator.cpu.registers[0x0] = 0x41;
        emulator.cpu.registers[0x1] = 0xb7;
        emulator.cpu.registers[0x2] = 0x09;
        emulator.cpu.registers[0x3] = 0xff;
        emulator.cpu.register_i = 0x765;

        // When
        emulator.execute(StoreRegisters { last_register: 0x2 });

        // Then
        assert_eq_hex!(emulator.memory[emulator.cpu.register_i as usize + 0], 0x41);
        assert_eq_hex!(emulator.memory[emulator.cpu.register_i as usize + 1], 0xb7);
        assert_eq_hex!(emulator.memory[emulator.cpu.register_i as usize + 2], 0x09);
        assert_eq_hex!(emulator.memory[emulator.cpu.register_i as usize + 3], 0);
    }

    #[test]
    fn should_execute_load_registers() {
        use Instruction::*;

        // Given
        let mut emulator = Emulator::new();
        emulator.cpu.registers[0x0] = 0xff;
        emulator.cpu.registers[0x1] = 0xff;
        emulator.cpu.registers[0x2] = 0xff;
        emulator.cpu.registers[0x3] = 0xff;
        emulator.cpu.register_i = 0x765;
        emulator.memory[emulator.cpu.register_i as usize + 0] = 0x71;
        emulator.memory[emulator.cpu.register_i as usize + 1] = 0xa5;
        emulator.memory[emulator.cpu.register_i as usize + 2] = 0x06;
        emulator.memory[emulator.cpu.register_i as usize + 3] = 0x51;

        // When
        emulator.execute(LoadRegisters { last_register: 0x2 });

        // Then
        assert_eq_hex!(emulator.cpu.registers[0x0], 0x71);
        assert_eq_hex!(emulator.cpu.registers[0x1], 0xa5);
        assert_eq_hex!(emulator.cpu.registers[0x2], 0x06);
        assert_eq_hex!(emulator.cpu.registers[0x3], 0xff);
    }
}
