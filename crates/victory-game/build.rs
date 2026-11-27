//! Bake our controller illustration for the console texture layout. The
//! compiler only needs the drawing in this crate, never a copy of the game.

#[path = "art/controller.rs"]
mod controller;

use std::{env, fs, path::PathBuf};

use resvg::{tiny_skia, usvg};
use victory_formats::texture::{Dimension, Swizzle, Texture, TextureDesc};

const WIDTH: usize = 1024;
const HEIGHT: usize = 512;

fn main() {
    println!("cargo:rerun-if-changed=art/controller.rs");
    println!("cargo:rerun-if-changed=build.rs");
    let drawing = usvg::Tree::from_str(controller::SVG, &usvg::Options::default()).unwrap();
    let mut canvas = tiny_skia::Pixmap::new(WIDTH as u32, HEIGHT as u32).unwrap();
    let scale = tiny_skia::Transform::from_scale(
        WIDTH as f32 / drawing.size().width(),
        HEIGHT as f32 / drawing.size().height(),
    );
    resvg::render(&drawing, scale, &mut canvas.as_mut());
    let mut linear = Vec::with_capacity(WIDTH * HEIGHT);
    for y in (0..HEIGHT).step_by(4) {
        for x in (0..WIDTH).step_by(4) {
            let block = std::array::from_fn(|i| {
                let p = canvas.pixels()[(y + i / 4) * WIDTH + x + i % 4].demultiply();
                [p.red(), p.green(), p.blue(), p.alpha()]
            });
            linear.extend_from_slice(&bc7_block(&block));
        }
    }
    let texture = Texture::new(&TextureDesc {
        dimension: Dimension::Tex2d,
        width: WIDTH as u32,
        height: HEIGHT as u32,
        depth_or_array: 1,
        mips: 1,
        format: 98,
        samples: 1,
        swizzle: Swizzle(10),
    })
    .unwrap();
    let mut tiled = vec![0; linear.len()];
    texture.planes[0]
        .layout
        .tile(&mut tiled, 0, 0, &linear, WIDTH * 4, linear.len())
        .unwrap();
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("dualsense.bc7"),
        tiled,
    )
    .unwrap();
}

type Pixel = [u8; 4];

fn distance(a: &Pixel, b: &Pixel) -> u32 {
    a.iter()
        .zip(b)
        .map(|(&a, &b)| i32::from(a.abs_diff(b)).pow(2) as u32)
        .sum()
}

/// BC7 mode 6 has one RGBA endpoint pair and 16 interpolation weights.
/// Two farthest-point passes suffice for the greys and antialiased edges
/// of this illustration. Each endpoint shares its low bit across RGBA.
fn bc7_block(block: &[Pixel; 16]) -> [u8; 16] {
    let farthest = |from: &Pixel| *block.iter().max_by_key(|p| distance(p, from)).unwrap();
    let first = farthest(&block[0]);
    let quantize = |p: Pixel| {
        [0, 1]
            .map(|low| p.map(|v| (v & !1) | low))
            .into_iter()
            .min_by_key(|q| distance(&p, q))
            .unwrap()
    };
    let mut a = quantize(first);
    let mut b = quantize(farthest(&first));
    let palette = [
        0u16, 4, 9, 13, 17, 21, 26, 30, 34, 38, 43, 47, 51, 55, 60, 64,
    ]
    .map(|w| {
        std::array::from_fn(|c| {
            ((u16::from(a[c]) * (64 - w) + u16::from(b[c]) * w + 32) / 64) as u8
        })
    });
    let mut indices = block.map(|p| (0..16).min_by_key(|&i| distance(&p, &palette[i])).unwrap());
    // The first texel carries three index bits. Swapping the endpoints
    // lets that texel refer to either end without changing any colours.
    if indices[0] >= 8 {
        std::mem::swap(&mut a, &mut b);
        indices = indices.map(|i| 15 - i);
    }
    let mut word = 1u128 << 6;
    for c in 0..4 {
        let pos = 7 + c * 14;
        word |= u128::from(a[c] >> 1) << pos;
        word |= u128::from(b[c] >> 1) << (pos + 7);
    }
    word |= u128::from(a[0] & 1) << 63 | u128::from(b[0] & 1) << 64 | (indices[0] as u128) << 65;
    for (i, &index) in indices[1..].iter().enumerate() {
        word |= (index as u128) << (68 + 4 * i);
    }
    word.to_le_bytes()
}
