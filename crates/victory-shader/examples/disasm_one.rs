//! Prints the console code of the shader containers in the given files in
//! SP3 syntax, one block of the translator's IR at a time.
//!
//! cargo run -p victory-shader --example disasm_one -- <file>...
//!
//! A file is one container as a run's `known-shaders.bin` holds it (the
//! title's own code: keep it outside the repository).

use victory_shader::dxbc::{Container, FourCC};
use victory_shader::gfx10::decode;
use victory_shader::native::native_programs;

fn main() {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).expect("readable");
        let container = Container::parse(&bytes).expect("a container");
        let dxil = container.part(FourCC::DXIL).or(container.part(FourCC::ACCC)).expect("DXIL part").data;
        for p in native_programs(dxil).expect("programs") {
            println!("== {path} {:?} wave64={}", p.stage, p.wave64);
            let program = victory_shader::ir::build(&p.code, p.wave64, p.vgprs).expect("IR");
            for (b, block) in program.blocks.iter().enumerate() {
                println!("-- block {b} at {:#x}, preds {:?}, exit {:?}", block.offset, block.preds, block.exit);
                for inst in &block.insts {
                    println!("   {}", inst.name());
                }
            }
            let _ = decode(&p.code).map(|insts| {
                for i in insts {
                    println!("{}", i.text(p.wave64));
                }
            });
        }
    }
}
