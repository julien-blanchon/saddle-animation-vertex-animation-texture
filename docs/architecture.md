# Architecture

## Overview

`saddle-animation-vertex-animation-texture` splits VAT playback into four layers:

1. Metadata
   - `VatAnimationData` is the canonical runtime description of a bake.
   - JSON loaders normalize source metadata into this one internal shape.
2. ECS playback
   - `VatPlayback` and `VatCrossfade` are the consumer-facing state.
   - Systems advance time, resolve loop policy, emit events, and prepare GPU frame selections.
3. Material state
   - `VatMaterialExt` owns the data textures, decode uniforms, and a storage buffer handle.
   - `MeshTag` is used as the per-entity index into the storage buffer.
4. Shader deformation
   - The vertex shader samples frame A and frame B, interpolates, optionally crossfades a secondary clip, and feeds the result into Bevy PBR.

## Runtime Data Flow

1. Load or construct `VatAnimationData`.
2. Build a `VatMaterial` with `build_vat_material(...)`.
3. Spawn a mesh with:
   - `Mesh3d`
   - `MeshMaterial3d<VatMaterial>`
   - `VatAnimationSource`
   - `VatPlayback`
4. `VatSystems::AdvancePlayback`
   - advances clip-local time
   - applies loop policy
   - advances crossfade source state
   - records pending events / finish notifications
5. `VatSystems::ResolveTransitions`
   - progresses and clears completed crossfades
6. `VatSystems::EmitMessages`
   - emits `VatClipFinished`
   - emits `VatEventReached`
7. `VatSystems::SyncGpuState`
   - validates mesh / metadata compatibility
   - applies metadata-driven bounds and frustum-culling policy
   - groups entities by material handle
   - writes one storage-buffer entry per entity
   - assigns `MeshTag` so the shader can index the right entry

## Material / Shader Path

The primary render path is:

`ExtendedMaterial<StandardMaterial, VatMaterialExt>`

Bindings:

- `100`: position texture
- `101`: position sampler
- `102`: normal texture
- `103`: normal sampler
- `104`: decode/layout uniform
- `105`: per-entity storage buffer

The shader uses mesh UV1 as the baked vertex lookup channel. UV1 is interpreted as a texel-center lookup into the first frame layout. The shader then offsets the row by `frame_index * rows_per_frame`.

## Bounds and Culling

Static proxy bounds are usually wrong for VAT motion extremes. The crate addresses this in two ways:

- `VatBoundsMode::UseMetadataAabb`
  - Inserts an `Aabb` derived from `VatAnimationData::animation_bounds_*`
- `VatBoundsMode::DisableFrustumCulling`
  - Adds `NoFrustumCulling`

World-space playback also disables built-in frustum culling because proxy-local bounds are not reliable there.

## Current Scope

Shipped now:

- fixed-topology soft-body VAT
- local-space or world-space metadata flags
- separate or packed normal textures
- multi-clip playback
- shared-material storage-buffer uploads

Deferred:

- rigid-body VAT rotation / pivot textures
- auxiliary shading channels in the shader path
- advanced GPU-side instancing extraction beyond the material storage-buffer path
