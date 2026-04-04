# Configuration

## Plugin

`VertexAnimationTexturePlugin`

Fields:

- `activate_schedule`
  - Type: `Interned<dyn ScheduleLabel>`
  - Effect: turns the runtime on
- `deactivate_schedule`
  - Type: `Interned<dyn ScheduleLabel>`
  - Effect: turns the runtime off
- `update_schedule`
  - Type: `Interned<dyn ScheduleLabel>`
  - Effect: hosts all `VatSystems`

Convenience:

- `VertexAnimationTexturePlugin::default()`
- `VertexAnimationTexturePlugin::always_on(Update)`

## Playback Components

### `VatAnimationSource`

- `animation`
  - `Handle<VatAnimationData>`
  - Required metadata asset
- `bounds_mode`
  - `VatBoundsMode`
  - Default: `UseMetadataAabb`

### `VatPlayback`

- `time_seconds`
  - Clip-local playback time
  - Default: `0.0`
- `speed`
  - Playback multiplier
  - Default: `1.0`
- `active_clip`
  - Clip index into `VatAnimationData::clips`
  - Default: `0`
- `loop_mode`
  - `VatLoopMode`
  - Default: `Loop`
  - Note: if left at `Loop`, clip metadata may provide a default override
- `playing`
  - Whether time advances
  - Default: `true`

### `VatCrossfade`

- `from_clip`
  - Source clip index
- `to_clip`
  - Destination clip index
- `elapsed`
  - Crossfade progress in seconds
- `duration`
  - Crossfade length in seconds
  - Clamped to at least `0.0001`

Usage:

- Insert `VatCrossfade` while leaving `VatPlayback.active_clip` on the currently playing source clip.
- The runtime captures the source clip/time internally and flips playback to `to_clip`.
- Do not reset `VatPlayback.time_seconds` manually when requesting the crossfade.

### `VatPlaybackTweaks`

- `disable_interpolation`
  - `false` by default
  - When `true`, the runtime snaps to frame A instead of blending to frame B

### `VatPlaybackFollower`

- `leader`
  - `Entity`
  - Required playback source entity to mirror
- `time_offset_seconds`
  - `0.0` by default
  - Applies a signed clip-local offset after copying the leader state
- `mirror_loop_mode`
  - `true` by default
  - When `true`, the follower copies the leader loop mode before time normalization
- `mirror_crossfade`
  - `true` by default
  - When `true`, `VatCrossfade` and its runtime source state are mirrored as well

Usage:

- Add `VatPlaybackFollower` to secondary meshes that should stay phase-locked to a leader mesh.
- Followers do not advance their own time independently while the component is present.
- Offsets are normalized through the resolved loop mode, so looping and ping-pong clips stay stable.

## Enums

### `VatLoopMode`

- `Loop`
  - Wraps at the end of the clip
- `Once`
  - Clamps at the clip boundary and pauses playback
- `PingPong`
  - Reflects at both clip boundaries
- `ClampForever`
  - Clamps at the boundary without toggling `playing`

### `VatBoundsMode`

- `UseMetadataAabb`
  - Inserts metadata-driven `Aabb`
- `KeepProxyAabb`
  - Leaves the proxy mesh bounds untouched
- `DisableFrustumCulling`
  - Adds `NoFrustumCulling`

## Material Builder

`build_vat_material(...)`

Inputs:

- base `StandardMaterial`
- `VatAnimationData`
- position texture handle
- optional normal texture handle
- `VatMaterialDefaults`
- mutable `Assets<ShaderStorageBuffer>`

Failure cases:

- metadata declares a separate normal texture but no normal handle is supplied

## Validation Helpers

- `validate_animation_data`
  - metadata-only validation
  - rejects undersized position-texture layouts, unsupported loop modes, unknown precision strings, and malformed normal descriptors
- `validate_mesh_for_animation`
  - proxy mesh validation against metadata
- `metadata_aabb`
  - converts metadata animation bounds into Bevy `Aabb`
- `should_disable_frustum_culling`
  - central policy helper for culling fallback
