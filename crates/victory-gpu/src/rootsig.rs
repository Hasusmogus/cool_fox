//! Root signatures of our own shaders, made here rather than taken from the
//! compiled shader. Under Wine `D3DCompile` is vkd3d-shader's compiler,
//! which ignores `[RootSignature]` and embeds none, so `CreateRootSignature`
//! on the shader's bytes fails there. The flags are those a version 1.1 root
//! signature string gets by default, so the result is what the string in
//! the shader describes.

use windows::Win32::Graphics::Direct3D::ID3DBlob;
use windows::Win32::Graphics::Direct3D12::*;

/// One root parameter, as a root signature string names it.
#[derive(Clone, Copy)]
pub(crate) enum Param {
    /// `RootConstants(num32BitConstants=count, b<register>)`.
    Constants { register: u32, count: u32 },
    /// `UAV(u<register>)`: a root descriptor.
    Uav(u32),
    /// `DescriptorTable(SRV(t<register>, numDescriptors=count))`.
    SrvTable { register: u32, count: u32, visibility: D3D12_SHADER_VISIBILITY },
    /// `DescriptorTable(UAV(u<register>, numDescriptors=count))`.
    UavTable { register: u32, count: u32 },
}

/// The root signature of `params`, without static samplers.
pub(crate) fn make(device: &ID3D12Device, params: &[Param], what: &str) -> windows::core::Result<ID3D12RootSignature> {
    // The ranges first: the parameters point into them.
    let ranges: Vec<D3D12_DESCRIPTOR_RANGE1> = params
        .iter()
        .map(|p| {
            let (kind, register, count, flags) = match *p {
                Param::SrvTable { register, count, .. } => (
                    D3D12_DESCRIPTOR_RANGE_TYPE_SRV,
                    register,
                    count,
                    D3D12_DESCRIPTOR_RANGE_FLAG_DATA_STATIC_WHILE_SET_AT_EXECUTE,
                ),
                Param::UavTable { register, count } => {
                    (D3D12_DESCRIPTOR_RANGE_TYPE_UAV, register, count, D3D12_DESCRIPTOR_RANGE_FLAG_DATA_VOLATILE)
                }
                _ => (D3D12_DESCRIPTOR_RANGE_TYPE_SRV, 0, 0, D3D12_DESCRIPTOR_RANGE_FLAG_NONE),
            };
            D3D12_DESCRIPTOR_RANGE1 {
                RangeType: kind,
                NumDescriptors: count,
                BaseShaderRegister: register,
                RegisterSpace: 0,
                Flags: flags,
                OffsetInDescriptorsFromTableStart: D3D12_DESCRIPTOR_RANGE_OFFSET_APPEND,
            }
        })
        .collect();
    let built: Vec<D3D12_ROOT_PARAMETER1> = params
        .iter()
        .zip(&ranges)
        .map(|(p, range)| match *p {
            Param::Constants { register, count } => D3D12_ROOT_PARAMETER1 {
                ParameterType: D3D12_ROOT_PARAMETER_TYPE_32BIT_CONSTANTS,
                Anonymous: D3D12_ROOT_PARAMETER1_0 {
                    Constants: D3D12_ROOT_CONSTANTS { ShaderRegister: register, RegisterSpace: 0, Num32BitValues: count },
                },
                ShaderVisibility: D3D12_SHADER_VISIBILITY_ALL,
            },
            Param::Uav(register) => D3D12_ROOT_PARAMETER1 {
                ParameterType: D3D12_ROOT_PARAMETER_TYPE_UAV,
                Anonymous: D3D12_ROOT_PARAMETER1_0 {
                    Descriptor: D3D12_ROOT_DESCRIPTOR1 {
                        ShaderRegister: register,
                        RegisterSpace: 0,
                        Flags: D3D12_ROOT_DESCRIPTOR_FLAG_DATA_VOLATILE,
                    },
                },
                ShaderVisibility: D3D12_SHADER_VISIBILITY_ALL,
            },
            Param::SrvTable { .. } | Param::UavTable { .. } => D3D12_ROOT_PARAMETER1 {
                ParameterType: D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE,
                Anonymous: D3D12_ROOT_PARAMETER1_0 {
                    DescriptorTable: D3D12_ROOT_DESCRIPTOR_TABLE1 { NumDescriptorRanges: 1, pDescriptorRanges: range },
                },
                ShaderVisibility: match *p {
                    Param::SrvTable { visibility, .. } => visibility,
                    _ => D3D12_SHADER_VISIBILITY_ALL,
                },
            },
        })
        .collect();
    let desc = D3D12_VERSIONED_ROOT_SIGNATURE_DESC {
        Version: D3D_ROOT_SIGNATURE_VERSION_1_1,
        Anonymous: D3D12_VERSIONED_ROOT_SIGNATURE_DESC_0 {
            Desc_1_1: D3D12_ROOT_SIGNATURE_DESC1 {
                NumParameters: built.len() as u32,
                pParameters: built.as_ptr(),
                NumStaticSamplers: 0,
                pStaticSamplers: std::ptr::null(),
                Flags: D3D12_ROOT_SIGNATURE_FLAG_NONE,
            },
        },
    };
    let mut blob: Option<ID3DBlob> = None;
    let mut error: Option<ID3DBlob> = None;
    // SAFETY: `desc` points at the live `built` and `ranges`; the blobs are
    // read in full before they are released.
    unsafe {
        if let Err(e) = D3D12SerializeVersionedRootSignature(&desc, &mut blob, Some(&mut error)) {
            if let Some(error) = error {
                let text = std::slice::from_raw_parts(error.GetBufferPointer().cast::<u8>(), error.GetBufferSize());
                tracing::warn!("{what} root signature: {}", String::from_utf8_lossy(text));
            }
            return Err(e);
        }
        let blob = blob.ok_or_else(windows::core::Error::empty)?;
        let bytes = std::slice::from_raw_parts(blob.GetBufferPointer().cast::<u8>(), blob.GetBufferSize());
        device.CreateRootSignature(0, bytes)
    }
}

/// The root signature a root signature string describes (the `[RootSignature]`
/// attribute's text), made here. For our shaders under Wine, whose compiler embeds
/// none: [`of_source`] takes the string from the shader's own `#define RS "..."`, so
/// the two cannot drift apart. Version 1.1 defaults, as the HLSL compiler gives them.
pub(crate) fn from_string(device: &ID3D12Device, rs: &str, what: &str) -> windows::core::Result<ID3D12RootSignature> {
    let parsed = parse(rs).map_err(|e| {
        tracing::warn!("{what} root signature {rs:?}: {e}");
        windows::core::Error::from_hresult(windows::Win32::Foundation::E_INVALIDARG)
    })?;
    serialize(device, &parsed, what)
}

/// [`from_string`] of the `#define RS "..."` in `source` (a shader's HLSL).
pub(crate) fn of_source(device: &ID3D12Device, source: &str, what: &str) -> windows::core::Result<ID3D12RootSignature> {
    let Some(rs) = define_rs(source) else {
        tracing::warn!("{what}: no #define RS in the shader source");
        return Err(windows::core::Error::from_hresult(windows::Win32::Foundation::E_INVALIDARG));
    };
    from_string(device, rs, what)
}

/// The text of `#define RS "..."` in `source`.
fn define_rs(source: &str) -> Option<&str> {
    let at = source.find("#define RS ")?;
    let rest = &source[at + "#define RS ".len()..];
    let open = rest.find('"')?;
    let close = rest[open + 1..].find('"')?;
    Some(&rest[open + 1..open + 1 + close])
}

/// A parsed root signature: plain data, turned into D3D12's structures by [`serialize`].
#[derive(Debug, Default, PartialEq)]
struct Parsed {
    flags: i32,
    params: Vec<ParsedParam>,
    samplers: Vec<D3D12_STATIC_SAMPLER_DESC>,
}

#[derive(Debug, PartialEq)]
enum ParsedParam {
    Constants { register: u32, space: u32, count: u32, visibility: i32 },
    /// A root descriptor: its type (CBV, SRV, UAV).
    View { kind: i32, register: u32, space: u32, flags: i32, visibility: i32 },
    Table { ranges: Vec<ParsedRange>, visibility: i32 },
}

#[derive(Debug, PartialEq)]
struct ParsedRange {
    kind: i32,
    register: u32,
    space: u32,
    count: u32,
    flags: i32,
    offset: u32,
}

/// `text`'s items at nesting depth 0, split on commas.
fn split_top(text: &str) -> Vec<&str> {
    let (mut depth, mut start, mut out) = (0i32, 0usize, Vec::new());
    for (i, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                out.push(text[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    let last = text[start..].trim();
    if !last.is_empty() {
        out.push(last);
    }
    out
}

/// `name(args)` into the name and the args' text.
fn call(item: &str) -> Result<(&str, &str), String> {
    let open = item.find('(').ok_or_else(|| format!("{item:?} is not name(...)"))?;
    let close = item.rfind(')').ok_or_else(|| format!("{item:?} has no closing parenthesis"))?;
    Ok((item[..open].trim(), &item[open + 1..close]))
}

/// `key = value` (spaces allowed), or None for a bare argument.
fn key_value(arg: &str) -> Option<(&str, &str)> {
    let (k, v) = arg.split_once('=')?;
    Some((k.trim(), v.trim()))
}

/// A register like `b0`, `t12`, `u3`, `s0`: its letter and number.
fn register(arg: &str) -> Option<(char, u32)> {
    let mut chars = arg.trim().chars();
    let letter = chars.next()?.to_ascii_lowercase();
    let n = chars.as_str().parse().ok()?;
    matches!(letter, 'b' | 't' | 'u' | 's').then_some((letter, n))
}

fn visibility(v: &str) -> Result<i32, String> {
    Ok(match v.trim_start_matches("SHADER_VISIBILITY_") {
        "ALL" => 0,
        "VERTEX" => 1,
        "HULL" => 2,
        "DOMAIN" => 3,
        "GEOMETRY" => 4,
        "PIXEL" => 5,
        "AMPLIFICATION" => 6,
        "MESH" => 7,
        other => return Err(format!("unknown visibility {other:?}")),
    })
}

/// `A|B|...` of named flags, or a number.
fn flag_set(v: &str, names: &[(&str, i32)]) -> Result<i32, String> {
    v.split('|').map(str::trim).try_fold(0i32, |acc, name| {
        if let Ok(n) = name.parse::<i32>() {
            return Ok(acc | n);
        }
        let found = names.iter().find(|(n, _)| *n == name).map(|(_, bits)| *bits);
        found.map(|bits| acc | bits).ok_or_else(|| format!("unknown flag {name:?}"))
    })
}

const ROOT_FLAGS: &[(&str, i32)] = &[
    ("ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT", 0x1),
    ("DENY_VERTEX_SHADER_ROOT_ACCESS", 0x2),
    ("DENY_HULL_SHADER_ROOT_ACCESS", 0x4),
    ("DENY_DOMAIN_SHADER_ROOT_ACCESS", 0x8),
    ("DENY_GEOMETRY_SHADER_ROOT_ACCESS", 0x10),
    ("DENY_PIXEL_SHADER_ROOT_ACCESS", 0x20),
    ("ALLOW_STREAM_OUTPUT", 0x40),
    ("LOCAL_ROOT_SIGNATURE", 0x80),
    ("DENY_AMPLIFICATION_SHADER_ROOT_ACCESS", 0x100),
    ("DENY_MESH_SHADER_ROOT_ACCESS", 0x200),
    ("CBV_SRV_UAV_HEAP_DIRECTLY_INDEXED", 0x400),
    ("SAMPLER_HEAP_DIRECTLY_INDEXED", 0x800),
];
const DESCRIPTOR_FLAGS: &[(&str, i32)] =
    &[("DATA_VOLATILE", 0x2), ("DATA_STATIC_WHILE_SET_AT_EXECUTE", 0x4), ("DATA_STATIC", 0x8)];
const RANGE_FLAGS: &[(&str, i32)] = &[
    ("DESCRIPTORS_VOLATILE", 0x1),
    ("DATA_VOLATILE", 0x2),
    ("DATA_STATIC_WHILE_SET_AT_EXECUTE", 0x4),
    ("DATA_STATIC", 0x8),
    ("DESCRIPTORS_STATIC_KEEPING_BUFFER_BOUNDS_CHECKS", 0x10000),
];

/// Range and root descriptor types: CBV, SRV, UAV, sampler (D3D12's numbering
/// differs between the two; this is the range type, see [`root_type`]).
fn range_type(name: &str) -> Option<i32> {
    match name {
        "SRV" => Some(0),
        "UAV" => Some(1),
        "CBV" => Some(2),
        "Sampler" => Some(3),
        _ => None,
    }
}

/// The root parameter type of a root descriptor of range type `kind`.
fn root_type(kind: i32) -> i32 {
    match kind {
        2 => 2, // CBV
        0 => 3, // SRV
        _ => 4, // UAV
    }
}

/// The version 1.1 default flags of a root descriptor or a range of `kind`.
fn default_flags(kind: i32) -> i32 {
    match kind {
        1 => 0x2, // UAV: data volatile
        3 => 0,   // samplers
        _ => 0x4, // CBV, SRV: data static while set at execute
    }
}

fn number(v: &str) -> Result<u32, String> {
    v.parse().map_err(|_| format!("{v:?} is not a number"))
}

fn float(v: &str) -> Result<f32, String> {
    v.trim_end_matches('f').parse().map_err(|_| format!("{v:?} is not a number"))
}

fn parse(rs: &str) -> Result<Parsed, String> {
    let mut parsed = Parsed::default();
    for item in split_top(rs) {
        let (name, args) = call(item)?;
        match name {
            "RootFlags" => parsed.flags = flag_set(args.trim(), ROOT_FLAGS)?,
            "RootConstants" => {
                let (mut register_, mut space, mut count, mut vis) = (None, 0, None, 0);
                for arg in split_top(args) {
                    match key_value(arg) {
                        Some(("num32BitConstants", v)) => count = Some(number(v)?),
                        Some(("space", v)) => space = number(v)?,
                        Some(("visibility", v)) => vis = visibility(v)?,
                        Some((k, _)) => return Err(format!("RootConstants: unknown {k:?}")),
                        None => register_ = register(arg).filter(|r| r.0 == 'b').map(|r| r.1),
                    }
                }
                parsed.params.push(ParsedParam::Constants {
                    register: register_.ok_or("RootConstants without a b register")?,
                    space,
                    count: count.ok_or("RootConstants without num32BitConstants")?,
                    visibility: vis,
                });
            }
            "CBV" | "SRV" | "UAV" => {
                let kind = range_type(name).unwrap_or(0);
                let (mut register_, mut space, mut flags, mut vis) = (None, 0, default_flags(kind), 0);
                for arg in split_top(args) {
                    match key_value(arg) {
                        Some(("space", v)) => space = number(v)?,
                        Some(("flags", v)) => flags = flag_set(v, DESCRIPTOR_FLAGS)?,
                        Some(("visibility", v)) => vis = visibility(v)?,
                        Some((k, _)) => return Err(format!("{name}: unknown {k:?}")),
                        None => register_ = register(arg).map(|r| r.1),
                    }
                }
                parsed.params.push(ParsedParam::View {
                    kind,
                    register: register_.ok_or_else(|| format!("{name} without a register"))?,
                    space,
                    flags,
                    visibility: vis,
                });
            }
            "DescriptorTable" => {
                let (mut ranges, mut vis) = (Vec::new(), 0);
                for arg in split_top(args) {
                    if let Some(("visibility", v)) = key_value(arg) {
                        vis = visibility(v)?;
                        continue;
                    }
                    let (range_name, range_args) = call(arg)?;
                    let kind = range_type(range_name).ok_or_else(|| format!("unknown range {range_name:?}"))?;
                    let mut range = ParsedRange {
                        kind,
                        register: 0,
                        space: 0,
                        count: 1,
                        flags: default_flags(kind),
                        offset: u32::MAX, // D3D12_DESCRIPTOR_RANGE_OFFSET_APPEND
                    };
                    let mut seen_register = false;
                    for a in split_top(range_args) {
                        match key_value(a) {
                            Some(("numDescriptors", "unbounded")) => range.count = u32::MAX,
                            Some(("numDescriptors", v)) => range.count = number(v)?,
                            Some(("space", v)) => range.space = number(v)?,
                            Some(("flags", v)) => range.flags = flag_set(v, RANGE_FLAGS)?,
                            Some(("offset", "DESCRIPTOR_RANGE_OFFSET_APPEND")) => range.offset = u32::MAX,
                            Some(("offset", v)) => range.offset = number(v)?,
                            Some((k, _)) => return Err(format!("{range_name} range: unknown {k:?}")),
                            None => {
                                range.register = register(a).ok_or_else(|| format!("bad register {a:?}"))?.1;
                                seen_register = true;
                            }
                        }
                    }
                    if !seen_register {
                        return Err(format!("{range_name} range without a register"));
                    }
                    ranges.push(range);
                }
                parsed.params.push(ParsedParam::Table { ranges, visibility: vis });
            }
            "StaticSampler" => {
                // the defaults the compiler gives a StaticSampler
                let mut s = D3D12_STATIC_SAMPLER_DESC {
                    Filter: D3D12_FILTER(0x55), // FILTER_ANISOTROPIC
                    AddressU: D3D12_TEXTURE_ADDRESS_MODE(1),
                    AddressV: D3D12_TEXTURE_ADDRESS_MODE(1),
                    AddressW: D3D12_TEXTURE_ADDRESS_MODE(1),
                    MipLODBias: 0.0,
                    MaxAnisotropy: 16,
                    ComparisonFunc: D3D12_COMPARISON_FUNC(4), // LESS_EQUAL
                    BorderColor: D3D12_STATIC_BORDER_COLOR(2), // OPAQUE_WHITE
                    MinLOD: 0.0,
                    MaxLOD: f32::MAX,
                    ShaderRegister: 0,
                    RegisterSpace: 0,
                    ShaderVisibility: D3D12_SHADER_VISIBILITY(0),
                };
                let address = |v: &str| -> Result<D3D12_TEXTURE_ADDRESS_MODE, String> {
                    Ok(D3D12_TEXTURE_ADDRESS_MODE(match v.trim_start_matches("TEXTURE_ADDRESS_") {
                        "WRAP" => 1,
                        "MIRROR" => 2,
                        "CLAMP" => 3,
                        "BORDER" => 4,
                        "MIRROR_ONCE" => 5,
                        other => return Err(format!("unknown address mode {other:?}")),
                    }))
                };
                for arg in split_top(args) {
                    match key_value(arg) {
                        Some(("filter", v)) => s.Filter = D3D12_FILTER(filter(v)?),
                        Some(("addressU", v)) => s.AddressU = address(v)?,
                        Some(("addressV", v)) => s.AddressV = address(v)?,
                        Some(("addressW", v)) => s.AddressW = address(v)?,
                        Some(("mipLODBias", v)) => s.MipLODBias = float(v)?,
                        Some(("maxAnisotropy", v)) => s.MaxAnisotropy = number(v)?,
                        Some(("minLOD", v)) => s.MinLOD = float(v)?,
                        Some(("maxLOD", v)) => s.MaxLOD = float(v)?,
                        Some(("space", v)) => s.RegisterSpace = number(v)?,
                        Some(("visibility", v)) => s.ShaderVisibility = D3D12_SHADER_VISIBILITY(visibility(v)?),
                        Some(("comparisonFunc", v)) => {
                            s.ComparisonFunc = D3D12_COMPARISON_FUNC(match v.trim_start_matches("COMPARISON_") {
                                "NEVER" => 1,
                                "LESS" => 2,
                                "EQUAL" => 3,
                                "LESS_EQUAL" => 4,
                                "GREATER" => 5,
                                "NOT_EQUAL" => 6,
                                "GREATER_EQUAL" => 7,
                                "ALWAYS" => 8,
                                other => return Err(format!("unknown comparison {other:?}")),
                            })
                        }
                        Some(("borderColor", v)) => {
                            s.BorderColor = D3D12_STATIC_BORDER_COLOR(match v.trim_start_matches("STATIC_BORDER_COLOR_") {
                                "TRANSPARENT_BLACK" => 0,
                                "OPAQUE_BLACK" => 1,
                                "OPAQUE_WHITE" => 2,
                                other => return Err(format!("unknown border color {other:?}")),
                            })
                        }
                        Some((k, _)) => return Err(format!("StaticSampler: unknown {k:?}")),
                        None => {
                            s.ShaderRegister =
                                register(arg).filter(|r| r.0 == 's').ok_or_else(|| format!("bad sampler register {arg:?}"))?.1
                        }
                    }
                }
                parsed.samplers.push(s);
            }
            other => return Err(format!("unknown item {other:?}")),
        }
    }
    Ok(parsed)
}

/// D3D12_FILTER by its name in a root signature string (the ones seen in
/// shaders; others are refused rather than guessed).
fn filter(v: &str) -> Result<i32, String> {
    Ok(match v {
        "FILTER_MIN_MAG_MIP_POINT" => 0x0,
        "FILTER_MIN_MAG_POINT_MIP_LINEAR" => 0x1,
        "FILTER_MIN_POINT_MAG_LINEAR_MIP_POINT" => 0x4,
        "FILTER_MIN_POINT_MAG_MIP_LINEAR" => 0x5,
        "FILTER_MIN_LINEAR_MAG_MIP_POINT" => 0x10,
        "FILTER_MIN_LINEAR_MAG_POINT_MIP_LINEAR" => 0x11,
        "FILTER_MIN_MAG_LINEAR_MIP_POINT" => 0x14,
        "FILTER_MIN_MAG_MIP_LINEAR" => 0x15,
        "FILTER_ANISOTROPIC" => 0x55,
        "FILTER_COMPARISON_MIN_MAG_MIP_POINT" => 0x80,
        "FILTER_COMPARISON_MIN_MAG_LINEAR_MIP_POINT" => 0x94,
        "FILTER_COMPARISON_MIN_MAG_MIP_LINEAR" => 0x95,
        "FILTER_COMPARISON_ANISOTROPIC" => 0xd5,
        other => return Err(format!("unknown filter {other:?}")),
    })
}

/// `parsed` as a version 1.1 root signature, serialized and made.
fn serialize(device: &ID3D12Device, parsed: &Parsed, what: &str) -> windows::core::Result<ID3D12RootSignature> {
    // The ranges first, per table: the parameters point into them.
    let ranges: Vec<Vec<D3D12_DESCRIPTOR_RANGE1>> = parsed
        .params
        .iter()
        .map(|p| match p {
            ParsedParam::Table { ranges, .. } => ranges
                .iter()
                .map(|r| D3D12_DESCRIPTOR_RANGE1 {
                    RangeType: D3D12_DESCRIPTOR_RANGE_TYPE(r.kind),
                    NumDescriptors: r.count,
                    BaseShaderRegister: r.register,
                    RegisterSpace: r.space,
                    Flags: D3D12_DESCRIPTOR_RANGE_FLAGS(r.flags),
                    OffsetInDescriptorsFromTableStart: r.offset,
                })
                .collect(),
            _ => Vec::new(),
        })
        .collect();
    let built: Vec<D3D12_ROOT_PARAMETER1> = parsed
        .params
        .iter()
        .zip(&ranges)
        .map(|(p, table)| match *p {
            ParsedParam::Constants { register, space, count, visibility } => D3D12_ROOT_PARAMETER1 {
                ParameterType: D3D12_ROOT_PARAMETER_TYPE_32BIT_CONSTANTS,
                Anonymous: D3D12_ROOT_PARAMETER1_0 {
                    Constants: D3D12_ROOT_CONSTANTS { ShaderRegister: register, RegisterSpace: space, Num32BitValues: count },
                },
                ShaderVisibility: D3D12_SHADER_VISIBILITY(visibility),
            },
            ParsedParam::View { kind, register, space, flags, visibility } => D3D12_ROOT_PARAMETER1 {
                ParameterType: D3D12_ROOT_PARAMETER_TYPE(root_type(kind)),
                Anonymous: D3D12_ROOT_PARAMETER1_0 {
                    Descriptor: D3D12_ROOT_DESCRIPTOR1 {
                        ShaderRegister: register,
                        RegisterSpace: space,
                        Flags: D3D12_ROOT_DESCRIPTOR_FLAGS(flags),
                    },
                },
                ShaderVisibility: D3D12_SHADER_VISIBILITY(visibility),
            },
            ParsedParam::Table { visibility, .. } => D3D12_ROOT_PARAMETER1 {
                ParameterType: D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE,
                Anonymous: D3D12_ROOT_PARAMETER1_0 {
                    DescriptorTable: D3D12_ROOT_DESCRIPTOR_TABLE1 {
                        NumDescriptorRanges: table.len() as u32,
                        pDescriptorRanges: table.as_ptr(),
                    },
                },
                ShaderVisibility: D3D12_SHADER_VISIBILITY(visibility),
            },
        })
        .collect();
    let desc = D3D12_VERSIONED_ROOT_SIGNATURE_DESC {
        Version: D3D_ROOT_SIGNATURE_VERSION_1_1,
        Anonymous: D3D12_VERSIONED_ROOT_SIGNATURE_DESC_0 {
            Desc_1_1: D3D12_ROOT_SIGNATURE_DESC1 {
                NumParameters: built.len() as u32,
                pParameters: built.as_ptr(),
                NumStaticSamplers: parsed.samplers.len() as u32,
                pStaticSamplers: parsed.samplers.as_ptr(),
                Flags: D3D12_ROOT_SIGNATURE_FLAGS(parsed.flags),
            },
        },
    };
    let mut blob: Option<ID3DBlob> = None;
    let mut error: Option<ID3DBlob> = None;
    // SAFETY: `desc` points at the live `built`, `ranges` and samplers; the
    // blobs are read in full before they are released.
    unsafe {
        if let Err(e) = D3D12SerializeVersionedRootSignature(&desc, &mut blob, Some(&mut error)) {
            if let Some(error) = error {
                let text = std::slice::from_raw_parts(error.GetBufferPointer().cast::<u8>(), error.GetBufferSize());
                tracing::warn!("{what} root signature: {}", String::from_utf8_lossy(text));
            }
            return Err(e);
        }
        let blob = blob.ok_or_else(windows::core::Error::empty)?;
        let bytes = std::slice::from_raw_parts(blob.GetBufferPointer().cast::<u8>(), blob.GetBufferSize());
        device.CreateRootSignature(0, bytes)
    }
}

#[cfg(test)]
mod string_tests {
    use super::*;

    #[test]
    fn every_root_signature_string_in_the_runtime_parses() {
        for rs in [
            "UAV(u0)",
            "RootFlags(0)",
            "DescriptorTable(SRV(t0), visibility = SHADER_VISIBILITY_PIXEL)",
            "RootConstants(num32BitConstants=6, b0), SRV(t0), UAV(u0), UAV(u1)",
            "RootConstants(num32BitConstants=4,b0), DescriptorTable(SRV(t0,numDescriptors=4),CBV(b1)), UAV(u0), StaticSampler(s0,filter=FILTER_MIN_MAG_MIP_LINEAR,addressU=TEXTURE_ADDRESS_WRAP,addressV=TEXTURE_ADDRESS_WRAP)",
            "RootConstants(num32BitConstants=2,b0), DescriptorTable(SRV(t0,numDescriptors=4,flags=DESCRIPTORS_VOLATILE)), UAV(u0), StaticSampler(s0,filter=FILTER_MIN_MAG_MIP_POINT,addressU=TEXTURE_ADDRESS_WRAP,addressV=TEXTURE_ADDRESS_WRAP,addressW=TEXTURE_ADDRESS_WRAP)",
            "RootConstants(num32BitConstants = 1, b0)",
            "RootConstants(num32BitConstants=12, b0), DescriptorTable(SRV(t0)), DescriptorTable(UAV(u0)), UAV(u1), UAV(u2)",
            "DescriptorTable(SRV(t0,numDescriptors=64,flags=DESCRIPTORS_VOLATILE|DATA_VOLATILE)), UAV(u0)",
            "DescriptorTable(SRV(t0, numDescriptors = 2)), UAV(u0), StaticSampler(s0, filter = FILTER_MIN_MAG_MIP_POINT)",
            "DescriptorTable(CBV(b0, numDescriptors=128)), UAV(u0)",
        ] {
            parse(rs).unwrap_or_else(|e| panic!("{rs}: {e}"));
        }
    }

    #[test]
    fn the_compare_string_is_constants_then_three_root_views_with_their_default_flags() {
        let p = parse("RootConstants(num32BitConstants=6, b0), SRV(t0), UAV(u0), UAV(u1)").unwrap();
        assert_eq!(p.params.len(), 4);
        assert_eq!(p.params[0], ParsedParam::Constants { register: 0, space: 0, count: 6, visibility: 0 });
        assert_eq!(p.params[1], ParsedParam::View { kind: 0, register: 0, space: 0, flags: 0x4, visibility: 0 });
        assert_eq!(p.params[3], ParsedParam::View { kind: 1, register: 1, space: 0, flags: 0x2, visibility: 0 });
    }

    #[test]
    fn a_table_keeps_its_ranges_in_order_with_explicit_flags() {
        let p = parse("DescriptorTable(SRV(t0,numDescriptors=4,flags=DESCRIPTORS_VOLATILE),CBV(b1)), UAV(u0)").unwrap();
        let ParsedParam::Table { ranges, .. } = &p.params[0] else { panic!("a table") };
        assert_eq!(ranges.len(), 2);
        assert_eq!((ranges[0].kind, ranges[0].count, ranges[0].flags), (0, 4, 0x1));
        assert_eq!((ranges[1].kind, ranges[1].register, ranges[1].flags), (2, 1, 0x4));
    }

    #[test]
    fn the_rs_define_is_found_in_a_shader_source() {
        let src = "// x\n#define RS \"UAV(u0), SRV(t1)\"\n[RootSignature(RS)]\n";
        assert_eq!(define_rs(src), Some("UAV(u0), SRV(t1)"));
    }

    #[test]
    fn unknown_items_are_refused() {
        assert!(parse("Bogus(u0)").is_err());
        assert!(parse("StaticSampler(s0, filter=FILTER_SOMETHING_NEW)").is_err());
    }
}
