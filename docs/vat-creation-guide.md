# VAT Creation Guide

A complete guide to creating, exporting, and using Vertex Animation Textures with this crate.

## What Are Vertex Animation Textures?

A Vertex Animation Texture (VAT) encodes per-vertex animation data into 2D textures. Instead
of evaluating skeletal rigs or physics simulations at runtime on the CPU, the entire animation
is pre-baked into image data. At runtime, a vertex shader samples the texture to reconstruct
each vertex's position (and optionally normal) at the current frame — entirely on the GPU.

**Texture layout**: the X axis maps to individual vertices, and the Y axis maps to animation
frames (time). Each pixel stores the XYZ position of one vertex at one frame, encoded into
the RGB channels. A second UV channel on the mesh (UV1 in engine terms) encodes each vertex's
lookup coordinate into the texture.

```
Position Texture                 Normal Texture (optional)
┌──────────────────────┐         ┌──────────────────────┐
│ v0  v1  v2  v3 ... vN│ frame 0 │ v0  v1  v2  v3 ... vN│ frame 0
│ v0  v1  v2  v3 ... vN│ frame 1 │ v0  v1  v2  v3 ... vN│ frame 1
│ v0  v1  v2  v3 ... vN│ frame 2 │ v0  v1  v2  v3 ... vN│ frame 2
│         ...          │   ...   │         ...          │   ...
│ v0  v1  v2  v3 ... vN│ frame F │ v0  v1  v2  v3 ... vN│ frame F
└──────────────────────┘         └──────────────────────┘
```

## When to Use VATs (vs Alternatives)

| Technique | CPU cost/instance | GPU cost/instance | Batching | Animation flexibility | Memory |
|---|---|---|---|---|---|
| **Skeletal animation** | High (bone transforms, skinning) | Moderate (GPU skinning) | Poor (each instance is separate) | Full blending, IK, ragdoll | Low |
| **VAT** | Near-zero | Low (single texture fetch) | Excellent (GPU instancing) | Pre-baked only, limited blending | Higher |
| **Morph targets** | Moderate | Moderate | Moderate | Runtime blending | Moderate |
| **Procedural** | Varies | Varies | Varies | Full runtime control | Lowest |

**Use VATs when:**

- You need **hundreds or thousands** of independently animated instances (crowds, vegetation,
  ambient props, particle-like effects)
- The animation is **fully pre-determined** — no runtime IK, ragdoll, or procedural adjustment
  needed
- You want to offload animation entirely to the GPU for **maximum draw-call batching**
- You're baking **simulation results** (cloth, fluid, destruction) that are too expensive to
  run at runtime

**Avoid VATs when:**

- You need **runtime animation blending** beyond simple crossfades between pre-baked clips
- You need **inverse kinematics**, ragdoll physics, or other runtime pose adjustments
- The animation has very few instances (skeletal is simpler for 1–10 characters)
- Texture memory is extremely constrained (mobile with many unique animations)

## VAT Types

### Soft Body (Fixed Topology) — supported

The mesh deforms but keeps the same vertex count and order every frame. This is the most
common type and is what this crate fully implements.

**Use cases**: cloth, flags, waving vegetation, character animations, organic deformation,
any simulation where topology doesn't change.

**Output**: position texture + optional normal texture + proxy mesh.

### Rigid Body — deferred (documented extension path)

Fracture and destruction simulations where each piece is a rigid body. Instead of per-vertex
positions, the texture stores per-piece transforms (translation + quaternion rotation). Each
mesh piece has a pivot point encoded in vertex colors or UV3.

**Use cases**: glass breaking, concrete crumbling, explosions, RBD simulations.

### Fluid / Dynamic Remesh — deferred

Simulations where topology changes every frame (different vertex counts and connectivity).
Requires a Lookup Table Texture (LUT) to map triangles to animation coordinates.

**Use cases**: fluid simulations, smoke, morphing effects.

### Sprite / Particle — deferred

Each particle is a small card (quad). The position texture stores per-particle positions
over time. Cards are billboarded in the shader.

**Use cases**: sparks, debris, rain, magic effects, particle simulations.

## Step-by-Step: Creating a VAT in Blender (OpenVAT)

### Prerequisites

- Blender 4.2 LTS or newer
- [OpenVAT](https://extensions.blender.org/add-ons/openvat/) addon installed

### Step 1: Prepare Your Animation

Animate your mesh using any method: armature, shape keys, Geometry Nodes, physics
simulations, or modifiers. The **only requirement** is that the vertex count and vertex order
remain constant across all frames (fixed topology).

> **Important**: if your mesh has hard edges or UV seams, Blender may split vertices during
> export. This changes the effective vertex count. OpenVAT handles this automatically, but
> be aware that the exported vertex count may differ from what you see in edit mode.

### Step 2: Select the Target Mesh

Select the mesh object (or collection of meshes) you want to bake.

### Step 3: Open the OpenVAT Panel

In the 3D Viewport sidebar (press `N`), find the OpenVAT panel.

### Step 4: Configure Encoding Settings

| Setting | Recommendation | Notes |
|---|---|---|
| **Frame range** | Start/end frames covering your animation | Only frames in this range are baked |
| **Proxy definition** | Start Frame or Current Frame | The "rest pose" mesh that gets exported |
| **Normal encoding** | Separate | Best quality; use Packed to save memory, None for positions only |
| **Texture format** | EXR half-float (production) or PNG 16-bit (compatibility) | Avoid 8-bit unless prototyping |
| **Mesh format** | glTF (.glb) for Bevy | FBX for Unity/Unreal |
| **Space** | Object space (default) | Use World space only if your animation is world-relative |

### Step 5: Define Animation Clips (OpenVAT 1.1.0+)

In the Animation Data panel, define named clips with their frame ranges and loop flags:

- `idle`: frames 0–23, looping
- `walk`: frames 24–47, looping
- `attack`: frames 48–71, play once

These are metadata only — they don't change which frames are baked, but they're written to
the output JSON for runtime clip selection.

### Step 6: Set Output Directory and Encode

Click **Encode**. OpenVAT will:

1. Capture every frame's vertex positions
2. Compute the bounding box across all frames (the "remap bounds")
3. Normalize positions to [0, 1] range using the bounds
4. Write the position texture
5. Optionally write the normal texture
6. Export the proxy mesh with UV1 baked
7. Write `remap_info.json` with decode metadata

### Step 7: Verify Output

Your output directory should contain:

```
output/
├── MeshName_vpos.exr       # Position texture
├── MeshName_vnrm.exr       # Normal texture (if configured)
├── MeshName.glb             # Proxy mesh with UV1
└── remap_info.json          # Decode metadata
```

### Step 8: Preview in Blender

After encoding, OpenVAT creates a copy of the mesh with a decoder modifier. Play the
timeline to verify the animation reconstructs correctly in Blender's viewport.

## Step-by-Step: Creating a VAT in Houdini

[SideFX Labs VAT 3.0](https://www.sidefx.com/docs/houdini/nodes/out/labs--vertex_animation_textures-3.0.html)
is the industry-standard tool for VAT creation.

### Workflow

1. Feed your simulation geometry (Vellum cloth, RBD, FLIP fluid, particles) into the
   `Labs Vertex Animation Textures 3.0` ROP node.
2. Choose the animation type: Soft, Rigid, Fluid, or Sprite.
3. Configure texture format (HDR EXR or non-HDR PNG), dimensions, and precision.
4. The tool outputs: FBX mesh, position texture, optional rotation/normal/color textures,
   and bounding box data embedded in the mesh.

Houdini VAT 3.0 also supports LODs, up to 9 custom data channels, conditional triggering,
and power-of-two texture padding.

## Importing into Bevy with This Crate

### Asset Setup

After exporting from your DCC tool, you need:

1. **Proxy mesh** (`.glb` or `.gltf`) — the reference mesh with UV1 baked
2. **Position texture** (`.exr` or `.png`) — the baked position data
3. **Normal texture** (`.exr` or `.png`, optional) — the baked normal data
4. **Metadata JSON** (`.vatanim.json` or `.vat.json`) — decode parameters

Place these in your Bevy `assets/` directory.

### Writing the Metadata JSON

#### Canonical Format (recommended)

The canonical format is the most explicit and gives you full control:

```json
{
  "format": "vertex_animation_texture@1",
  "animation_mode": "soft_body_fixed_topology",
  "vertex_count": 500,
  "frame_count": 72,
  "frames_per_second": 24.0,
  "decode_bounds": {
    "min": [-2.0, -0.5, -2.0],
    "max": [2.0, 3.0, 2.0]
  },
  "animation_bounds": {
    "min": [-2.5, -0.5, -2.5],
    "max": [2.5, 3.5, 2.5]
  },
  "clips": [
    {
      "name": "idle",
      "start_frame": 0,
      "end_frame": 23,
      "default_loop_mode": "loop",
      "events": [
        { "name": "footstep", "frame": 6 },
        { "name": "footstep", "frame": 18 }
      ]
    },
    {
      "name": "walk",
      "start_frame": 24,
      "end_frame": 47,
      "default_loop_mode": "loop"
    },
    {
      "name": "attack",
      "start_frame": 48,
      "end_frame": 71,
      "default_loop_mode": "once"
    }
  ],
  "position_texture": {
    "relative_path": "character_vpos.exr",
    "width": 500,
    "height": 72,
    "rows_per_frame": 1,
    "precision": "exr_half"
  },
  "normal_texture": {
    "mode": "separate",
    "texture": {
      "relative_path": "character_vnrm.exr",
      "width": 500,
      "height": 72,
      "rows_per_frame": 1,
      "precision": "exr_half"
    },
    "encoding": "signed_normalized"
  },
  "coordinate_system": "z_up_right_handed",
  "playback_space": "local",
  "vertex_id_attribute": "uv1",
  "position_encoding": "absolute_normalized_bounds"
}
```

#### OpenVAT Format (if using OpenVAT export)

OpenVAT's `remap_info.json` is also supported. You may need to add layout fields that
OpenVAT doesn't include by default:

```json
{
  "os-remap": {
    "Min": [-2.0, -0.5, -2.0],
    "Max": [2.0, 3.0, 2.0],
    "Frames": 72
  },
  "vertex_count": 500,
  "texture_width": 500,
  "texture_height": 72,
  "rows_per_frame": 1,
  "packed_normals": false,
  "animations": {
    "idle": { "start_frame": 0, "end_frame": 23, "looping": true },
    "walk": { "start_frame": 24, "end_frame": 47, "looping": true },
    "attack": { "start_frame": 48, "end_frame": 71, "looping": false }
  }
}
```

### Loading and Spawning in Bevy

```rust
use bevy::prelude::*;
use saddle_animation_vertex_animation_texture::*;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<VatMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    animations: Res<Assets<VatAnimationData>>,
) {
    // 1. Load assets
    let mesh: Handle<Mesh> = asset_server.load("my_character.glb#Mesh0/Primitive0");
    let metadata: Handle<VatAnimationData> = asset_server.load("my_character.vatanim.json");
    let position_tex: Handle<Image> = asset_server.load("my_character_vpos.exr");
    let normal_tex: Handle<Image> = asset_server.load("my_character_vnrm.exr");

    // 2. Configure textures for VAT sampling (nearest filter, no mipmaps)
    // Do this in an asset-ready system, or use asset preprocessing.

    // 3. Build material (once metadata is loaded)
    // Typically done in a system that waits for assets to load:
    // let material = build_vat_material(
    //     StandardMaterial { ... },
    //     &animation_data,
    //     position_tex,
    //     Some(normal_tex),
    //     &defaults,
    //     &mut buffers,
    // ).unwrap();

    // 4. Spawn entity
    commands.spawn((
        Name::new("VAT Character"),
        Mesh3d(mesh),
        // MeshMaterial3d(materials.add(material)),
        VatAnimationSource::new(metadata),
        VatPlayback::default()
            .with_clip(0)
            .with_speed(1.0),
    ));
}
```

### Configuring Textures

VAT textures **must** use nearest/point filtering and should not have mipmaps:

```rust
use saddle_animation_vertex_animation_texture::configure_vat_data_image;

fn configure_loaded_textures(
    mut images: ResMut<Assets<Image>>,
    mut events: EventReader<AssetEvent<Image>>,
) {
    for event in events.read() {
        if let AssetEvent::Added { id } = event {
            if let Some(image) = images.get_mut(*id) {
                configure_vat_data_image(image);
            }
        }
    }
}
```

## Texture Encoding Deep Dive

### Position Encoding

At bake time, positions are normalized into [0, 1]:

```
encoded.r = (position.x - bounds_min.x) / (bounds_max.x - bounds_min.x)
encoded.g = (position.y - bounds_min.y) / (bounds_max.y - bounds_min.y)
encoded.b = (position.z - bounds_min.z) / (bounds_max.z - bounds_min.z)
```

At runtime, the shader reverses this:

```
decoded = decode_min + encoded * (decode_max - decode_min)
```

The bounding box min/max values are the `decode_bounds` in the metadata JSON.

### Two Position Encoding Modes

- **`absolute_normalized_bounds`**: the decoded value is the absolute vertex position in
  source space. The proxy mesh position is replaced entirely.
- **`offset_normalized_bounds`**: the decoded value is an offset (delta) from the proxy mesh
  vertex. The proxy mesh position is added to the decoded offset.

Most Houdini exports use absolute encoding. OpenVAT uses offset encoding by default.

### Normal Encoding

Unit normals (range [-1, 1]) are remapped to [0, 1]:

```
encoded_normal = (normal + 1.0) / 2.0
```

Decoded in shader:

```
decoded_normal = normalize(encoded * 2.0 - 1.0)
```

### Normal Storage Options

| Mode | Description | Memory | Quality |
|---|---|---|---|
| `none` | No animated normals; uses proxy mesh normals | Lowest | Poor for large deformations |
| `packed` | Normals packed into additional rows of the position texture | Medium | Good |
| `separate` | Dedicated normal texture | Highest | Best |

### Precision Profiles

| Format | Bits/Channel | Quantization Steps | Artifact Level | Use Case |
|---|---|---|---|---|
| **EXR half** | 16 float | ~65,504 | Negligible | Production |
| **PNG 16-bit** | 16 integer | 65,536 | Minimal | Good compromise |
| **PNG 8-bit** | 8 integer | 256 | Visible "vertex swimming" | Prototyping only |

**The precision trade-off**: quantization error scales with the bounding box size. A large
bounding box means each quantization step covers more world-space distance. Strategies:

- Use offset encoding (smaller per-vertex deltas = smaller bounding box)
- Use higher-precision textures (EXR half or PNG 16)
- Keep animations spatially compact

## Coordinate Systems

| Tool | Up axis | Handedness |
|---|---|---|
| Blender | Z-up | Right-handed |
| Houdini | Y-up | Right-handed |
| Bevy | Y-up | Right-handed |
| Unity | Y-up | Left-handed |
| Unreal | Z-up | Left-handed |

This crate handles coordinate conversion automatically via the `coordinate_system` metadata
field. Set it to `z_up_right_handed` for Blender exports, and the shader will convert
`(x, y, z)` to Bevy's Y-up space: `(x, z, -y)`.

## UV1 — The Vertex Lookup Channel

The mesh must have a second UV channel (UV1 in Bevy, "UV2" in most DCC tools). This channel
maps each vertex to its corresponding texel in the VAT texture.

**Rules:**

- Each vertex must map to a **unique texel**. Shared UV1 values cause vertices to read the
  same animation data.
- UV1 values encode texel-center coordinates: `uv = (texel_index + 0.5) / texture_width`
- The DCC export tool (OpenVAT, Houdini VAT) generates UV1 automatically during the bake.
- Do not manually edit UV1 after baking — even small rounding errors cause vertex swimming.

**Verifying UV1**: the crate's `validate_mesh_for_animation()` function checks that UV1
exists, has the correct format, and maps to the expected number of unique texels.

## Multi-Clip Workflow

A single VAT bake can contain multiple animation clips, each using a sub-range of the
texture's frame rows:

```
Frame  0–23:  idle
Frame 24–47:  walk
Frame 48–71:  attack
```

Define clips in the metadata JSON:

```json
"clips": [
  { "name": "idle",   "start_frame": 0,  "end_frame": 23, "default_loop_mode": "loop" },
  { "name": "walk",   "start_frame": 24, "end_frame": 47, "default_loop_mode": "loop" },
  { "name": "attack", "start_frame": 48, "end_frame": 71, "default_loop_mode": "once" }
]
```

Switch clips at runtime:

```rust
// Instant switch
playback.active_clip = 1; // switch to "walk"
playback.time_seconds = 0.0;

// Smooth crossfade
commands.entity(entity).insert(
    VatCrossfade::new(0, 1, 0.3) // fade from clip 0 to clip 1 over 0.3s
);
```

## Playback Configuration

### Loop Modes

| Mode | Behavior |
|---|---|
| `Loop` | Wraps at end, plays forever |
| `Once` | Plays to end, pauses playback |
| `PingPong` | Bounces back and forth between start and end |
| `ClampForever` | Plays to end, holds last frame (no pause) |

### Speed Control

```rust
VatPlayback::default()
    .with_speed(2.0)    // double speed
    .with_speed(0.5)    // half speed
    .with_speed(-1.0)   // reverse playback
```

### Frame Interpolation

By default, the shader blends between adjacent frames for smooth playback. Disable this for
a "stepped" or "sprite-sheet" look:

```rust
commands.entity(entity).insert(VatPlaybackTweaks {
    disable_interpolation: true,
});
```

### Crossfade Transitions

Crossfade blends two clips simultaneously during a transition:

```rust
// Currently playing clip 0, crossfade to clip 2 over 0.5 seconds
commands.entity(entity).insert(
    VatCrossfade::new(0, 2, 0.5)
);
// The system handles everything: captures source state, switches active clip,
// blends in the shader, cleans up when done.
```

### Animation Events

Define events at specific frames in the metadata:

```json
"events": [
  { "name": "footstep", "frame": 6 },
  { "name": "impact",   "frame": 12 }
]
```

React to them with Bevy observers:

```rust
app.add_observer(|trigger: Trigger<VatEventReached>| {
    let event = trigger.event();
    if event.event_name == "footstep" {
        // Play footstep sound
    }
});
```

## Modular Multi-Mesh Actors

For characters assembled from multiple meshes (body + armor + weapon), use the
leader/follower system to keep all parts synchronized:

```rust
// Leader: the authoritative playback source
let leader = commands.spawn((
    Name::new("Body"),
    Mesh3d(body_mesh),
    MeshMaterial3d(body_material),
    VatAnimationSource::new(metadata.clone()),
    VatPlayback::default(),
)).id();

// Follower: mirrors the leader's playback state
commands.spawn((
    Name::new("Armor"),
    Mesh3d(armor_mesh),
    MeshMaterial3d(armor_material),
    VatAnimationSource::new(metadata.clone()),
    VatPlayback::default(),
    VatPlaybackFollower::new(leader)
        .with_time_offset_seconds(0.05), // optional stagger
));
```

Followers automatically mirror clip selection, play/pause state, loop mode, and crossfade
transitions from the leader.

## Bounds and Frustum Culling

Static proxy mesh bounds are almost always wrong for VAT because vertices move far beyond
the rest pose. Three strategies:

| Mode | When to use |
|---|---|
| `UseMetadataAabb` (default) | Most cases — uses `animation_bounds` from metadata |
| `KeepProxyAabb` | When you know the proxy bounds are correct |
| `DisableFrustumCulling` | World-space playback, or when bounds are unreliable |

Set the bounds mode per-entity:

```rust
VatAnimationSource::new(metadata)
    .with_bounds_mode(VatBoundsMode::DisableFrustumCulling)
```

> **Tip**: define `animation_bounds` in your metadata JSON as a slightly-padded bounding box
> that encompasses all vertex positions across all frames. This is separate from
> `decode_bounds` (which defines the texture encoding range).

## Performance Characteristics

### Why VATs Excel at Crowds

- **Zero CPU animation cost**: no bone transforms, no skinning, no blend tree evaluation
- **Excellent GPU batching**: all instances sharing the same mesh and material can be drawn
  together. This crate uses a single storage buffer per material for all instances.
- **Minimal per-instance data**: only frame indices and blend weights (12 floats per entity
  in the storage buffer)

### Benchmark Reference Points

- 10,000 independently animated characters at 30+ FPS is achievable with modern GPUs
- A single draw call can render all instances sharing the same VAT material
- CPU-side cost scales with entity count (ECS iteration), not vertex count

### Memory Budget

Position texture memory: `width * height * bytes_per_pixel`

| Format | 500 vertices, 72 frames | 2000 vertices, 120 frames |
|---|---|---|
| EXR half (8 B/pixel) | ~144 KB | ~960 KB |
| PNG 16 (6 B/pixel) | ~108 KB | ~720 KB |
| PNG 8 (3 B/pixel) | ~54 KB | ~360 KB |

Normal textures double these numbers. For crowds, all instances share the same textures.

## Full Worked Example: Animated Flag

### 1. In Blender

1. Create a subdivided plane (10x10 segments = 121 vertices).
2. Add a Cloth simulation with some wind.
3. Bake the simulation (frames 1–48).
4. Install and open OpenVAT.
5. Set frame range 1–48, normal encoding: Separate, format: PNG 16, mesh: glTF.
6. Define one clip: "wave", frames 0–47, looping.
7. Click Encode.

### 2. File Organization

```
assets/
├── flag/
│   ├── flag.glb              # Proxy mesh
│   ├── flag_vpos.png         # Position texture (16-bit)
│   ├── flag_vnrm.png         # Normal texture (16-bit)
│   └── flag.vatanim.json     # Metadata
```

### 3. Metadata JSON

```json
{
  "format": "vertex_animation_texture@1",
  "animation_mode": "soft_body_fixed_topology",
  "vertex_count": 121,
  "frame_count": 48,
  "frames_per_second": 24.0,
  "decode_bounds": {
    "min": [-1.0, -0.2, -1.0],
    "max": [1.0, 0.5, 1.0]
  },
  "animation_bounds": {
    "min": [-1.2, -0.3, -1.2],
    "max": [1.2, 0.6, 1.2]
  },
  "clips": [
    { "name": "wave", "start_frame": 0, "end_frame": 47, "default_loop_mode": "loop" }
  ],
  "position_texture": {
    "width": 121, "height": 48, "rows_per_frame": 1, "precision": "png16"
  },
  "normal_texture": {
    "mode": "separate",
    "texture": { "width": 121, "height": 48, "rows_per_frame": 1, "precision": "png16" },
    "encoding": "signed_normalized"
  },
  "coordinate_system": "z_up_right_handed",
  "playback_space": "local",
  "vertex_id_attribute": "uv1",
  "position_encoding": "absolute_normalized_bounds"
}
```

### 4. Bevy Code

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
    let mesh = asset_server.load("flag/flag.glb#Mesh0/Primitive0");
    let metadata = asset_server.load("flag/flag.vatanim.json");
    let position_tex = asset_server.load("flag/flag_vpos.png");
    let normal_tex = asset_server.load("flag/flag_vnrm.png");

    // Material setup happens once assets are loaded (see examples for full pattern)

    commands.spawn((
        Name::new("Animated Flag"),
        Mesh3d(mesh),
        VatAnimationSource::new(metadata),
        VatPlayback::default(), // clip 0, looping, speed 1.0
    ));

    // Camera
    commands.spawn((
        Name::new("Camera"),
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
```

## Programmatic VAT Generation (No DCC Tool Required)

You can create VAT data entirely in Rust at startup or build time — useful for procedural
animations, unit testing, or prototyping without any Blender/Houdini dependency.

### Overview

To create a VAT programmatically you need three things:

1. A proxy mesh with UV1 baked (mapping each vertex to its texel in the VAT texture)
2. A position texture (and optionally a normal texture) containing per-vertex, per-frame data
3. A `VatAnimationData` struct describing the bake layout

### Step 1: Generate the Proxy Mesh

Build a `Mesh` with `ATTRIBUTE_POSITION`, `ATTRIBUTE_NORMAL`, `ATTRIBUTE_UV_0`, and
`ATTRIBUTE_UV_1`. UV1 maps each vertex to a unique texel:

```rust
// For vertex i out of N total, at frame 0:
let u = (i as f32 + 0.5) / texture_width as f32;
let v = 0.5 / texture_height as f32; // first row, texel center
uv1.push([u, v]);
```

### Step 2: Generate the Position Texture

For each frame, compute each vertex's position and encode it into [0, 1]:

```rust
let bounds_min = /* minimum across all vertices and all frames */;
let bounds_max = /* maximum across all vertices and all frames */;
let extent = bounds_max - bounds_min;

let mut data = Vec::new();
for frame in 0..frame_count {
    for vertex in 0..vertex_count {
        let pos = compute_position(vertex, frame);
        let encoded = (pos - bounds_min) / extent;
        data.extend_from_slice(&[
            (encoded.x * 255.0).round() as u8,
            (encoded.y * 255.0).round() as u8,
            (encoded.z * 255.0).round() as u8,
            255,
        ]);
    }
}

let image = make_linear_rgba8_image(
    UVec2::new(vertex_count as u32, frame_count as u32),
    data,
);
```

### Step 3: Build VatAnimationData

```rust
let animation = VatAnimationData {
    source_format: VatSourceFormat::Canonical,
    animation_mode: VatAnimationMode::SoftBodyFixedTopology,
    vertex_count: 100,
    frame_count: 48,
    frames_per_second: 24.0,
    decode_bounds_min: bounds_min,
    decode_bounds_max: bounds_max,
    animation_bounds_min: bounds_min - Vec3::splat(0.1), // pad for culling
    animation_bounds_max: bounds_max + Vec3::splat(0.1),
    clips: vec![VatClip {
        name: "default".into(),
        start_frame: 0,
        end_frame: 47,
        default_loop_mode: Some(VatLoopMode::Loop),
        events: vec![],
    }],
    position_texture: VatTextureDescriptor {
        relative_path: None,
        width: 100,
        height: 48,
        rows_per_frame: 1,
        precision: VatTexturePrecision::Png8,
    },
    normal_texture: VatNormalTexture::None,
    rotation_texture: None,
    auxiliary_textures: vec![],
    coordinate_system: VatCoordinateSystem::YUpRightHanded,
    playback_space: VatPlaybackSpace::Local,
    vertex_id_attribute: VatVertexIdAttribute::Uv1,
    position_encoding: VatPositionEncoding::AbsoluteNormalizedBounds,
};
animation.validate().expect("metadata should be valid");
```

### Step 4: Assemble and Spawn

```rust
let animation_handle = animations.add(animation);
let position_handle = images.add(position_image);
let material = build_vat_material(
    StandardMaterial::default(),
    &animation_data,
    position_handle,
    None,
    &defaults,
    &mut buffers,
).unwrap();

commands.spawn((
    Name::new("Procedural VAT"),
    Mesh3d(meshes.add(proxy_mesh)),
    MeshMaterial3d(materials.add(material)),
    VatAnimationSource::new(animation_handle),
    VatPlayback::default(),
));
```

The crate's own examples use this exact approach — see `examples/support/src/lib.rs` for a
complete working implementation that generates a waving mesh entirely in code.

## Advanced Optimization Techniques

### Reducing Texture Memory

1. **Offset encoding**: use `offset_normalized_bounds` to store per-vertex deltas from the
   proxy pose. Deltas are typically much smaller than absolute positions, so the decode bounds
   shrink and quantization error decreases — even 8-bit PNG can work well for small-deformation
   animations like cloth or facial blendshapes.

2. **Reduce frame count**: export every other frame and let the shader interpolate. The crate
   interpolates between adjacent frames by default, so halving the frame count halves texture
   height with barely visible quality loss.

3. **Reduce vertex count**: use a lower-resolution simulation mesh for the VAT bake. The
   rendering mesh can have more visual detail in UV0 (normal maps, textures) while the VAT
   drives a simplified vertex set.

4. **Pack normals**: use packed normals in the position texture instead of a separate normal
   texture. This halves the number of textures but doubles `rows_per_frame`.

### LOD Strategies

Since all VAT instances share the same texture data, traditional mesh LOD (swapping meshes)
requires separate VAT bakes per LOD level. Strategies:

1. **Distance-based interpolation toggle**: disable frame interpolation
   (`VatPlaybackTweaks::disable_interpolation`) for distant instances to halve texture fetches.

2. **Speed reduction**: slow down or pause playback for off-screen or very distant instances —
   no GPU cost when `playing` is `false`.

3. **Separate VAT bakes**: create lower-vertex-count bakes for distant LODs, each with their
   own metadata and textures. Switch using Bevy's standard LOD or visibility systems.

### Crowd Rendering Best Practices

1. **Share materials**: all instances using the same VAT should share a single
   `Handle<VatMaterial>`. The crate batches all entities per material into one storage buffer
   upload and one draw call.

2. **Stagger phase offsets**: give each instance a different `VatPlayback::time_seconds` start
   value to avoid the "synchronized marching" look. A hash of the entity position works well.

3. **Mix clips**: randomly assign different clips (idle, walk variants) to break visual
   repetition. The metadata supports multiple clips in a single texture.

4. **Use animation events**: attach footstep sounds or dust particle spawns to `VatEventReached`
   messages instead of polling playback time.

## Glossary

| Term | Definition |
|---|---|
| **Proxy mesh** | The reference mesh exported alongside the VAT textures. Contains UV1 for texture lookup. |
| **UV1** | The second UV channel, used as the vertex-to-texel mapping. Called "UV2" in most DCC tools. |
| **Decode bounds** | The min/max bounding box used to normalize/denormalize position data in the texture. |
| **Animation bounds** | A bounding box encompassing all vertex positions across all frames. Used for frustum culling. |
| **Rows per frame** | How many texture rows one animation frame occupies. Usually 1, but >1 for meshes with more vertices than texture width. |
| **Position encoding** | Whether texture values decode to absolute positions or offsets from the proxy. |
| **Crossfade** | Blending two clips simultaneously during a transition, rendered in the shader. |
| **Storage buffer** | GPU buffer holding per-entity playback state (frame indices, blend weights). |
| **MeshTag** | Per-entity index into the storage buffer, assigned by the sync system. |

## Further Reading

- [SideFX Labs VAT 3.0 Documentation](https://www.sidefx.com/docs/houdini/nodes/out/labs--vertex_animation_textures-3.0.html)
- [OpenVAT Blender Extension](https://extensions.blender.org/add-ons/openvat/)
- [OpenVAT Website](https://openvat.org/)
- [NVIDIA GPU Gems 3 — Animated Crowd Rendering](https://developer.nvidia.com/gpugems/gpugems3/part-i-geometry/chapter-2-animated-crowd-rendering)
- [Stoyan Dimitrov — VAT Technical Breakdown](https://stoyan3d.wordpress.com/2021/07/23/vertex-animation-texture-vat/)
- [Snap Lens Studio — VAT Guide (covers all 4 VAT types)](https://developers.snap.com/lens-studio/assets-pipeline/3d/animation/vertex-animation-textures-guide)
- [Wildlife Studios — Texture Animation Techniques](https://medium.com/tech-at-wildlife-studios/texture-animation-techniques-1daecb316657)
- [Unity Mesh-Animation — GPU VAT Library](https://github.com/codewriter-packages/Mesh-Animation)
- [Godot VAT Plugin](https://github.com/antzGames/Godot_Vertex_Animation_Textures_Plugin)
