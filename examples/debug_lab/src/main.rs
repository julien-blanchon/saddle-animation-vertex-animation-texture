use saddle_animation_vertex_animation_texture_example_support as support;

use bevy::prelude::*;
use saddle_animation_vertex_animation_texture::{
    VatClipFinished, VatCrossfade, VatEventReached, VatMaterial, VatMaterialDefaults, VatPlayback,
    VatPlaybackTweaks,
};
use support::{
    demo_app, load_demo_assets, spawn_demo_camera, spawn_demo_environment, spawn_overlay,
    spawn_vat_actor, spin_demo_lights, write_overlay,
};

#[derive(Component)]
struct Hero;

#[derive(Component)]
struct DebugOverlay;

#[derive(Resource, Default)]
struct DebugState {
    active_clip: usize,
    paused: bool,
    interpolation_enabled: bool,
    last_event: String,
    finish_count: u32,
}

fn main() {
    let mut app = demo_app("vertex_animation_texture debug lab");
    app.init_resource::<DebugState>();
    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            spin_demo_lights,
            handle_debug_input,
            record_messages,
            update_overlay,
        ),
    );
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
        Color::srgb(0.92, 0.88, 1.0),
    );

    let hero = spawn_vat_actor(
        &mut commands,
        "Hero",
        &assets,
        VatPlayback::default(),
        Vec3::new(-0.8, 0.0, 0.0),
        Vec3::splat(2.4),
    );
    commands
        .entity(hero)
        .insert((Hero, VatPlaybackTweaks::default()));

    spawn_vat_actor(
        &mut commands,
        "Reference No Interp",
        &assets,
        VatPlayback::default().with_clip(1).with_time_seconds(0.2),
        Vec3::new(1.25, 0.0, 0.0),
        Vec3::splat(1.8),
    );

    let overlay = spawn_overlay(&mut commands, "VAT Debug Lab");
    commands.entity(overlay).insert(DebugOverlay);
}

fn handle_debug_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut state: ResMut<DebugState>,
    mut query: Query<(Entity, &mut VatPlayback, &mut VatPlaybackTweaks), With<Hero>>,
) {
    let Ok((entity, mut playback, mut tweaks)) = query.single_mut() else {
        return;
    };

    if keys.just_pressed(KeyCode::Space) {
        playback.playing = !playback.playing;
        state.paused = !playback.playing;
    }
    if keys.just_pressed(KeyCode::KeyI) {
        tweaks.disable_interpolation = !tweaks.disable_interpolation;
        state.interpolation_enabled = !tweaks.disable_interpolation;
    }

    let requested_clip = if keys.just_pressed(KeyCode::Digit1) {
        Some(0)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(1)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(2)
    } else {
        None
    };

    if let Some(clip_index) = requested_clip {
        if playback.active_clip != clip_index {
            commands.entity(entity).insert(VatCrossfade::new(
                playback.active_clip,
                clip_index,
                0.45,
            ));
        }
        state.active_clip = clip_index;
    }
}

fn record_messages(
    mut state: ResMut<DebugState>,
    mut events: MessageReader<VatEventReached>,
    mut finished: MessageReader<VatClipFinished>,
) {
    for event in events.read() {
        state.last_event = format!("{} @ frame {}", event.event_name, event.clip_frame);
    }
    for _ in finished.read() {
        state.finish_count += 1;
    }
}

fn update_overlay(
    state: Res<DebugState>,
    mut overlay: Query<&mut Text, With<DebugOverlay>>,
    hero: Single<&VatPlayback, With<Hero>>,
) {
    let Ok(mut text) = overlay.single_mut() else {
        return;
    };
    write_overlay(
        &mut text,
        "VAT Debug Lab",
        &format!(
            "clip: {}\ntime: {:.2}\npaused: {}\ninterpolation: {}\nlast event: {}\nclip finishes: {}\n\nkeys: 1/2/3 switch clips, Space pause, I toggle interpolation",
            hero.active_clip,
            hero.time_seconds,
            !hero.playing,
            state.interpolation_enabled,
            if state.last_event.is_empty() {
                "none".into()
            } else {
                state.last_event.clone()
            },
            state.finish_count,
        ),
    );
}
