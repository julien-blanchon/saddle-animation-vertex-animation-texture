# saddle-animation-vertex-animation-texture lab

Purpose: crate-local BRP and E2E harness for the `saddle-animation-vertex-animation-texture` shared crate.

Status: working

How to run:

```bash
cargo run -p saddle-animation-vertex-animation-texture-lab
```

With BRP:

```bash
BRP_EXTRAS_PORT=15716 cargo run -p saddle-animation-vertex-animation-texture-lab
```

With E2E:

```bash
cargo run -p saddle-animation-vertex-animation-texture-lab --features e2e -- vat_smoke
```

Available scenarios:

- `vat_smoke`
- `vat_multi_clip`
- `vat_crowd`
- `vat_bounds_regression`
- `vat_crossfade`

Findings:

- The lab uses a shared material for the crowd path, so `MeshTag`-indexed storage buffer uploads can be observed under BRP while the hero and bounds probe keep independent playback state.
- The bounds probe stays near the edge of the camera framing so culling regressions show up quickly in screenshots.
