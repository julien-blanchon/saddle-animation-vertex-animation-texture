# Saddle Animation Vertex Animation Texture

Reusable GPU vertex animation texture playback for Bevy PBR.

The crate targets fixed-topology soft-body VAT first: pre-baked mesh deformation stored in textures, ECS-owned playback state, and a PBR-compatible `ExtendedMaterial<StandardMaterial, ...>` render path. It is designed for large crowds, ambient motion, one-shot replays, and other cases where moving animation evaluation from CPU-side pose solving to GPU-side texture sampling is worth the memory trade.

## Public API

Plugin:

- `VertexAnimationTexturePlugin`
  - Injectable schedules: `activate_schedule`, `deactivate_schedule`, `update_schedule`
  - `Default` / `always_on(Update)` convenience path

System sets:

- `VatSystems::AdvancePlayback`
- `VatSystems::ResolveTransitions`
- `VatSystems::EmitMessages`
- `VatSystems::SyncGpuState`

Consumer-facing components:

- `VatAnimationSource`
- `VatPlayback`
- `VatCrossfade`
- `VatPlaybackTweaks`
- `VatAnimationBundle`

Assets and materials:

- `VatAnimationData`
- `VatAnimationDataLoader`
- `VatMaterial = ExtendedMaterial<StandardMaterial, VatMaterialExt>`
- `VatMaterialDefaults`
- `build_vat_material(...)`

Messages:

- `VatClipFinished`
- `VatEventReached`

Validation helpers:

- `validate_animation_data`
- `validate_mesh_for_animation`
- `metadata_aabb`
- `should_disable_frustum_culling`

## Dependencies

- `bevy = "0.18"`
- `serde`
- `serde_json`
- `thiserror`

## Communication

Reads:

- `VatAnimationSource`
- `VatPlayback`
- `VatCrossfade`
- `VatPlaybackTweaks`
- `Mesh3d`
- `MeshMaterial3d<VatMaterial>`

Writes:

- `VatClipFinished`
- `VatEventReached`
- `MeshTag`
- `Aabb`
- `NoFrustumCulling`

## Scope

Implemented in v0.1:

- Canonical JSON metadata loading
- OpenVAT-compatible metadata normalization subset
- Multi-clip playback
- Loop / once / ping-pong / clamp playback policies
- Crossfade support
- Optional separate or packed normal textures
- Shared-material storage-buffer uploads for many independently timed entities
- Crate-local examples and lab
- Strict canonical metadata parsing for loop modes, texture precision, normal layout, and auxiliary texture semantics

Deferred / documented extension paths:

- Rigid-body VAT rotation textures
- Auxiliary data textures beyond the metadata model
- Bone animation textures
- Streaming / clip windowing

## Examples

```bash
cargo run --example basic
cargo run --example crowd
cargo run --example multi_clip
cargo run --example debug_lab
```

Crate-local lab:

```bash
cargo run -p saddle-animation-vertex-animation-texture-lab
cargo run -p saddle-animation-vertex-animation-texture-lab --features e2e -- vat_smoke
```

## Crossfade Requests

To request a transition, insert `VatCrossfade::new(from_clip, to_clip, duration)` and leave
`VatPlayback.active_clip` on the currently playing source clip. The runtime captures source state,
then switches to the target clip on the next update without needing the caller to rewrite material
state or playback time manually.
