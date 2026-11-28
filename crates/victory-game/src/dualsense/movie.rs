use std::collections::{HashMap, HashSet};

use super::glyphs::{self, Button};

fn u16le(data: &[u8]) -> Option<u16> {
    Some(u16::from_le_bytes(data.get(..2)?.try_into().ok()?))
}

fn u32le(data: &[u8]) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(..4)?.try_into().ok()?))
}

struct Tag<'a> {
    kind: u16,
    body: &'a [u8],
    encoded: &'a [u8],
}

fn tags(mut bytes: &[u8]) -> Option<Vec<Tag<'_>>> {
    let mut result = Vec::new();
    loop {
        let head = u16le(bytes)?;
        let (n, header) = if head & 63 == 63 {
            (u32le(bytes.get(2..)?)? as usize, 6)
        } else {
            ((head & 63) as usize, 2)
        };
        let body = bytes.get(header..)?.get(..n)?;
        let (encoded, rest) = bytes.split_at(header + n);
        let kind = head >> 6;
        result.push(Tag {
            kind,
            body,
            encoded,
        });
        if kind == 0 {
            return (n == 0 && rest.is_empty()).then_some(result);
        }
        bytes = rest;
    }
}

/// The short prefix available while searching the archive. Test the export
/// header here so unrelated movies and deflate streams never get rebuilt.
pub(super) fn is_input_movie(data: &[u8]) -> bool {
    data.starts_with(b"GFX") && memchr::memmem::find(data, b"input_icons\0").is_some()
}

fn exports(mut table: &[u8]) -> Option<HashMap<u16, Button>> {
    let count = u16le(table)?;
    table = &table[2..];
    let mut sprites = HashMap::new();
    let mut found = HashSet::new();
    for _ in 0..count {
        let id = u16le(table)?;
        let name = table.get(2..)?;
        let end = memchr::memchr(0, name)?;
        if let Some(number) = name[..end].strip_prefix(b"platform_xbsx.input_icons.Main_Button_") {
            let number: u8 = std::str::from_utf8(number).ok()?.parse().ok()?;
            if let Some(b) = Button::ALL.into_iter().find(|&b| b as u8 == number)
                && (sprites.insert(id, b).is_some() || !found.insert(number))
            {
                return None;
            }
        }
        table = &name[end + 1..];
    }
    (sprites.len() == Button::ALL.len() && table.is_empty()).then_some(sprites)
}

fn push_tag(out: &mut Vec<u8>, kind: u16, body: &[u8]) {
    out.extend(((kind << 6) | (body.len().min(63) as u16)).to_le_bytes());
    if body.len() >= 63 {
        out.extend((body.len() as u32).to_le_bytes());
    }
    out.extend(body);
}

/// Replace only the glyph shapes. Leave placement, action bindings and
/// ActionScript intact. Unknown exports or shared geometry fail the entire
/// conversion before a result is returned.
pub(super) fn replace(data: &[u8]) -> Option<Vec<u8>> {
    if !is_input_movie(data.get(..128)?) || u32le(&data[4..])? as usize != data.len() {
        return None;
    }
    let rect_bits = 5 + 4 * usize::from(data[8] >> 3);
    let start = 8 + rect_bits.div_ceil(8) + 4;
    let movie = tags(data.get(start..)?)?;
    let wanted = exports(movie.iter().find(|t| t.kind == 56)?.body)?;
    let mut replacements = HashMap::new();
    let mut references = HashMap::<u16, usize>::new();
    let mut used_ids = HashSet::new();
    let mut replaced_sprites = 0;
    for tag in &movie {
        if matches!(tag.kind, 32 | 39 | 87) && !used_ids.insert(u16le(tag.body)?) {
            return None;
        }
        if tag.kind != 39 {
            continue;
        }
        let id = u16le(tag.body)?;
        let children = tags(tag.body.get(4..)?)?;
        let mut shapes = Vec::new();
        for child in children.iter().filter(|t| t.kind != 0) {
            // Fixed-depth static artwork with a character and a matrix.
            if child.kind != 26 || child.body.first() != Some(&6) {
                return None;
            }
            let shape = u16le(child.body.get(3..)?)?;
            *references.entry(shape).or_default() += 1;
            shapes.push(shape);
        }
        let Some(&b) = wanted.get(&id) else {
            continue;
        };
        if b.face() {
            let &symbol = shapes.last()?;
            replacements.insert(symbol, glyphs::artwork(symbol, b));
        } else {
            let (&first, others) = shapes.split_first()?;
            replacements.insert(first, glyphs::artwork(first, b));
            for &other in others {
                replacements.insert(other, glyphs::empty(other));
            }
        }
        replaced_sprites += 1;
    }
    if replaced_sprites != wanted.len()
        || replacements.keys().any(|id| references.get(id) != Some(&1))
    {
        return None;
    }
    let mut out = data[..start].to_vec();
    for tag in &movie {
        if tag.kind == 32
            && let Some(body) = replacements.remove(&u16le(tag.body)?)
        {
            push_tag(&mut out, 32, &body);
        } else if tag.kind != 0 {
            out.extend(tag.encoded);
        }
    }
    if !replacements.is_empty() {
        return None;
    }
    // RPF advertises the unpacked length. An unreferenced DefineBinaryData
    // fills the gap with a valid tag without changing the movie's frame count.
    let gap = data.len().checked_sub(out.len() + 2)?;
    if gap < 69 {
        return None;
    }
    let spare = (1..u16::MAX).rev().find(|id| !used_ids.contains(id))?;
    let mut padding = vec![0; gap - 6];
    padding[..2].copy_from_slice(&spare.to_le_bytes());
    push_tag(&mut out, 87, &padding);
    push_tag(&mut out, 0, &[]);
    (out.len() == data.len()).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    // A synthetic movie with independent drawing IDs and export order.
    // Multiple layers give its sprites enough room for new outlines.
    fn sample(shared: bool) -> Vec<u8> {
        let mut out = b"GFX*\0\0\0\0\x08\0\0\x1e\x01\0".to_vec();
        push_tag(&mut out, 1000, b"input_icons\0");
        let mut names = (Button::ALL.len() as u16).to_le_bytes().to_vec();
        for (i, b) in Button::ALL.into_iter().rev().enumerate() {
            let sprite = 1000 + i as u16;
            names.extend(sprite.to_le_bytes());
            names.extend(format!("platform_xbsx.input_icons.Main_Button_{}\0", b as u8).bytes());
            let mut children = [sprite.to_le_bytes(), 0u16.to_le_bytes()].concat();
            for layer in 0..4u16 {
                let shape = 1 + 4 * i as u16 + layer;
                push_tag(&mut out, 32, &glyphs::artwork(shape, Button::R2));
                let placed = if shared && i == 1 && layer == 3 {
                    4
                } else {
                    shape
                };
                let mut object = vec![6];
                object.extend((layer + 1).to_le_bytes());
                object.extend(placed.to_le_bytes());
                object.push(0);
                push_tag(&mut children, 26, &object);
            }
            push_tag(&mut children, 0, &[]);
            push_tag(&mut out, 39, &children);
        }
        push_tag(&mut out, 56, &names);
        push_tag(&mut out, 1, &[]);
        push_tag(&mut out, 0, &[]);
        let n = out.len() as u32;
        out[4..8].copy_from_slice(&n.to_le_bytes());
        out
    }

    #[test]
    fn patches_drawing_ids_from_the_exports_and_preserves_sprite_placements() {
        let source = sample(false);
        let result = replace(&source).unwrap();
        assert_eq!(source.len(), result.len());
        let original = tags(&source[14..]).unwrap();
        let patched = tags(&result[14..]).unwrap();
        for kind in [39, 56, 1000] {
            let bodies = |tags: &[Tag<'_>]| {
                tags.iter()
                    .filter(|t| t.kind == kind)
                    .map(|t| t.body.to_vec())
                    .collect::<Vec<_>>()
            };
            assert_eq!(bodies(&original), bodies(&patched));
        }
        for (i, button) in Button::ALL.into_iter().rev().enumerate() {
            let id = 1 + 4 * i as u16 + if button.face() { 3 } else { 0 };
            let new = patched
                .iter()
                .find(|t| t.kind == 32 && u16le(t.body) == Some(id))
                .unwrap();
            assert_eq!(new.body, glyphs::artwork(id, button));
        }
        assert!(replace(&sample(true)).is_none());
    }
}
