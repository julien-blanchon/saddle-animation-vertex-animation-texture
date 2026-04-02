use saddle_animation_vertex_animation_texture_example_support as support;

use bevy::prelude::*;
use saddle_animation_vertex_animation_texture::{
    VatCrossfade, VatMaterial, VatMaterialDefaults, VatPlayback, VatPlaybackTweaks,
};
use support::{
    demo_app, load_demo_assets, spawn_demo_camera, spawn_demo_environment, spawn_vat_actor,
    spin_demo_lights,
};

#[derive(Component)]
struct ClipShowcase;

#[derive(Resource)]
struct ClipCycle {
    timer: Timer,
    current_index: usize,
}

fn main() {
    let mut app = demo_app("vertex_animation_texture multi clip");
    app.insert_resource(ClipCycle {
        timer: Timer::from_seconds(2.5, TimerMode::Repeating),
        current_index: 0,
    });
    app.add_systems(Startup, setup);
    app.add_systems(Update, (spin_demo_lights, cycle_clips));
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
        Color::srgb(0.98, 0.86, 0.70),
    );

    let entity = spawn_vat_actor(
        &mut commands,
        "Clip Showcase",
        &assets,
        VatPlayback::default(),
        Vec3::ZERO,
        Vec3::splat(2.3),
    );
    commands
        .entity(entity)
        .insert((ClipShowcase, VatPlaybackTweaks::default()));
}

fn cycle_clips(
    time: Res<Time>,
    mut cycle: ResMut<ClipCycle>,
    query: Query<(Entity, &VatPlayback, Option<&VatCrossfade>), With<ClipShowcase>>,
    mut commands: Commands,
) {
    if !cycle.timer.tick(time.delta()).just_finished() {
        return;
    }

    let Ok((entity, playback, crossfade)) = query.single() else {
        return;
    };
    if crossfade.is_some() {
        return;
    }

    let next_clip = (cycle.current_index + 1) % 3;
    commands
        .entity(entity)
        .insert(VatCrossfade::new(playback.active_clip, next_clip, 0.6));
    cycle.current_index = next_clip;
}
