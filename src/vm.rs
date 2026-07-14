use std::{fs::File, io::Read};

struct Screen {
    pixels: Vec<Vec<bool>>,
}

impl Screen {
    const CLEAR_TERMINAL: &'static str = "\x1b[2J\x1b[H";

    fn new() -> Screen {
        Screen {
            pixels: vec![vec![false; 64]; 32],
        }
    }

    fn clear(&mut self) {
        for yi in 0..self.pixels.len() {
            for xi in 0..self.pixels[yi].len() {
                self.pixels[yi][xi] = false;
            }
        }
    }

    fn set(&mut self, value: bool, x: usize, y: usize) -> bool {
        let updated = if self.pixels[y][x] != value {
            true
        } else {
            false
        };
        self.pixels[y][x] = value;
        updated
    }

    // return whether or not all of the bits were set to set VF
    fn draw_line(&mut self, value: u8, x: usize, start_y: usize) -> bool {
        let mut all_set: Vec<bool> = vec![];
        for i in 0..8 {
            let is_set = (value & (1 << i)) != 0;
            let was_updated = self.set(is_set, x + (8 - i), start_y);
            all_set.push(was_updated);
        }

        all_set.iter().fold(true, |acc, curr| acc & curr)
    }

    fn draw(&mut self, vals: Vec<u8>, x: usize, start_y: usize) -> bool {
        let mut all_set: Vec<bool> = vec![];
        for i in 0..vals.len() {
            let val = vals[i];
            let set = self.draw_line(val, x, start_y + i);
            all_set.push(set);
        }

        all_set.iter().fold(true, |acc, curr| acc & curr)
    }

    fn render(&self) {
        print!("{}", Self::CLEAR_TERMINAL);
        let mut out = String::new();

        for yi in 0..self.pixels.len() {
            for xi in 0..self.pixels[yi].len() {
                if self.pixels[yi][xi] {
                    out.push('█');
                } else {
                    out.push(' ');
                }
            }
            out.push('\n');
        }

        println!("{}", out);
    }
}

struct Input {
    keys: Vec<bool>,
}

impl Input {
    fn new() -> Input {
        Input {
            keys: vec![false; 16],
        }
    }

    fn set(&mut self, i: usize) {
        self.keys[i] = true;
    }

    fn is_set(&self, i: usize) -> bool {
        self.keys[i]
    }

    fn any_pressed(&self) -> bool {
        for i in self.keys.iter() {
            if *i == true {
                return true;
            }
        }
        false
    }

    // naive scan for first key pressed
    fn get_key_pressed(&self) -> u8 {
        for i in 0..self.keys.len() {
            if self.keys[i] != false {
                return i as u8;
            }
        }
        0
    }
}

struct Memory {
    slots: Vec<u8>,
}

impl Memory {
    const MEMORY_SIZE: usize = 4096;

    fn new() -> Memory {
        Memory {
            slots: vec![0x00; Self::MEMORY_SIZE],
        }
    }

    fn get(&self, address: usize) -> u8 {
        if address <= Self::MEMORY_SIZE {
            return self.slots[address];
        }
        panic!("tried to read memory out of bounds. max address = 4096")
    }

    fn get_block(&self, address: usize, length: usize) -> Vec<u8> {
        self.slots[address..(address + length)].into()
    }

    fn set(&mut self, address: usize, value: u8) {
        if address <= Self::MEMORY_SIZE {
            self.slots[address] = value
        } else {
            panic!("tried to set a memory segment out of bounds. max address = 4096")
        }
    }

    fn dump_memory(&self) {
        let mut counter = 16;
        for i in 0..self.slots.len() {
            counter -= 1;
            let hex_string = hex::encode(vec![self.slots[i]]);
            print!("{} ", hex_string);
            if counter == 0 {
                print!("\n");
                counter = 16;
            }
        }
    }
}

pub struct Vm {
    registers: Vec<u8>,
    address_register: u16,
    program_counter: usize,
    memory: Memory,
    delay_timer: u8,
    sound_timer: u8,
    display: Screen,
    cycle_count: u8, // use this for "entropy source"
    input: Input,
}

impl Vm {
    const PROGRAM_COUNTER_START_POS: usize = 0x200; // 512

    pub fn new() -> Vm {
        Self::vm(Memory::new())
    }

    pub fn new_with_memory(mem: Memory) -> Vm {
        Self::vm(mem)
    }

    fn vm(mem: Memory) -> Vm {
        Vm {
            registers: vec![0x00; 16],
            address_register: 0x000,
            program_counter: Self::PROGRAM_COUNTER_START_POS,
            memory: mem,
            delay_timer: 0,
            sound_timer: 0,
            display: Screen::new(), // where screen is 64 * 32
            cycle_count: 0,
            input: Input::new(),
        }
    }

    pub fn start(&mut self) {
        #[cfg(debug_assertions)]
        {
            self.memory.dump_memory();
        }

        loop {
            // check if there is a key pressed?
            let op = self.get_op(self.program_counter);
            let jump_address = self.execute(op);
            self.update_program_counter(jump_address);

            // handle key pressing here..?

            // check and handle timers here..?

            self.display.render();
            self.cycle_count = self.cycle_count.wrapping_add(1);
        }
    }

    // take a path to a program and put it in memory slots starting at 0x200
    pub fn load(&mut self, path: String) {
        let f = File::open(path).unwrap();
        let mut pos = 0;
        for b in f.bytes() {
            self.memory.set(self.program_counter + pos, b.unwrap());
            pos += 1;
        }
    }

    fn update_program_counter(&mut self, value: Option<usize>) {
        match value {
            Some(val) => self.program_counter = val,
            None => self.program_counter = self.program_counter + 2,
        }
    }

    fn get_op(&self, start_address: usize) -> u16 {
        let ins1 = self.memory.get(start_address);
        let ins2 = self.memory.get(start_address + 1);
        ((ins1 as u16) << 8) | ins2 as u16
    }

    fn execute(&mut self, input: u16) -> Option<usize> {
        let code = get_op_code(input);
        match code {
            0 => {
                match input {
                    0x00E0 => self.display.clear(), // this is clear the display
                    0x00EE => {} // this is return, so we should pop the stack and return that val to set program counter with
                    _ => println!("unknown op in range 0: {}", input),
                }
                None
            }
            1 => Some(get_address(input).into()),  // jump
            2 => self.execute(get_address(input)), // call subroutine at address
            3 => {
                let reg: usize = get_x(input).into();
                let val = get_constant(input);

                if self.registers[reg] == val {
                    // skip the next instruction, so should we incrememnt program counter by 4..?
                    return Some(self.program_counter + 4);
                }
                None
            }
            4 => {
                let reg: usize = get_x(input).into();
                let val = get_constant(input);

                if self.registers[reg] != val {
                    // skip the next instruction, so should we incrememnt program counter by 4..?
                    return Some(self.program_counter + 4);
                }
                None
            }
            5 => {
                let reg1: usize = get_x(input).into();
                let reg2: usize = get_y(input).into();

                if self.registers[reg1] == self.registers[reg2] {
                    return Some(self.program_counter + 4);
                }
                None
            }
            6 => {
                let reg: usize = get_x(input).into();
                let val = get_constant(input);

                self.registers[reg] = val;
                None
            }
            7 => {
                let reg: usize = get_x(input).into();
                let addr = get_constant(input);

                self.registers[reg] = self.registers[reg] + addr; // what does this do if it overflows..?

                None
            }
            8 => {
                let reg1: usize = get_x(input).into();
                let reg2: usize = get_y(input).into();

                let case = get_4_bit_constant(input);
                match case {
                    0 => self.registers[reg1] = self.registers[reg2],
                    1 => self.registers[reg1] = self.registers[reg1] | self.registers[reg2],
                    2 => self.registers[reg1] = self.registers[reg1] & self.registers[reg2],
                    3 => self.registers[reg1] = self.registers[reg1] ^ self.registers[reg2],
                    4 => {
                        let (val, carry) =
                            self.registers[reg1].overflowing_add(self.registers[reg2]);
                        self.registers[reg1] = val;
                        self.registers[15] = carry.into();
                    }
                    5 => {
                        let (val, carry) =
                            self.registers[reg1].overflowing_sub(self.registers[reg2]);
                        self.registers[reg1] = val;
                        self.registers[15] = (!carry).into(); // carry reg is set to 0 in chip-8 if it is an underflow
                    }
                    6 => {
                        let lsb = self.registers[reg1] & 0b00000001;
                        self.registers[reg1] = self.registers[reg1] >> 1;
                        self.registers[15] = lsb;
                    }
                    7 => {
                        let (val, carry) =
                            self.registers[reg2].overflowing_sub(self.registers[reg1]);
                        self.registers[reg2] = val;
                        self.registers[15] = (!carry).into(); // carry reg is set to 0 in chip-8 if it is an underflow
                    }
                    8 => {
                        let msb = self.registers[reg1] & 0b10000000;
                        self.registers[reg1] = self.registers[reg1] << 1;
                        self.registers[15] = msb;
                    }
                    _ => (),
                };
                None
            }
            9 => {
                let reg1: usize = get_x(input).into();
                let reg2: usize = get_y(input).into();

                if self.registers[reg1] != self.registers[reg2] {
                    return Some(self.program_counter + 4);
                }
                None
            }
            10 => {
                self.address_register = get_address(input);
                None
            }
            11 => Some((self.address_register + (self.registers[0] as u16)).into()),
            12 => {
                let reg1: usize = get_x(input).into();
                let con = get_constant(input);
                self.registers[reg1] = lsfr(self.cycle_count) & con;
                None
            }
            13 => {
                let reg1: usize = get_x(input).into();
                let reg2: usize = get_y(input).into();

                let x: usize = self.registers[reg1].into();
                let y: usize = self.registers[reg2].into();

                let con = get_4_bit_constant(input);

                let block = self
                    .memory
                    .get_block(self.address_register.into(), con.into());
                // get block of memory from i to i + con
                let res: bool = self.display.draw(block.clone(), x, y);

                self.registers[15] = res.into();

                None
            }
            14 => {
                let op = get_4_bit_constant(input);
                let reg1: usize = get_x(input).into();
                match op {
                    1 => {
                        if !self.input.is_set(self.registers[reg1].into()) {
                            return Some(self.program_counter + 4);
                        }
                        None
                    }
                    14 => {
                        if self.input.is_set(self.registers[reg1].into()) {
                            return Some(self.program_counter + 4);
                        }
                        None
                    }
                    _ => None,
                }
            }
            15 => {
                let op = get_constant(input);
                match op {
                    7 => {
                        let reg1: usize = get_x(input).into();
                        self.registers[reg1] = self.delay_timer;
                    }
                    10 => {
                        // check if a key is pressed, if not set the program counter back to where we are so we essentially loop until there is a key
                        let key_pressed = self.input.any_pressed();
                        if key_pressed {
                            let reg1: usize = get_x(input).into();
                            let k = self.input.get_key_pressed();
                            self.registers[reg1] = k;
                        } else {
                            return Some(self.program_counter);
                        }
                    }
                    21 => {
                        let reg1 = get_x(input);
                        self.delay_timer = reg1;
                    }
                    24 => {
                        let reg1 = get_x(input);
                        self.sound_timer = reg1;
                    }
                    30 => {
                        let reg1 = get_x(input);
                        self.address_register = self.address_register + reg1 as u16;
                    }
                    41 => {
                        // sprite base is 0x00
                        let reg1: usize = get_x(input).into();
                        let val = self.registers[reg1] & 0b00001111;
                        self.address_register = 0x00 + val as u16;
                    }
                    51 => {
                        let reg1: usize = get_x(input).into();
                        let val = self.registers[reg1];

                        self.memory.set(self.address_register as usize, val / 100);
                        self.memory
                            .set((self.address_register + 1) as usize, (val % 100) / 10);
                        self.memory
                            .set((self.address_register + 2) as usize, val % 10);
                    }
                    85 => {
                        for i in 0..self.registers.len() {
                            self.memory
                                .set(self.address_register as usize + i, self.registers[i]);
                        }
                    }
                    101 => {
                        for i in 0..self.registers.len() {
                            self.registers[i] = self.memory.get(self.address_register as usize + i);
                        }
                    }
                    _ => {}
                }
                None
            }
            _ => {
                println!("op code not implemented");
                None
            }
        }
    }
}

// input is 1 byte and we want to get the first nibble
fn get_op_code(input: u16) -> u8 {
    ((input & 0b1111000000000000) >> 12).try_into().unwrap() // this will give us the upper 4 bits
}

fn get_address(input: u16) -> u16 {
    input & 0b0000111111111111 // bottom 12 bits
}

// this can also be used for the ops where the bottom byte contains an "id" for example all of the different operations for the number 8 op
fn get_constant(input: u16) -> u8 {
    (input & 0b0000000011111111).try_into().unwrap()
}

fn get_4_bit_constant(input: u16) -> u8 {
    (input & 0b0000000000001111).try_into().unwrap()
}

fn get_x(input: u16) -> u8 {
    ((input & 0b0000111100000000) >> 8).try_into().unwrap()
}

fn get_y(input: u16) -> u8 {
    ((input & 0b0000000011110000) >> 4).try_into().unwrap()
}

fn lsfr(val: u8) -> u8 {
    let bit = ((val >> 7) ^ (val >> 5) ^ (val >> 4) ^ (val >> 3)) & 1;
    (val << 1) | bit
}

#[cfg(test)]
mod test {
    use super::*; // bring items from parent module into scope

    #[test]
    fn test_get_address() {
        let val = 0b1111111111111111;
        let out = get_address(val);

        assert_eq!(out, 0b0000111111111111);
    }

    #[test]
    fn test_get_address_2() {
        let val = 0xA25F;
        let out = get_address(val);
        assert_eq!(out, 0x25F)
    }

    #[test]
    fn test_get_op_code() {
        let val = 0b1111111111111111;
        let out = get_op_code(val);

        assert_eq!(out, 0b00001111);
    }

    #[test]
    fn test_get_constant() {
        let val = 0b1111111111111111;
        let out = get_constant(val);

        assert_eq!(out, 0b11111111);
    }

    #[test]
    fn test_get_4_bit_constant() {
        let val = 0b1111111111111111;
        let out = get_4_bit_constant(val);

        assert_eq!(out, 0b00001111);
    }

    #[test]
    fn test_get_x() {
        let val = 0b1010101010101010;
        let out = get_x(val);

        assert_eq!(out, 0b00001010);
    }

    #[test]
    fn test_get_y() {
        let val = 0b1010101010101010;
        let out = get_y(val);

        assert_eq!(out, 0b00001010);
    }

    #[test]
    fn test_get_set_memory() {
        let p1: u8 = 0x00;
        let p2: u8 = 0xE0;

        let mut mem = Memory::new();
        mem.set(0x200, p1);
        mem.set(0x200 + 1, p2);

        assert_eq!(mem.get(0x200), p1);
        assert_eq!(mem.get(0x200 + 1), p2);
    }

    #[test]
    fn test_get_op() {
        let p1: u8 = 0x00;
        let p2: u8 = 0xE0;

        let mut mem = Memory::new();
        mem.set(0x200, p1);
        mem.set(0x200 + 1, p2);

        let vm = Vm::new_with_memory(mem);

        let val = vm.get_op(0x200);

        assert_eq!(val, 0x00E0);
    }
}
