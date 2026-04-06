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
   - resolves `VatPlayback.startup_clip` into a concrete clip index
   - applies `invalid_clip_fallback` if the resolved clip becomes invalid
   - advances clip-local time
   - applies loop policy
   - advances crossfade source state
   - records pending events / finish notifications
5. `VatSystems::SyncFollowers`
   - copies authoritative playback state from leader meshes into follower meshes
   - applies optional time offsets after loop-mode normalization
   - mirrors active crossfade requests for modular multi-mesh actors
6. `VatSystems::ResolveTransitions`
   - progresses and clears completed crossfades
7. `VatSystems::EmitMessages`
   - emits `VatClipFinished`
   - emits `VatEventReached`
8. `VatSystems::SyncGpuState`
   - validates mesh / metadata compatibility
   - applies metadata-driven bounds and frustum-culling policy
   - groups entities by material handle
   - writes one storage-buffer entry per entity
   - assigns `MeshTag` so the shader can index the right entry

## Modular Multi-Mesh Playback

`VatPlaybackFollower` provides a light ECS-level sync layer for modular actors made from several
meshes that all share the same VAT metadata layout.

- The leader owns the real `VatPlayback`
- Followers skip independent time advancement
- The follower sync pass copies the resolved clip selection, play/pause state, optional loop mode, and optional
  crossfade state
- Per-follower `time_offset_seconds` is applied after loop normalization so crowds and layered props
  can intentionally stagger motion without drifting out of phase

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

## Performance Model

The runtime cost scales as follows:

- **CPU per-entity**: one `VatPlaybackRuntime` iteration per entity with `VatPlayback`, plus one
  storage-buffer write per entity per frame. This is dominated by ECS iteration, not animation math.
- **CPU per-material**: one storage-buffer upload per unique `Handle<VatMaterial>`. Groups of entities
  sharing the same material are packed into a single buffer.
- **GPU per-entity**: two texture fetches per frame (frame A + frame B) for position, optionally two
  more for normals. During crossfade, these double (secondary clip). Each fetch is a nearest-neighbor
  point sample — no filtering overhead.
- **GPU per-draw-call**: all entities sharing the same material and mesh are drawn in one instanced
  call. The vertex shader indexes the storage buffer via `MeshTag`.

For crowds of 1,000–10,000 entities sharing one material, the bottleneck is typically vertex shader
throughput (texture bandwidth), not CPU-side ECS iteration.

## Message Flow

The crate uses Bevy 0.18 Messages (not Events) for `VatClipFinished` and `VatEventReached`.
Messages are written in `VatSystems::EmitMessages` and can be read by consumers in any later
system. Messages are transient — they only exist for one frame.

Clip selection is intentionally split between a high-level startup selector and a resolved runtime
index:

- `VatPlayback.startup_clip`
  - metadata default, clip name, or explicit index
- `VatPlayback.active_clip`
  - the validated runtime index currently driving playback
- `VatPlayback.invalid_clip_fallback`
  - policy used when an explicit or stale clip selection no longer resolves cleanly

Event detection works by recording "traversal segments" during time advancement. Each segment
represents a contiguous range of clip-local time that was traversed in a single frame. Events
fire when a segment crosses a threshold time. This correctly handles:

- Normal forward playback
- Reverse playback (negative speed)
- PingPong direction changes
- Multiple loop wraps in a single large delta
