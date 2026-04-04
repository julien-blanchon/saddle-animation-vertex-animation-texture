use saddle_animation_vertex_animation_texture_example_support as support;

use bevy::prelude::*;
use saddle_animation_vertex_animation_texture::{
    VatMaterial, VatMaterialDefaults, VatPlayback, VatPlaybackFollower,
};
use support::{
    VatFollowerOffsetScale, VatPaneControlled, demo_app, load_demo_assets, spawn_demo_camera,
    spawn_demo_environment, spawn_vat_actor, spin_demo_lights,
};

fn main() {
    let mut app = demo_app("vertex_animation_texture modular sync");
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
        Color::srgb(0.88, 0.96, 0.78),
    );

    let leader_scale = Vec3::splat(2.25);
    let leader = spawn_vat_actor(
        &mut commands,
        "Sync Leader",
        &assets,
        VatPlayback::default().with_clip(1),
        Vec3::new(-2.4, 0.0, 0.0),
        leader_scale,
    );
    commands.entity(leader).insert(
        VatPaneControlled::new(1.0, leader_scale).with_clip_sync(),
    );

    for (index, x) in [-0.6, 1.2, 3.0].into_iter().enumerate() {
        let follower_scale = Vec3::splat(1.8 - index as f32 * 0.12);
        let entity = spawn_vat_actor(
            &mut commands,
            &format!("Follower {}", index + 1),
            &assets,
            VatPlayback::default(),
            Vec3::new(x, 0.0, -0.25 * index as f32),
            follower_scale,
        );
        commands.entity(entity).insert((
            VatPlaybackFollower::new(leader),
            VatFollowerOffsetScale(index as f32 + 1.0),
            VatPaneControlled::new(1.0, follower_scale),
        ));
    }
}
