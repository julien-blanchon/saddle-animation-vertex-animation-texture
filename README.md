# Saddle Animation Vertex Animation Texture

Reusable GPU vertex animation texture playback for Bevy PBR.

The crate targets fixed-topology soft-body VAT first: pre-baked mesh deformation stored in textures, ECS-owned playback state, and a PBR-compatible `ExtendedMaterial<StandardMaterial, ...>` render path. It is designed for large crowds, ambient motion, one-shot replays, and other cases where moving animation evaluation from CPU-side pose solving to GPU-side texture sampling is worth the memory trade.

## What Are Vertex Animation Textures?

A Vertex Animation Texture (VAT) encodes per-vertex animation data into 2D textures. Instead of evaluating skeletal rigs or physics simulations at runtime on the CPU, the entire animation is pre-baked into image data. At runtime, a vertex shader samples the texture to reconstruct each vertex's position (and optionally normal) at the current frame — entirely on the GPU.

**Why use VATs?**

- **Zero CPU animation cost** — no bone transforms, no skinning, no blend tree evaluation
- **Excellent GPU batching** — all instances sharing the same mesh and material render in one draw call
- **10,000+ independently animated characters** at interactive frame rates on modern GPUs
- Perfect for **crowds, vegetation, ambient props, VFX replays**, and baked simulations

For a complete guide on creating VAT assets, see [docs/vat-creation-guide.md](docs/vat-creation-guide.md).

## Quick Start

```rust
use bevy::prelude::*;
use saddle_animation_vertex_animation_texture::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(VertexAnimationTexturePlugin::default())
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // Load assets exported from Blender (OpenVAT) or Houdini (VAT 3.0)
    let mesh = asset_server.load("character.glb#Mesh0/Primitive0");
    let metadata = asset_server.load("character.vatanim.json");

    commands.spawn((
        Name::new("VAT Character"),
        Mesh3d(mesh),
        VatAnimationSource::new(metadata),
        VatPlayback::default()
            .with_clip_name("idle") // or omit this and use metadata default_clip
            .with_speed(1.0),     // normal speed
    ));
}
```

The material must be built once the metadata asset is loaded. See the `basic` example for the complete asset-loading pattern with `build_vat_material()`.

## Public API

Plugin:

- `VertexAnimationTexturePlugin`
  - Injectable schedules: `activate_schedule`, `deactivate_schedule`, `update_schedule`
  - `Default` / `always_on(Update)` convenience path

System sets:

- `VatSystems::AdvancePlayback`
- `VatSystems::SyncFollowers`
- `VatSystems::ResolveTransitions`
- `VatSystems::EmitMessages`
- `VatSystems::SyncGpuState`

Consumer-facing components:

- `VatAnimationSource` — links an entity to its VAT metadata asset
- `VatPlayback` — playback state: resolved clip index, startup clip selection, time, speed, loop mode, play/pause
- `VatCrossfade` — request a smooth blend between two clips (named helpers included)
- `VatPlaybackFollower` — mirror a leader entity's playback for modular multi-mesh actors
- `VatPlaybackTweaks` — per-entity options (e.g., disable frame interpolation)
- `VatAnimationBundle` — convenience bundle for `VatAnimationSource` + `VatPlayback`

Assets and materials:

- `VatAnimationData` — the runtime metadata asset (loaded from JSON)
- `VatAnimationDataLoader` — Bevy asset loader for `.vatanim.json` and `.vat.json` files
- `VatMaterial = ExtendedMaterial<StandardMaterial, VatMaterialExt>` — the PBR VAT material
- `VatMaterialDefaults` — resource holding fallback textures
- `build_vat_material(...)` — constructs a `VatMaterial` from metadata, textures, and a base `StandardMaterial`

Messages:

- `VatClipFinished` — emitted when a clip reaches its end (per loop/once/clamp policy)
- `VatEventReached` — emitted when playback crosses a named event frame

Validation helpers:

- `validate_animation_data` — metadata-only validation
- `validate_mesh_for_animation` — proxy mesh validation against metadata
- `metadata_aabb` — converts metadata animation bounds into Bevy `Aabb`
- `should_disable_frustum_culling` — central policy helper for culling fallback

## Playback Features

### Loop Modes

| Mode | Behavior |
|---|---|
| `Loop` | Wraps at end, plays forever |
| `Once` | Plays to end, pauses playback |
| `PingPong` | Bounces back and forth between start and end |
| `ClampForever` | Plays to end, holds last frame without pausing |

### Speed and Direction

```rust
VatPlayback::default()
    .with_speed(2.0)    // 2x speed
    .with_speed(-1.0)   // reverse playback
    .with_speed(0.5)    // half speed
```

### Frame Interpolation

The shader blends between adjacent frames by default. Disable for a "stepped" look:

```rust
VatPlaybackTweaks { disable_interpolation: true }
```

### Crossfade Transitions

Blend between clips in the vertex shader:

```rust
let crossfade = VatCrossfade::between_clip_names(&animation, "idle", "burst", 0.5)
    .expect("clip names should exist in metadata");
commands.entity(entity).insert(crossfade);
```

The system captures source state, switches to the target clip, blends in the shader, and cleans up when the crossfade completes.

### Animation Events

Define events at specific frames in the metadata JSON:

```json
"events": [{ "name": "footstep", "frame": 6 }]
```

React to them:

```rust
fn on_event(mut events: MessageReader<VatEventReached>) {
    for event in events.read() {
        if event.event_name == "footstep" {
            // play sound, spawn particles, etc.
        }
    }
}
```

### Modular Multi-Mesh Sync

Keep multiple meshes phase-aligned (body + armor + weapon):

```rust
let leader = commands.spawn((/* body mesh, VatPlayback, etc. */)).id();
commands.spawn((
    /* armor mesh */
    VatPlaybackFollower::new(leader)
        .with_time_offset_seconds(0.05),
));
```

Followers mirror clip selection, play/pause, loop mode, and crossfade transitions.

## Metadata Formats

The crate loads two JSON formats:

1. **Canonical** (`.vatanim.json`) — explicit, full control. See [docs/asset-format.md](docs/asset-format.md).
2. **OpenVAT** (`.vat.json`) — compatible with OpenVAT's `remap_info.json` + layout extensions.

See [docs/vat-creation-guide.md](docs/vat-creation-guide.md) for complete examples of both formats.

## Dependencies

- `bevy = "0.18"`
- `serde`
- `serde_json`
- `thiserror`

## Bounds and Frustum Culling

| Mode | When to use |
|---|---|
| `UseMetadataAabb` (default) | Most cases — uses `animation_bounds` from metadata |
| `KeepProxyAabb` | When proxy bounds are correct |
| `DisableFrustumCulling` | World-space playback, or when bounds are unreliable |

## Scope

Implemented in v0.1:

- Canonical JSON metadata loading
- OpenVAT-compatible metadata normalization subset
- Multi-clip playback with configurable loop modes
- Crossfade support between clips
- Leader/follower playback sync for modular characters
- Optional separate or packed normal textures
- Shared-material storage-buffer uploads for many independently timed entities
- Animation event system (`VatClipFinished`, `VatEventReached`)

Deferred / documented extension paths:

- Rigid-body VAT rotation textures
- Auxiliary data textures beyond the metadata model
- Bone animation textures
- Streaming / clip windowing

## Examples

```bash
cd examples
cargo run -p saddle-animation-vertex-animation-texture-example-basic
cargo run -p saddle-animation-vertex-animation-texture-example-crowd
cargo run -p saddle-animation-vertex-animation-texture-example-multi-clip
cargo run -p saddle-animation-vertex-animation-texture-example-modular-sync
cargo run -p saddle-animation-vertex-animation-texture-example-debug-lab
```

| Example | Description |
|---|---|
| **basic** | Single VAT mesh using the metadata default clip, with pane controls for speed and interpolation |
| **crowd** | 54 independently animated instances sharing one material, assigned to named metadata clips |
| **multi_clip** | Auto-cycles through named clips with shader-side crossfade helpers |
| **modular_sync** | Leader/follower sync using named startup clips and configurable time offsets |
| **debug_lab** | Interactive controls: `1` idle, `2` gust, `3` burst, `Space` pause, `I` toggle interpolation |

Crate-local lab:

```bash
cargo run -p saddle-animation-vertex-animation-texture-lab
cargo run -p saddle-animation-vertex-animation-texture-lab --features e2e -- vat_smoke
```

## Documentation

- [VAT Creation Guide](docs/vat-creation-guide.md) — complete guide to creating VATs from scratch (Blender, Houdini, programmatic)
- [Troubleshooting](docs/troubleshooting.md) — common issues and fixes
- [Asset Format](docs/asset-format.md) — JSON metadata schema reference
- [Configuration](docs/configuration.md) — all components, parameters, and tuning options
- [Architecture](docs/architecture.md) — internal design, data flow, system ordering
