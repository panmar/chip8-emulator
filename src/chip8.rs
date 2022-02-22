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
    fn draw_pixel(&mut self, x: u32, y: u32);
    fn update(&mut self);
    fn pending_close(&self) -> bool;
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

    fn execute(&mut self, instruction: Instruction) {
        let cpu = &mut self.cpu;
        let memory = &mut self.memory;
        let platform = &mut self.platform;

        cpu.program_counter += 2;

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
                for i in 0..n_bytes {
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
            CondKeyPressed { register } => {}
            CondKeyNotPressed { register } => {}
            AssignDelayTimerToReg { register } => cpu.registers[register] = cpu.delay_timer,
            AwaitAndSetKeyPress { register } => {}
            SetDelayTimer { register } => cpu.delay_timer = cpu.registers[register],
            SetSoundTimer { register } => cpu.sound_timer = cpu.registers[register],
            AddRegToAddressWithoutCarry { register } => {
                cpu.register_i += cpu.registers[register] as u16
            }
            AssignFontSpriteToAddress { register } => {}
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

            _ => {}
        }
    }

    fn emulation_step(&mut self) {
        let cpu = &mut self.cpu;
        let memory = &mut self.memory;

        let opcode = u16::from_be_bytes([
            memory[cpu.program_counter as usize],
            memory[(cpu.program_counter + 1) as usize],
        ]);

        let instruction = Instruction::parse(opcode);

        print!(
            "[{:#06x}] instruction: {:#06x}",
            cpu.program_counter, opcode
        );
        print!(" ");
        for i in 0..0xf {
            print!("r[{}]={} ", i, cpu.registers[i as usize]);
        }
        println!();

        self.execute(instruction);
    }
}
