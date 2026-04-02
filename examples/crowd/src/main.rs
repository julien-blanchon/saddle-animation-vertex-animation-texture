use saddle_animation_vertex_animation_texture_example_support as support;

use bevy::prelude::*;
use support::{
    demo_app, load_demo_assets, spawn_demo_camera, spawn_demo_environment, spawn_vat_actor,
    spin_demo_lights,
};
use saddle_animation_vertex_animation_texture::{VatMaterial, VatMaterialDefaults, VatPlayback};

fn main() {
    let mut app = demo_app("vertex_animation_texture crowd");
    app.add_systems(Startup, setup);
    app.add_systems(Update, spin_demo_lights);
    app.run();
}

fn setup(
    mut commands: Commands,
    defaults: Res<VatMaterialDefaults>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<VatMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut buffers: ResMut<Assets<bevy::render::storage::ShaderStorageBuffer>>,
    mut animations: ResMut<Assets<saddle_animation_vertex_animation_texture::VatAnimationData>>,
) {
    spawn_demo_camera(&mut commands);
    spawn_demo_environment(&mut commands, &mut meshes, &mut standard_materials);

    let assets = load_demo_assets(
        &mut meshes,
        &mut images,
        &mut materials,
        &mut buffers,
        &mut animations,
        &defaults,
        Color::srgb(0.76, 0.88, 0.98),
    );

    for row in 0..6 {
        for column in 0..9 {
            let seed = row * 9 + column;
            let x = column as f32 * 0.75 - 3.0;
            let z = row as f32 * 0.55 - 1.2;
            let phase = (seed as f32 * 0.173).fract()
                * support::DEMO_FRAMES_PER_CLIP as f32
                / 24.0;
            let clip_index = if seed % 5 == 0 { 1 } else { 0 };
            spawn_vat_actor(
                &mut commands,
                &format!("Crowd Actor {}", seed + 1),
                &assets,
                VatPlayback::default()
                    .with_clip(clip_index)
                    .with_speed(0.85 + (seed % 7) as f32 * 0.07)
                    .with_time_seconds(phase),
                Vec3::new(x, 0.0, z),
                Vec3::splat(1.35),
            );
        }
    }
}
