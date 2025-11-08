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

    fn set(&mut self, value: u8, x: usize, y: usize) {
        self.pixels[y][x] = value
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
        }
    }

    // blocks forever
    fn start(&mut self) {
        loop {
            // get the next instruction
            let op = self.get_op(self.program_counter);

            let jump_address = self.execute(op);

            // check the timers here..?

            // update the program counter
            self.update_program_counter(jump_address);
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

                self.registers[reg1] = self.registers[reg2];
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
