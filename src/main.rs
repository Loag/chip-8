use crate::vm::Vm;

mod vm;

fn main() {
    let mut chip8 = Vm::new();

    chip8.load("./1-chip8-logo.ch8".to_string());

    chip8.start();
}
