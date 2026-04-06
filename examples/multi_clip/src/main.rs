use saddle_animation_vertex_animation_texture_example_support as support;

use bevy::prelude::*;
use saddle_animation_vertex_animation_texture::{
    VatCrossfade, VatMaterial, VatMaterialDefaults, VatPlayback, VatPlaybackTweaks,
};
use support::{
    demo_app, load_demo_assets, spawn_demo_camera, spawn_demo_environment, spawn_overlay,
    spawn_vat_actor, spin_demo_lights, write_overlay,
};

#[derive(Component)]
struct ClipShowcase;

#[derive(Component)]
struct Overlay;

#[derive(Resource)]
struct ClipCycle {
    timer: Timer,
    next_clip_name: &'static str,
}

fn main() {
    let mut app = demo_app("vertex_animation_texture multi clip");
    app.insert_resource(ClipCycle {
        timer: Timer::from_seconds(2.5, TimerMode::Repeating),
        next_clip_name: support::DEMO_CLIP_GUST,
    });
    app.add_systems(Startup, setup);
    app.add_systems(Update, (spin_demo_lights, cycle_clips, update_overlay));
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

    let overlay = spawn_overlay(&mut commands, "VAT Multi Clip");
    commands.entity(overlay).insert(Overlay);
}

fn cycle_clips(
    time: Res<Time>,
    mut cycle: ResMut<ClipCycle>,
    animations: Res<Assets<saddle_animation_vertex_animation_texture::VatAnimationData>>,
    query: Query<
        (
            Entity,
            &VatPlayback,
            &saddle_animation_vertex_animation_texture::VatAnimationSource,
            Option<&VatCrossfade>,
        ),
        With<ClipShowcase>,
    >,
    mut commands: Commands,
) {
    if !cycle.timer.tick(time.delta()).just_finished() {
        return;
    }

    let Ok((entity, playback, source, crossfade)) = query.single() else {
        return;
    };
    if crossfade.is_some() {
        return;
    }

    let Some(animation) = animations.get(&source.animation) else {
        return;
    };
    if let Ok(crossfade) =
        VatCrossfade::to_clip_name(animation, playback, cycle.next_clip_name, 0.6)
    {
        commands.entity(entity).insert(crossfade);
        cycle.next_clip_name = next_demo_clip_name(Some(cycle.next_clip_name));
    }
}

fn update_overlay(
    mut overlay: Query<&mut Text, With<Overlay>>,
    showcase: Query<
        (
            &VatPlayback,
            &saddle_animation_vertex_animation_texture::VatAnimationSource,
            Option<&VatCrossfade>,
        ),
        With<ClipShowcase>,
    >,
    cycle: Res<ClipCycle>,
    animations: Res<Assets<saddle_animation_vertex_animation_texture::VatAnimationData>>,
) {
    let Ok(mut text) = overlay.single_mut() else {
        return;
    };
    let (playback, source, crossfade) = showcase.single().unwrap();
    let active_clip_name = animations
        .get(&source.animation)
        .and_then(|animation| playback.active_clip_name(animation))
        .unwrap_or("resolving");
    write_overlay(
        &mut text,
        "VAT Multi Clip",
        &format!(
            "Auto-cycles through named clips with crossfade.\nClips share a single baked VAT texture.\n\nactive clip: {}  next clip: {}\ntime: {:.2}  crossfading: {}\n\nCrossfade blends two clips in the vertex\nshader with metadata-resolved clip names.",
            active_clip_name,
            cycle.next_clip_name,
            playback.time_seconds,
            crossfade.is_some(),
        ),
    );
}

fn next_demo_clip_name(current: Option<&str>) -> &'static str {
    match current {
        Some(support::DEMO_CLIP_IDLE) => support::DEMO_CLIP_GUST,
        Some(support::DEMO_CLIP_GUST) => support::DEMO_CLIP_BURST,
        _ => support::DEMO_CLIP_IDLE,
    }
}
