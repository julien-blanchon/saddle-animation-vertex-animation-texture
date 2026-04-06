# Troubleshooting

Common issues when working with Vertex Animation Textures and how to fix them.

## Visual Issues

### Vertices "swim" or jitter

**Symptom**: vertices wobble or drift slightly from their expected positions.

**Causes and fixes**:

1. **Low texture precision**: 8-bit PNG textures have only 256 steps per axis. Switch to
   PNG 16-bit or EXR half-float.

2. **Large decode bounds**: quantization error scales with the bounding box size. If your
   `decode_bounds` min/max span a large range, each quantization step covers more world
   space. Solutions:
   - Use `offset_normalized_bounds` encoding (smaller per-vertex deltas)
   - Tighten the bounds to the actual animation range
   - Use higher-precision textures

3. **UV1 precision loss**: if the mesh has many vertices, half-precision UVs may round the
   texel lookup. Ensure your export uses full-precision UVs.

### Mesh appears corrupted or exploded

**Symptom**: vertices fly to random positions, mesh looks like a spiky ball or scattered
points.

**Causes and fixes**:

1. **Wrong texture filtering**: VAT textures must use nearest/point sampling. Bilinear
   filtering interpolates between adjacent vertices' data, producing garbage. Fix:
   ```rust
   use saddle_animation_vertex_animation_texture::configure_vat_data_image;
   // Call on your loaded Image assets
   configure_vat_data_image(&mut image);
   ```

2. **Mismatched vertex count**: the proxy mesh vertex count doesn't match the metadata
   `vertex_count`. This often happens because DCC export splits vertices at hard edges or
   UV seams. Check the export log and update the metadata to match the actual exported
   vertex count.

3. **Wrong decode bounds**: if `decode_bounds` min/max don't match the values used during
   the bake, positions will be decoded incorrectly. Copy the exact values from your DCC
   tool's output (the `remap_info.json` for OpenVAT, or the mesh bounds for Houdini).

4. **Coordinate system mismatch**: if the texture was baked in Z-up (Blender) but the
   metadata says `y_up_right_handed`, the Y and Z axes will be swapped. Set
   `coordinate_system` to `z_up_right_handed` for Blender exports.

5. **Missing or wrong UV1**: the mesh doesn't have a second UV channel, or UV1 was
   overwritten after the bake. Re-export from the DCC tool.

### Normals look wrong (flat shading, dark faces)

**Symptom**: the mesh animates correctly but lighting is flat, inverted, or produces dark
patches.

**Causes and fixes**:

1. **No normal texture loaded**: if metadata declares `normal_texture: "separate"` but you
   haven't loaded the normal texture, the shader falls back to proxy normals which won't
   match the deformed surface. Load and configure the normal texture.

2. **Normal texture uses wrong filtering**: like position textures, normal textures must use
   nearest filtering. Apply `configure_vat_data_image()`.

3. **sRGB color space**: VAT data textures must be loaded as linear (non-sRGB). If the
   image loader applies sRGB-to-linear conversion, the data will be corrupted. EXR files
   are inherently linear. For PNG files, ensure they're loaded without sRGB conversion.

4. **Packed normals row offset wrong**: if using packed normals, verify that
   `normal_row_offset` in the metadata matches the actual texture layout.

### Animation plays at wrong speed

**Symptom**: animation is too fast, too slow, or doesn't match the source.

**Causes and fixes**:

1. **Wrong `frames_per_second`**: the metadata FPS must match the DCC tool's bake FPS. If
   you baked at 30 FPS but the metadata says 24, the animation will play 20% too slow.

2. **Frame count mismatch**: verify `frame_count` matches the actual number of baked frames
   (texture height / rows_per_frame).

3. **Playback speed**: check `VatPlayback::speed`. The default is `1.0`.

### Entity disappears when it should be visible

**Symptom**: the entity vanishes when the camera moves, or is only visible from certain
angles.

**Cause**: frustum culling uses bounds that don't account for the animated vertex range.

**Fix**: set `animation_bounds` in the metadata to encompass all vertex positions across all
frames (with some padding). Or use `VatBoundsMode::DisableFrustumCulling`:

```rust
VatAnimationSource::new(metadata)
    .with_bounds_mode(VatBoundsMode::DisableFrustumCulling)
```

World-space playback automatically disables frustum culling.

### Crossfade produces a visual "pop"

**Symptom**: sharp discontinuity at the start or end of a crossfade transition.

**Causes and fixes**:

1. **Crossfade duration too short**: very short durations (< 0.1s) may not produce visible
   blending. Increase the duration.

2. **Source and target poses very different**: VAT crossfade blends vertex positions
   linearly, which can produce artifacts when the source and target poses are radically
   different (e.g., standing to lying down). This is an inherent limitation. Strategies:
   - Add transition clips that bridge between poses
   - Use longer crossfade durations to smooth the blend

## Loading Errors

### "metadata declares separate normal texture but no handle supplied"

The metadata JSON declares `"normal_texture": { "mode": "separate", ... }` but you didn't
pass a normal texture handle to `build_vat_material()`. Either:

- Load the normal texture and pass it as `Some(handle)`
- Change the metadata to `"normal_texture": { "mode": "none" }` if you don't need animated
  normals

### "VAT binding validation failed"

The system detected a mismatch between the mesh and metadata. Common sub-errors:

- **"proxy mesh vertex count X does not match metadata vertex count Y"**: re-export the mesh
  or update the metadata.
- **"missing UV1 attribute"**: the mesh doesn't have a second UV channel. Re-export from the
  DCC tool with UV1 baked.
- **"UV1 format not supported"**: UV1 must be `Float32x2`. Some exporters use half-precision;
  check your export settings.

### "requested clip index N but metadata only has M clips"

`VatPlayback.active_clip` is set to an index that doesn't exist. Clip indices are 0-based.
With 3 clips, valid indices are 0, 1, 2.

Preferred fixes:

- Switch to `VatPlayback::with_clip_name(...)` or `play_clip_named(...)` so metadata resolves the clip by name.
- Add `"default_clip": "..."` to metadata and let `VatPlayback::default()` resolve from metadata.
- If numeric indices are unavoidable, set `VatPlayback::invalid_clip_fallback` explicitly so recovery behavior is deliberate.

## Performance Issues

### Low FPS with many VAT instances

1. **Check draw calls**: instances sharing the same material are batched. If each instance
   has a unique material, you lose batching. Ensure crowd members share a single
   `Handle<VatMaterial>`.

2. **Storage buffer size**: the per-entity storage buffer grows with instance count. This is
   lightweight (48 bytes per entity) but verify the GPU upload isn't a bottleneck.

3. **Texture size**: very large VAT textures (>4K) consume GPU memory and bandwidth. Consider
   reducing frame count, vertex count, or using lower precision.

### Compilation / shader warmup stutter

The first frame rendering a VAT material may stutter due to shader compilation. This is a
general Bevy/wgpu behavior, not specific to VAT. Strategies:

- Use shader warmup / pipeline caching (Bevy's built-in mechanisms)
- Spawn a tiny offscreen VAT entity early to trigger compilation before it's visible

## DCC Tool Issues

### OpenVAT: exported vertex count differs from Blender

Blender splits vertices at hard edges and UV seams during export. The exported `.glb` may
have more vertices than Blender's edit mode shows. This is normal. Use the post-export
vertex count in your metadata.

### Houdini: textures look different in engine

Houdini VAT 3.0 can produce HDR (EXR) or non-HDR (PNG) textures. Ensure the texture format
in metadata matches what was exported. EXR textures provide better precision but require the
engine to support EXR loading.

### Animations don't loop cleanly

If there's a visible "hitch" at the loop point:

1. **Frame count**: for a clean loop, the last frame should smoothly connect to the first.
   Some tools bake `N+1` frames (the last frame equals the first). In this case, set
   `end_frame` to `N-1` (the last unique frame) so the loop skips the duplicate.

2. **Loop mode**: ensure the clip uses `"default_loop_mode": "loop"` in metadata, or set
   `VatPlayback::loop_mode` to `VatLoopMode::Loop`.

## Modular Sync Issues

### Follower doesn't track the leader

**Symptom**: a `VatPlaybackFollower` entity plays independently instead of mirroring.

**Causes and fixes**:

1. **Leader entity ID is wrong**: `VatPlaybackFollower::new(leader)` requires the exact `Entity`
   ID of the leader. If the leader was spawned in a different system or frame, ensure you're
   passing the correct entity. Check with `commands.entity(leader).id()`.

2. **Leader has no `VatPlayback`**: the follower sync system reads the leader's `VatPlayback`.
   If the leader is missing this component, the follower will be skipped silently.

3. **Leader despawned**: if the leader entity was despawned, the follower can't find it. The
   follower will continue playing independently in this case.

### Follower crossfade out of sync

**Symptom**: leader and follower crossfade at different speeds or to different clips.

**Fix**: ensure `mirror_crossfade` is `true` (default). The follower sync system copies the
exact crossfade state from the leader, including elapsed time and duration.

## Programmatic VAT Issues

### Texture data looks correct but mesh doesn't animate

**Symptom**: you built the VAT texture in code but the mesh stays in its rest pose.

**Causes and fixes**:

1. **Missing `MeshMaterial3d<VatMaterial>`**: the entity must use the VAT material, not a
   standard `StandardMaterial`. Without the VAT material, the vertex shader never samples the
   animation texture.

2. **Storage buffer not populated**: `build_vat_material()` creates an initial single-entry
   storage buffer. The `sync_gpu_state` system updates it each frame. Ensure the plugin is
   added and the entity has both `VatAnimationSource` and `VatPlayback`.

3. **Texture not configured for nearest sampling**: call `configure_vat_data_image()` on your
   generated `Image` or use `make_linear_rgba8_image()` which does this automatically.

4. **UV1 coordinates off by half a texel**: UV1 values must point to texel centers, not edges.
   The formula is `(texel_index + 0.5) / texture_dimension`. An off-by-one here causes each
   vertex to read its neighbor's animation data.

### Position data encodes correctly but decodes wrong

**Symptom**: `decode_position_sample()` returns wrong values for hand-crafted texture data.

**Causes and fixes**:

1. **8-bit quantization**: Rgba8Unorm stores values as `u8 / 255.0`. The round-trip
   `encode → store → decode` loses precision. For values close to 0 or 1, the quantization
   error can shift vertices noticeably. Use higher precision formats for production.

2. **Bounds mismatch**: the `decode_bounds_min` and `decode_bounds_max` in the metadata must
   exactly match what was used to encode the texture data. Even a small mismatch scales all
   positions incorrectly.

## Build and Integration Issues

### "No method named `add_message` found"

You're on an older Bevy version. This crate requires Bevy 0.18 which introduced the `Message`
trait and `add_message()`. Upgrade your Bevy dependency.

### Shader compilation errors on first frame

The `ExtendedMaterial<StandardMaterial, VatMaterialExt>` shader compiles on first use, which
can cause a frame stutter. This is standard Bevy/wgpu behavior. To mitigate:

- Spawn a tiny offscreen VAT entity during loading to trigger compilation early
- Use Bevy's pipeline caching if available

### Multiple VAT materials cause separate draw calls

Each unique `Handle<VatMaterial>` produces a separate draw call and storage buffer. For crowds,
all instances should share one material handle:

```rust
// GOOD: one material, many instances
let material = materials.add(vat_material);
for i in 0..1000 {
    commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), ...));
}

// BAD: 1000 materials, 1000 draw calls
for i in 0..1000 {
    let material = materials.add(vat_material.clone()); // unique handle each time
    commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material), ...));
}
```

## Validation Utilities

The crate provides validation helpers you can call manually for debugging:

```rust
use saddle_animation_vertex_animation_texture::*;

// Validate metadata internally
animation_data.validate()?;

// Validate mesh against metadata
validate_mesh_for_animation(&mesh, &animation_data)?;

// Decode a specific sample for manual inspection
let position = decode_position_sample(
    encoded_sample,
    &animation_data,
    proxy_vertex_position,
);
```

Use `decode_position_sample()` to spot-check that a specific encoded sample decodes to the
expected world-space position from your DCC tool.
