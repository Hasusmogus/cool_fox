//! Vector buttons in the 64-pixel artwork coordinate system (20 twips per pixel).

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(super) enum Button {
    L3 = 16,
    R3 = 25,
    Cross = 30,
    Circle,
    Square,
    Triangle,
    L1,
    L2,
    R1,
    R2,
    Options,
    Create,
}

impl Button {
    pub const ALL: [Self; 12] = [
        Self::L3,
        Self::R3,
        Self::Cross,
        Self::Circle,
        Self::Square,
        Self::Triangle,
        Self::L1,
        Self::L2,
        Self::R1,
        Self::R2,
        Self::Options,
        Self::Create,
    ];

    pub fn face(self) -> bool {
        matches!(
            self,
            Self::Cross | Self::Circle | Self::Square | Self::Triangle
        )
    }
}

type Point = (i16, i16);
type Path = Vec<Point>;
type Layer = (u8, Vec<Path>);

fn circle(x: i16, y: i16, radius: i16) -> Path {
    (0..32)
        .map(|i| {
            let a = std::f64::consts::TAU * f64::from(i) / 32.0;
            (
                x + (f64::from(radius) * a.cos()).round() as i16,
                y + (f64::from(radius) * a.sin()).round() as i16,
            )
        })
        .collect()
}

fn hole(mut path: Path) -> Path {
    path.reverse();
    path
}

fn rounded(left: i16, top: i16, right: i16, bottom: i16, r: i16) -> Path {
    let corners = [
        (right - r, top + r),
        (right - r, bottom - r),
        (left + r, bottom - r),
        (left + r, top + r),
    ];
    let mut points = Vec::new();
    for (corner, (x, y)) in corners.into_iter().enumerate() {
        for i in 0..=4 {
            let a = (corner as f64 - 1.0 + f64::from(i) / 4.0) * std::f64::consts::FRAC_PI_2;
            points.push((
                x + (f64::from(r) * a.cos()).round() as i16,
                y + (f64::from(r) * a.sin()).round() as i16,
            ));
        }
    }
    points
}

// Our condensed lettering for the shoulder, trigger and stick labels.
fn contours(ch: char) -> &'static [&'static [(i16, i16)]] {
    match ch {
        'L' => &[&[(0, 0), (17, 0), (17, 83), (62, 83), (62, 100), (0, 100)]],
        'R' => &[
            &[
                (0, 0),
                (42, 0),
                (64, 18),
                (64, 42),
                (45, 59),
                (68, 100),
                (48, 100),
                (26, 61),
                (17, 61),
                (17, 100),
                (0, 100),
            ],
            &[(17, 16), (17, 45), (38, 45), (47, 37), (47, 25), (37, 16)],
        ],
        '1' => &[&[
            (4, 21),
            (31, 0),
            (46, 0),
            (46, 84),
            (64, 84),
            (64, 100),
            (6, 100),
            (6, 84),
            (28, 84),
            (28, 22),
            (12, 34),
        ]],
        '2' => &[&[
            (0, 22),
            (18, 2),
            (46, 2),
            (64, 19),
            (64, 42),
            (23, 84),
            (66, 84),
            (66, 100),
            (0, 100),
            (0, 85),
            (47, 36),
            (47, 25),
            (40, 18),
            (25, 18),
            (13, 32),
        ]],
        '3' => &[&[
            (0, 16),
            (18, 1),
            (46, 1),
            (63, 16),
            (63, 40),
            (51, 50),
            (64, 62),
            (64, 85),
            (46, 100),
            (18, 100),
            (0, 86),
            (11, 73),
            (23, 83),
            (40, 83),
            (47, 77),
            (47, 68),
            (39, 59),
            (21, 59),
            (21, 42),
            (39, 42),
            (46, 34),
            (46, 25),
            (39, 18),
            (24, 18),
            (11, 28),
        ]],
        _ => unreachable!(),
    }
}

fn label(pair: &str, cy: i16, height: i16) -> Vec<Path> {
    let scale = f64::from(height) / 100.0;
    let mut paths = Vec::new();
    for (i, ch) in pair.chars().enumerate() {
        let start = 640.0 + (85.0 * i as f64 - 74.5) * scale;
        for contour in contours(ch) {
            paths.push(
                contour
                    .iter()
                    .map(|&(x, y)| {
                        (
                            (start + f64::from(x) * scale).round() as i16,
                            cy + (f64::from(y - 50) * scale).round() as i16,
                        )
                    })
                    .collect(),
            );
        }
    }
    paths
}

pub(super) fn artwork(id: u16, button: Button) -> Vec<u8> {
    use Button::*;
    // The face symbols go on the movie's existing circular button background.
    let paths = match button {
        Cross => vec![vec![
            (416, 469),
            (469, 416),
            (640, 587),
            (811, 416),
            (864, 469),
            (693, 640),
            (864, 811),
            (811, 864),
            (640, 693),
            (469, 864),
            (416, 811),
            (587, 640),
        ]],
        Circle => vec![circle(640, 640, 252), hole(circle(640, 640, 180))],
        Square => vec![
            vec![(418, 418), (862, 418), (862, 862), (418, 862)],
            vec![(484, 484), (484, 796), (796, 796), (796, 484)],
        ],
        Triangle => vec![
            vec![(640, 370), (930, 861), (350, 861)],
            vec![(640, 510), (476, 794), (804, 794)],
        ],
        L1 | L2 | R1 | R2 => {
            let (text, top, bottom) = match button {
                L1 => ("L1", 270, 1010),
                R1 => ("R1", 270, 1010),
                L2 => ("L2", 120, 1160),
                _ => ("R2", 120, 1160),
            };
            return shape(
                id,
                &[
                    (0, vec![rounded(4, top, 1276, bottom, 155)]),
                    (255, vec![rounded(67, top + 62, 1213, bottom - 62, 99)]),
                    (0, label(text, 640, 385)),
                ],
            );
        }
        L3 | R3 => {
            let mut text = label(if button == L3 { "L3" } else { "R3" }, 735, 380);
            text.push(vec![(494, 265), (786, 265), (640, 420)]);
            return shape(
                id,
                &[
                    (0, vec![circle(640, 690, 585)]),
                    (255, vec![circle(640, 690, 524)]),
                    (0, text),
                ],
            );
        }
        Options | Create => {
            let marks = if button == Options {
                [407, 600, 793]
                    .map(|y| rounded(345, y, 935, y + 77, 37))
                    .to_vec()
            } else {
                vec![
                    vec![(273, 470), (338, 430), (438, 590), (372, 630)],
                    vec![(601, 335), (679, 335), (679, 533), (601, 533)],
                    vec![(842, 590), (942, 430), (1006, 470), (906, 630)],
                    rounded(479, 714, 801, 815, 48),
                ]
            };
            return shape(
                id,
                &[
                    (0, vec![circle(640, 640, 637)]),
                    (255, vec![circle(640, 640, 577)]),
                    (0, marks),
                ],
            );
        }
    };
    shape(id, &[(0, paths)])
}

pub(super) fn empty(id: u16) -> Vec<u8> {
    shape(id, &[])
}

#[derive(Default)]
struct Bits {
    bytes: Vec<u8>,
    used: u8,
}

impl Bits {
    fn put(&mut self, value: i32, width: u8) {
        for bit in (0..width).rev() {
            if self.used == 0 {
                self.bytes.push(0);
            }
            *self.bytes.last_mut().unwrap() |= (((value >> bit) & 1) as u8) << (7 - self.used);
            self.used = (self.used + 1) % 8;
        }
    }
}

fn signed_width(v: i16) -> u8 {
    (17 - (if v < 0 { !v } else { v }).leading_zeros()) as u8
}

/// Encode a SWF DefineShape3 body with solid fills and straight edges.
/// Holes run in the opposite direction from their outside contour.
fn shape(id: u16, layers: &[Layer]) -> Vec<u8> {
    let mut bounds = None;
    for &p in layers.iter().flat_map(|(_, paths)| paths.iter().flatten()) {
        let [x0, x1, y0, y1] = bounds.unwrap_or([p.0, p.0, p.1, p.1]);
        bounds = Some([x0.min(p.0), x1.max(p.0), y0.min(p.1), y1.max(p.1)]);
    }
    let bounds = bounds.unwrap_or([0; 4]);
    let width = bounds.map(signed_width).into_iter().max().unwrap();
    let mut rect = Bits::default();
    rect.put(width.into(), 5);
    for value in bounds {
        rect.put(value.into(), width);
    }
    let mut out = id.to_le_bytes().to_vec();
    out.extend(rect.bytes);
    out.push(layers.len() as u8);
    for &(grey, _) in layers {
        out.extend([0, grey, grey, grey, 255]);
    }
    out.push(0); // no line styles
    let fill_bits = (usize::BITS - layers.len().leading_zeros()) as u8;
    let mut edges = Bits::default();
    edges.put(fill_bits.into(), 4);
    edges.put(0, 4);
    for (i, (_, paths)) in layers.iter().enumerate() {
        for path in paths {
            let mut point = path[0];
            let n = signed_width(point.0).max(signed_width(point.1));
            edges.put(5, 6); // style change, MoveTo + FillStyle1
            edges.put(n.into(), 5);
            edges.put(point.0.into(), n);
            edges.put(point.1.into(), n);
            edges.put(i as i32 + 1, fill_bits);
            for &next in path[1..].iter().chain(&path[..1]) {
                let (dx, dy) = (next.0 - point.0, next.1 - point.1);
                point = next;
                if dx == 0 && dy == 0 {
                    continue;
                }
                let n = signed_width(dx).max(signed_width(dy)).max(2);
                edges.put(3, 2); // straight edge
                edges.put((n - 2).into(), 4);
                if dx != 0 && dy != 0 {
                    edges.put(1, 1);
                    edges.put(dx.into(), n);
                    edges.put(dy.into(), n);
                } else {
                    edges.put(i32::from(dy != 0), 2);
                    edges.put(if dx == 0 { dy } else { dx }.into(), n);
                }
            }
        }
    }
    edges.put(0, 6); // EndShapeRecord
    out.extend(edges.bytes);
    out
}
