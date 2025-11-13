struct Screen {
    pixels: Vec<Vec<u8>>,
}

impl Screen {
    fn new() -> Screen {
        Screen {
            pixels: vec![vec![0x00; 32]; 64],
        }
    }

    fn clear(&mut self) {
        self.pixels = vec![vec![0x00; 32]; 64]
    }

    fn set(&mut self, value: u8, x: usize, y: usize) -> bool {
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
        for i in 0..7 {
            let val = value; // TODO get bit values here
            let was_updated = self.set(val, x, start_y + i);
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
    fn new() -> Memory {
        Memory {
            slots: vec![0x00; 4096],
        }
    }
    fn get(&self, address: usize) -> u8 {
        if address <= 4096 {
            return self.slots[address];
        }
        panic!("tried to read memory out of bounds")
    }

    fn get_block(&self, address: usize, length: usize) -> Vec<u8> {
        self.slots[address..(address + length)].into()
    }

    fn set(&mut self, address: usize, value: u8) {
        if address <= 4096 {
            self.slots[address] = value
        } else {
            panic!("tried to set a memory segment out of bounds.")
        }
    }
}

struct Vm {
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
    fn new() -> Vm {
        Vm {
            registers: vec![0x00; 16],
            address_register: 0x000,
            program_counter: 0x200,
            memory: Memory::new(),
            delay_timer: 0,
            sound_timer: 0,
            display: Screen::new(), // where screen is 64 * 32
            cycle_count: 0,
            input: Input::new(),
        }
    }

    // blocks forever
    fn start(&mut self) {
        loop {
            // check if there is a key pressed?
            let op = self.get_op(self.program_counter);

            let jump_address = self.execute(op);
            self.update_program_counter(jump_address);

            // handle key pressing here..?

            // check and handle timers here..?

            self.cycle_count = self.cycle_count.wrapping_add(1);
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
                    0x00E0 => self.clear_display(), // this is clear the display
                    0x00EE => {} // this is return, so we should pop the stack and return that val to set program counter with
                    _ => println!("unknown op in range 0"),
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
                let reg2: usize = get_x(input).into();
                let con = get_constant(input);

                // get block of memory from i to i + con
                let res = self.display.draw(
                    self.memory
                        .get_block(self.address_register.into(), con.into()),
                    reg1,
                    reg2,
                );

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
                    41 => {}
                    51 => {}
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
                println!("code not implemented");
                None
            }
        }
    }

    fn clear_display(&mut self) {
        self.display.clear()
    }
}

fn main() {
    let mut chip_8 = Vm::new();
    chip_8.start()
}

// input is 1 byte and we want to get the first nibble
fn get_op_code(input: u16) -> u8 {
    (input & 0b1111000000000000).try_into().unwrap() // this will give us the upper 4 bits
}

fn get_address(input: u16) -> u16 {
    input & 0b00001111111111111111 // bottom 12 bits
}

// this can also be used for the ops where the bottom byte contains an "id" for example all of the different operations for the number 8 op
fn get_constant(input: u16) -> u8 {
    (input & 0b0000000011111111).try_into().unwrap()
}

fn get_4_bit_constant(input: u16) -> u8 {
    (input & 0b0000000000001111).try_into().unwrap()
}

fn get_x(input: u16) -> u8 {
    (input & 0b0000111100000000).try_into().unwrap()
}

fn get_y(input: u16) -> u8 {
    (input & 0b0000000011110000).try_into().unwrap()
}

fn lsfr(val: u8) -> u8 {
    let bit = ((val >> 7) ^ (val >> 5) ^ (val >> 4) ^ (val >> 3)) & 1;
    (val << 1) | bit
}

// for n, beginning at address register, read bytes from memory.
// get the bits of the piece of memory and draw bits to screen starting at location x, y, going down
fn draw() {}
