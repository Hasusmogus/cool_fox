//! Translates the shader containers in the given files, natively (no D3D12,
//! no Windows): for work on translator gaps without the title running.
//!
//! cargo run -p victory-shader --example translate_one -- [--wave 32|64] <file>...
//!
//! A file is one container as a run's `known-shaders.bin` holds it. Such
//! files are the title's own code: keep them outside the repository.

use victory_shader::{Container, FourCC, ShaderError, emit, io, native_programs, resource, spill, structure};

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut wave = 32;
    if let Some(at) = args.iter().position(|a| a == "--wave") {
        wave = args.get(at + 1).and_then(|w| w.parse().ok()).unwrap_or(32);
        args.drain(at..(at + 2).min(args.len()));
    }
    emit::set_host_max_wave(wave);
    let mut failed = 0;
    for path in &args {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                println!("{path}: {e}");
                failed += 1;
                continue;
            }
        };
        match translate(&bytes) {
            Ok(lines) => {
                for l in lines {
                    println!("{path}: {l}");
                }
            }
            Err(e) => {
                println!("{path}: FAILED {e}");
                failed += 1;
            }
        }
    }
    std::process::exit(i32::from(failed > 0));
}

fn translate(bytes: &[u8]) -> Result<Vec<String>, String> {
    let container = Container::parse(bytes).map_err(|e| e.to_string())?;
    let dxil = container
        .part(FourCC::DXIL)
        .or(container.part(FourCC::ACCC))
        .map(|p| p.data)
        .ok_or("no DXIL part")?;
    let programs = native_programs(dxil).map_err(|e| e.to_string())?;
    let mut lines = Vec::new();
    for p in programs {
        let program = spill::build(&p).map_err(|e| format!("{:?}: ir: {e}", p.stage))?;
        let st = structure::structure(&program).map_err(|e| format!("{:?}: structure: {e}", p.stage))?;
        let res = resource::track(&program, &p.layout, p.stage, Some(&st)).map_err(|e| format!("{:?}: resources: {e}", p.stage))?;
        let raised = io::raise(&program, &p).map_err(|e| format!("{:?}: I/O: {e}", p.stage))?;
        match emit::emit(&container, &p, &program, &st, &raised, &res) {
            Ok(shader) => lines.push(format!(
                "{:?}: emitted {} bytes of DXIL",
                p.stage,
                shader.dxil.len()
            )),
            Err(ShaderError::Unsupported { offset, what }) => {
                return Err(format!("{:?}: emit at {offset:#x}: {what}", p.stage));
            }
            Err(e) => return Err(format!("{:?}: emit: {e}", p.stage)),
        }
    }
    Ok(lines)
}
