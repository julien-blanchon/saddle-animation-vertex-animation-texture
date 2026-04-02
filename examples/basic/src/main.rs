use saddle_animation_vertex_animation_texture_example_support as support;

use bevy::prelude::*;
use saddle_animation_vertex_animation_texture::{VatMaterial, VatMaterialDefaults, VatPlayback};
use support::{
    demo_app, load_demo_assets, spawn_demo_camera, spawn_demo_environment, spawn_vat_actor,
    spin_demo_lights,
};

fn main() {
    let mut app = demo_app("vertex_animation_texture basic");
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
        Color::srgb(0.86, 0.92, 1.0),
    );

    spawn_vat_actor(
        &mut commands,
        "Basic VAT Mesh",
        &assets,
        VatPlayback::default(),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::splat(2.2),
    );
}
