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

## Messages

### `VatClipFinished`

Emitted each time a clip reaches its boundary (loop wrap, once completion, ping-pong bounce).

Fields:

- `entity` — the entity that finished
- `clip_index` — which clip finished
- `clip_name` — the clip's metadata name
- `finished_at_seconds` — the clip-local time at the boundary

Usage:

```rust
fn on_clip_finished(mut messages: MessageReader<VatClipFinished>) {
    for msg in messages.read() {
        info!("{} finished clip '{}'", msg.entity, msg.clip_name);
    }
}
```

### `VatEventReached`

Emitted when playback crosses a named event frame defined in the metadata.

Fields:

- `entity` — the entity that crossed the event
- `clip_index` — which clip the event belongs to
- `clip_name` — the clip's metadata name
- `event_name` — the event's metadata name
- `clip_frame` — the frame index within the clip
- `normalized_time` — the event's position as a 0–1 fraction of the clip
- `reached_at_seconds` — the clip-local time of the event

Events fire exactly once per threshold crossing, even during reverse playback or ping-pong.

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

## Utility Functions

- `configure_vat_data_image(image)` — sets nearest/point sampling on a VAT data texture
- `make_linear_rgba8_image(size, data)` — creates a linear Rgba8Unorm image with nearest sampling
- `decode_position_sample(encoded, animation, proxy_position)` — decodes a single position sample for debugging
- `convert_coordinate_system(value, coordinate_system)` — converts from source coordinate system to Bevy's Y-up
- `valid_bounds(min, max)` — checks that bounds are finite and min <= max
- `parse_vat_animation_data_str(json)` — parses a JSON string into `VatAnimationData`
- `parse_vat_animation_data_bytes(bytes)` — parses JSON bytes into `VatAnimationData`
