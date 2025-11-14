use crate::vm::Vm;
use std::env;

mod vm;

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
        panic!("only input path to rom")
    }

    let mut chip8 = Vm::new();

    chip8.load(args[1].to_string());

    chip8.start();
}
