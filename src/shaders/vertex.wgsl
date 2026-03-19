// ===================================================
// Reference vertex shader for the Slug algorithm.
// Translated from HLSL to WGSL for wgpu.
// Copyright 2017, by Eric Lengyel. MIT License.
// ===================================================

// Vertex attributes: pos, tex, jac, bnd, col (5 x vec4)

fn slug_unpack(tex: vec4f, bnd: vec4f) -> vec2f {
    // Unpack glyph data from tex.zw - handled in struct
    return bnd.xy;
}

struct DilateResult {
    texcoord: vec2f,
    vpos: vec2f,
}

fn slug_dilate(
    pos: vec4f,
    tex: vec4f,
    jac: vec4f,
    m0: vec4f,
    m1: vec4f,
    m3: vec4f,
    dim: vec2f,
) -> DilateResult {
    let n = normalize(vec2f(pos.z, pos.w));
    let s = dot(vec2f(m3.x, m3.y), vec2f(pos.x, pos.y)) + m3.w;
    let t = dot(vec2f(m3.x, m3.y), n);

    let u = (s * dot(vec2f(m0.x, m0.y), n) - t * (dot(vec2f(m0.x, m0.y), vec2f(pos.x, pos.y)) + m0.w)) * dim.x;
    let v = (s * dot(vec2f(m1.x, m1.y), n) - t * (dot(vec2f(m1.x, m1.y), vec2f(pos.x, pos.y)) + m1.w)) * dim.y;

    let s2 = s * s;
    let st = s * t;
    let uv = u * u + v * v;
    let d = vec2f(pos.z, pos.w) * (s2 * (st + sqrt(uv)) / (uv - st * st));

    var result: DilateResult;
    result.vpos = vec2f(pos.x, pos.y) + d;
    result.texcoord = vec2f(tex.x + dot(d, vec2f(jac.x, jac.y)), tex.y + dot(d, vec2f(jac.z, jac.w)));
    return result;
}

struct Params {
    slug_matrix: mat4x4f,
    slug_viewport: vec4f,
}

struct VertexInput {
    @location(0) pos: vec4f,
    @location(1) tex: vec4f,
    @location(2) jac: vec4f,
    @location(3) bnd: vec4f,
    @location(4) col: vec4f,
}

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f,
    @location(1) texcoord: vec2f,
    @location(2) @interpolate(flat) banding: vec4f,
    @location(3) @interpolate(flat) glyph: vec4i,
}

@group(0) @binding(0) var<uniform> params: Params;

// Set to true for proper Slug dilation; false bypasses it for debugging
const kUseDilate = false;

@vertex
fn main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    var vpos: vec2f;
    var texcoord: vec2f;
    if (kUseDilate) {
        let p = slug_dilate(
            input.pos,
            input.tex,
            input.jac,
            params.slug_matrix[0],
            params.slug_matrix[1],
            params.slug_matrix[3],
            params.slug_viewport.xy,
        );
        vpos = p.vpos;
        texcoord = p.texcoord;
    } else {
        vpos = vec2f(input.pos.x, input.pos.y);
        // Pass glyph-local coords so curve lookup matches (curves are stored per-glyph in local space)
        texcoord = vec2f(input.pos.x - input.jac.x, input.pos.y - input.jac.y);
    }
    out.texcoord = texcoord;

    // Apply MVP matrix (column-major from glam)
    out.position = params.slug_matrix * vec4f(vpos.x, vpos.y, 0.0, 1.0);

    // SlugUnpack: tex.z = (gy<<16)|gx, tex.w = (band_max_y<<16)|band_max_x
    let gx_bits = bitcast<u32>(input.tex.z);
    let gy_bits = bitcast<u32>(input.tex.w);
    out.glyph = vec4i(
        i32(gx_bits & 0xFFFFu),   // glyphData.x = gx
        i32(gx_bits >> 16u),      // glyphData.y = gy
        i32(gy_bits & 0xFFFFu),   // glyphData.z = band_max_x
        i32(gy_bits >> 16u),      // glyphData.w = band_max_y + flags
    );
    out.banding = input.bnd;
    out.color = input.col;
    return out;
}
