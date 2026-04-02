# Asset Format

## Canonical Runtime Format

The loader normalizes supported source metadata into `VatAnimationData`.

Required top-level fields in the canonical JSON format:

- `format`
  - Accepted: `vertex_animation_texture@1`, `vertex_animation_texture`
- `animation_mode`
  - Currently supported runtime mode: `soft_body_fixed_topology`
- `vertex_count`
- `frame_count`
- `frames_per_second` or `seconds_per_frame`
- `decode_bounds`
- `clips`
- `position_texture`
- `coordinate_system`
- `playback_space`
- `vertex_id_attribute`
- `position_encoding`

Optional top-level fields:

- `animation_bounds`
- `normal_texture`
- `rotation_texture`
- `auxiliary_textures`

## Canonical Example

```json
{
  "format": "vertex_animation_texture@1",
  "animation_mode": "soft_body_fixed_topology",
  "vertex_count": 81,
  "frame_count": 72,
  "frames_per_second": 24.0,
  "decode_bounds": {
    "min": [-0.85, -0.1, -0.42],
    "max": [0.85, 1.75, 0.42]
  },
  "clips": [
    { "name": "idle", "start_frame": 0, "end_frame": 23 }
  ],
  "position_texture": {
    "relative_path": "wave_positions.png",
    "width": 81,
    "height": 72,
    "rows_per_frame": 1,
    "precision": "png8"
  },
  "coordinate_system": "y_up_right_handed",
  "playback_space": "local",
  "vertex_id_attribute": "uv1",
  "position_encoding": "absolute_normalized_bounds"
}
```

## OpenVAT-Compatible Subset

Accepted today:

- `os-remap.Min`
- `os-remap.Max`
- `os-remap.Frames`
- `animations`
- plus runtime-required layout fields:
  - `vertex_count`
  - `texture_width`
  - `texture_height`
  - optional `rows_per_frame`
  - optional `packed_normals`
  - optional `normal_row_offset`

This subset is intentionally explicit because raw remap JSON alone is not enough for runtime validation.

## Validation Rules

Metadata validation rejects:

- non-positive FPS
- zero frames
- zero vertices
- empty clip list
- invalid decode or animation bounds
- position textures that cannot address `vertex_count` texels per frame
- position textures whose `height` and `rows_per_frame` do not cover all baked frames
- clip ranges outside `frame_count`
- event frames outside clip-local ranges
- invalid packed-normal row offsets
- unsupported rigid-body mode in v0.1
- unknown loop modes, texture precision strings, normal encodings, normal texture modes, or auxiliary texture semantics in canonical JSON

Mesh validation rejects:

- missing positions
- missing normals
- missing UV0
- missing UV1
- proxy mesh vertex count mismatches
- malformed UV1 format
- UV1 data that does not map to the expected number of unique VAT texels

## Coordinate and Encoding Assumptions

- UV1 is interpreted as the VAT lookup channel
  - in DCC terms this is usually “UV2”
- `absolute_normalized_bounds`
  - sampled values decode directly into absolute source-space positions
- `offset_normalized_bounds`
  - sampled values decode into offsets added to the proxy vertex position
- `coordinate_system`
  - `z_up_right_handed` is converted to Bevy’s Y-up basis at decode time

## Precision Profiles

- `exr_half`
  - preferred for production bakes
  - best precision, higher memory
- `png16`
  - acceptable compromise when the toolchain supports it cleanly
- `png8`
  - smallest and easiest demo path
  - highest quantization error

## Current Non-Goals

Documented but not fully implemented in v0.1:

- rigid-body rotation textures
- auxiliary shading channels in the runtime shader path
- dynamic-remesh fluid VAT
- streamed clip windowing
