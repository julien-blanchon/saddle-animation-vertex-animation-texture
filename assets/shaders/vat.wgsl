#import bevy_pbr::{
    forward_io::{Vertex, VertexOutput},
    mesh_bindings::mesh,
    mesh_functions,
    view_transformations::position_world_to_clip,
}

struct VatMaterialUniform {
    decode_min: vec4<f32>,
    decode_extent: vec4<f32>,
    texture_layout: vec4<u32>,
    normal_layout: vec4<u32>,
    modes: vec4<u32>,
}

struct VatInstanceState {
    primary_frames: vec4<f32>,
    secondary_frames: vec4<f32>,
    options: vec4<f32>,
}

struct VatSample {
    position: vec3<f32>,
    normal: vec3<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var vat_position_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(101)
var vat_position_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(102)
var vat_normal_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(103)
var vat_normal_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(104)
var<uniform> vat_material: VatMaterialUniform;
@group(#{MATERIAL_BIND_GROUP}) @binding(105)
var<storage, read> vat_instances: array<VatInstanceState>;

fn convert_source_to_bevy(value: vec3<f32>) -> vec3<f32> {
    if vat_material.modes.y == 1u {
        return vec3<f32>(value.x, value.z, -value.y);
    }
    return value;
}

fn vertex_texel(uv_b: vec2<f32>) -> vec2<u32> {
    let width = f32(vat_material.texture_layout.x);
    let height = f32(vat_material.texture_layout.y);
    return vec2<u32>(
        u32(round(uv_b.x * width - 0.5)),
        u32(round(uv_b.y * height - 0.5)),
    );
}

fn texel_center(texel: vec2<u32>, texture_size: vec2<u32>) -> vec2<f32> {
    return vec2<f32>(
        (f32(texel.x) + 0.5) / f32(texture_size.x),
        (f32(texel.y) + 0.5) / f32(texture_size.y),
    );
}

fn sample_position_frame(frame_index: f32, uv_b: vec2<f32>, proxy_position: vec3<f32>) -> vec3<f32> {
    let vertex_lookup = vertex_texel(uv_b);
    let row = u32(frame_index) * vat_material.texture_layout.z + vertex_lookup.y;
    let uv = texel_center(vec2<u32>(vertex_lookup.x, row), vat_material.texture_layout.xy);
    let encoded = textureSampleLevel(vat_position_texture, vat_position_sampler, uv, 0.0).xyz;
    var decoded = vat_material.decode_min.xyz + encoded * vat_material.decode_extent.xyz;
    decoded = convert_source_to_bevy(decoded);
    if vat_material.modes.x == 1u {
        decoded += proxy_position;
    }
    return decoded;
}

fn sample_normal_frame(frame_index: f32, uv_b: vec2<f32>, fallback_normal: vec3<f32>) -> vec3<f32> {
    if vat_material.modes.w == 0u {
        return fallback_normal;
    }

    let vertex_lookup = vertex_texel(uv_b);
    var encoded: vec3<f32>;
    if vat_material.modes.w == 1u {
        let row = u32(frame_index) * vat_material.texture_layout.z + vat_material.normal_layout.w + vertex_lookup.y;
        let uv = texel_center(vec2<u32>(vertex_lookup.x, row), vat_material.texture_layout.xy);
        encoded = textureSampleLevel(vat_position_texture, vat_position_sampler, uv, 0.0).xyz;
    } else {
        let row = u32(frame_index) * vat_material.normal_layout.z + vertex_lookup.y;
        let uv = texel_center(vec2<u32>(vertex_lookup.x, row), vat_material.normal_layout.xy);
        encoded = textureSampleLevel(vat_normal_texture, vat_normal_sampler, uv, 0.0).xyz;
    }

    let decoded = normalize(convert_source_to_bevy(encoded * 2.0 - vec3<f32>(1.0)));
    return decoded;
}

fn sample_clip(
    frame_a: f32,
    frame_b: f32,
    blend: f32,
    uv_b: vec2<f32>,
    proxy_position: vec3<f32>,
    proxy_normal: vec3<f32>,
) -> VatSample {
    let position_a = sample_position_frame(frame_a, uv_b, proxy_position);
    let position_b = sample_position_frame(frame_b, uv_b, proxy_position);
    let normal_a = sample_normal_frame(frame_a, uv_b, proxy_normal);
    let normal_b = sample_normal_frame(frame_b, uv_b, proxy_normal);

    return VatSample(
        mix(position_a, position_b, blend),
        normalize(mix(normal_a, normal_b, blend)),
    );
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let mesh_world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let vat_index = mesh_functions::get_tag(vertex.instance_index);
    let vat_instance = vat_instances[vat_index];
    let primary = sample_clip(
        vat_instance.primary_frames.x,
        vat_instance.primary_frames.y,
        vat_instance.primary_frames.z * vat_instance.options.x,
        vertex.uv_b,
        vertex.position,
        vertex.normal,
    );

    var local_position = primary.position;
    var local_normal = primary.normal;

    if vat_instance.options.z > 0.5 {
        let secondary = sample_clip(
            vat_instance.secondary_frames.x,
            vat_instance.secondary_frames.y,
            vat_instance.secondary_frames.z * vat_instance.options.x,
            vertex.uv_b,
            vertex.position,
            vertex.normal,
        );
        local_position = mix(local_position, secondary.position, vat_instance.options.y);
        local_normal = normalize(mix(local_normal, secondary.normal, vat_instance.options.y));
    }

    if vat_material.modes.z == 1u {
        out.world_position = vec4<f32>(local_position, 1.0);
        out.world_normal = normalize(local_normal);
    } else {
        out.world_position =
            mesh_functions::mesh_position_local_to_world(mesh_world_from_local, vec4<f32>(local_position, 1.0));
        out.world_normal = mesh_functions::mesh_normal_local_to_world(local_normal, vertex.instance_index);
    }

    out.position = position_world_to_clip(out.world_position.xyz);

#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif
#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        mesh_world_from_local,
        vertex.tangent,
        vertex.instance_index,
    );
#endif
#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither =
        mesh_functions::get_visibility_range_dither_level(vertex.instance_index, mesh_world_from_local[3]);
#endif

    return out;
}
