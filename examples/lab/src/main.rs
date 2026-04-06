#[cfg(feature = "e2e")]
mod e2e;
#[cfg(feature = "e2e")]
mod scenarios;

use saddle_animation_vertex_animation_texture_example_support as support;

use bevy::prelude::*;
#[cfg(feature = "dev")]
use bevy::remote::{RemotePlugin, http::RemoteHttpPlugin};
#[cfg(feature = "dev")]
use bevy_brp_extras::BrpExtrasPlugin;
use saddle_animation_vertex_animation_texture::{
    VatClipFinished, VatCrossfade, VatEventReached, VatMaterial, VatMaterialDefaults, VatPlayback,
    VatPlaybackFollower, VatPlaybackTweaks,
};
use support::{
    VatFollowerOffsetScale, demo_app, load_demo_assets, spawn_demo_camera, spawn_demo_environment,
    spawn_overlay, spawn_vat_actor, spin_demo_lights, write_overlay,
};

#[derive(Component)]
pub struct Hero;

#[derive(Component)]
pub struct CrowdMember;

#[derive(Component)]
pub struct BoundsProbe;

#[derive(Component)]
pub struct HeroFollower;

#[derive(Component)]
struct Overlay;

#[derive(Resource, Clone, Debug)]
#[allow(dead_code)]
pub struct LabControl {
    pub auto: bool,
    pub requested_clip_name: String,
    pub interpolation_enabled: bool,
    pub paused: bool,
}

impl Default for LabControl {
    fn default() -> Self {
        Self {
            auto: true,
            requested_clip_name: support::DEMO_CLIP_IDLE.into(),
            interpolation_enabled: true,
            paused: false,
        }
    }
}

#[derive(Resource, Clone, Debug, Default, Reflect)]
#[reflect(Resource)]
pub struct LabDiagnostics {
    pub hero_clip: usize,
    pub hero_clip_name: String,
    pub hero_time: f32,
    pub hero_playing: bool,
    pub event_count: u32,
    pub finish_count: u32,
    pub last_event: String,
    pub crowd_phase_span: f32,
    pub bounds_probe_visible: bool,
    pub crossfade_active: bool,
    pub follower_sync_error: f32,
    pub follower_count: u32,
}

fn main() {
    let mut app = demo_app("vertex_animation_texture crate-local lab");
    app.init_resource::<LabControl>();
    app.init_resource::<LabDiagnostics>();
    app.register_type::<LabDiagnostics>();
    #[cfg(feature = "dev")]
    app.add_plugins(RemotePlugin::default());
    #[cfg(feature = "dev")]
    app.add_plugins(BrpExtrasPlugin::with_http_plugin(
        RemoteHttpPlugin::default().with_port(lab_brp_port()),
    ));
    #[cfg(feature = "e2e")]
    app.add_plugins(e2e::VatLabE2EPlugin);
    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            spin_demo_lights,
            drive_auto_mode,
            handle_manual_input,
            apply_control,
            record_messages,
            refresh_diagnostics,
            update_overlay,
        ),
    );
    app.run();
}

#[cfg(feature = "dev")]
fn lab_brp_port() -> u16 {
    std::env::var("BRP_EXTRAS_PORT")
        .or_else(|_| std::env::var("BRP_PORT"))
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(15_716)
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
        Color::srgb(0.92, 0.90, 1.0),
    );

    let hero = spawn_vat_actor(
        &mut commands,
        "Lab Hero",
        &assets,
        VatPlayback::default(),
        Vec3::new(-0.7, 0.0, 0.0),
        Vec3::splat(2.35),
    );
    commands
        .entity(hero)
        .insert((Hero, VatPlaybackTweaks::default()));

    for (index, x) in [0.9_f32, 2.2_f32].into_iter().enumerate() {
        let follower_scale = Vec3::splat(1.65 - index as f32 * 0.1);
        let follower = spawn_vat_actor(
            &mut commands,
            &format!("Hero Follower {}", index + 1),
            &assets,
            VatPlayback::default(),
            Vec3::new(x, 0.0, -0.55 - index as f32 * 0.25),
            follower_scale,
        );
        commands.entity(follower).insert((
            HeroFollower,
            VatPlaybackFollower::new(hero),
            VatFollowerOffsetScale(index as f32 + 1.0),
        ));
    }

    let probe = spawn_vat_actor(
        &mut commands,
        "Bounds Probe",
        &assets,
        VatPlayback::default()
            .with_clip_name(support::DEMO_CLIP_GUST)
            .with_time_seconds(0.2),
        Vec3::new(2.2, 0.0, -0.4),
        Vec3::splat(2.1),
    );
    commands.entity(probe).insert(BoundsProbe);

    for row in 0..4 {
        for column in 0..6 {
            let seed = row * 6 + column;
            let entity = spawn_vat_actor(
                &mut commands,
                &format!("Crowd Member {}", seed + 1),
                &assets,
                VatPlayback::default()
                    .with_clip_name(if seed % 4 == 0 {
                        support::DEMO_CLIP_GUST
                    } else {
                        support::DEMO_CLIP_IDLE
                    })
                    .with_speed(0.85 + (seed % 5) as f32 * 0.09)
                    .with_time_seconds((seed as f32 * 0.111).fract()),
                Vec3::new(column as f32 * 0.78 - 2.1, 0.0, row as f32 * 0.65 + 1.1),
                Vec3::splat(1.2),
            );
            commands.entity(entity).insert(CrowdMember);
        }
    }

    let overlay = spawn_overlay(&mut commands, "VAT crate-local lab");
    commands.entity(overlay).insert(Overlay);
}

fn drive_auto_mode(time: Res<Time>, mut control: ResMut<LabControl>) {
    if !control.auto {
        return;
    }

    let phase = time.elapsed_secs().rem_euclid(7.0);
    control.requested_clip_name = if phase < 2.2 {
        support::DEMO_CLIP_IDLE.into()
    } else if phase < 4.6 {
        support::DEMO_CLIP_GUST.into()
    } else {
        support::DEMO_CLIP_BURST.into()
    };
    control.paused = false;
}

fn handle_manual_input(keys: Res<ButtonInput<KeyCode>>, mut control: ResMut<LabControl>) {
    if keys.just_pressed(KeyCode::Space) {
        control.auto = false;
        control.paused = !control.paused;
    }
    if keys.just_pressed(KeyCode::KeyI) {
        control.auto = false;
        control.interpolation_enabled = !control.interpolation_enabled;
    }
    if keys.just_pressed(KeyCode::Digit1) {
        control.auto = false;
        control.requested_clip_name = support::DEMO_CLIP_IDLE.into();
    }
    if keys.just_pressed(KeyCode::Digit2) {
        control.auto = false;
        control.requested_clip_name = support::DEMO_CLIP_GUST.into();
    }
    if keys.just_pressed(KeyCode::Digit3) {
        control.auto = false;
        control.requested_clip_name = support::DEMO_CLIP_BURST.into();
    }
}

fn apply_control(
    animations: Res<Assets<saddle_animation_vertex_animation_texture::VatAnimationData>>,
    mut commands: Commands,
    control: Res<LabControl>,
    mut hero: Single<
        (
            Entity,
            &saddle_animation_vertex_animation_texture::VatAnimationSource,
            &mut VatPlayback,
            &mut VatPlaybackTweaks,
            Option<&VatCrossfade>,
        ),
        With<Hero>,
    >,
) {
    hero.2.playing = !control.paused;
    hero.3.disable_interpolation = !control.interpolation_enabled;

    if hero.4.is_none()
        && let Some(animation) = animations.get(&hero.1.animation)
        && hero.2.active_clip_name(animation) != Some(control.requested_clip_name.as_str())
        && let Ok(crossfade) =
            VatCrossfade::to_clip_name(animation, &hero.2, &control.requested_clip_name, 0.5)
    {
        commands.entity(hero.0).insert(crossfade);
    }
}

fn record_messages(
    mut diagnostics: ResMut<LabDiagnostics>,
    mut events: MessageReader<VatEventReached>,
    mut finished: MessageReader<VatClipFinished>,
) {
    for event in events.read() {
        diagnostics.event_count += 1;
        diagnostics.last_event = event.event_name.clone();
    }
    for _ in finished.read() {
        diagnostics.finish_count += 1;
    }
}

fn refresh_diagnostics(
    mut diagnostics: ResMut<LabDiagnostics>,
    hero: Single<
        (
            &VatPlayback,
            &saddle_animation_vertex_animation_texture::VatAnimationSource,
            Option<&VatCrossfade>,
        ),
        With<Hero>,
    >,
    crowd: Query<&VatPlayback, With<CrowdMember>>,
    followers: Query<(&VatPlaybackFollower, &VatPlayback), With<HeroFollower>>,
    probe_visibility: Single<&ViewVisibility, With<BoundsProbe>>,
    animations: Res<Assets<saddle_animation_vertex_animation_texture::VatAnimationData>>,
) {
    diagnostics.hero_clip = hero.0.active_clip.unwrap_or_default();
    diagnostics.hero_clip_name = animations
        .get(&hero.1.animation)
        .and_then(|animation| hero.0.active_clip_name(animation))
        .unwrap_or("resolving")
        .to_owned();
    diagnostics.hero_time = hero.0.time_seconds;
    diagnostics.hero_playing = hero.0.playing;
    diagnostics.crossfade_active = hero.2.is_some();
    diagnostics.bounds_probe_visible = probe_visibility.get();

    let mut min_time = f32::MAX;
    let mut max_time = f32::MIN;
    for playback in &crowd {
        min_time = min_time.min(playback.time_seconds);
        max_time = max_time.max(playback.time_seconds);
    }
    diagnostics.crowd_phase_span = if min_time.is_finite() && max_time.is_finite() {
        max_time - min_time
    } else {
        0.0
    };

    diagnostics.follower_count = followers.iter().count() as u32;
    diagnostics.follower_sync_error = followers
        .iter()
        .map(|(follower, playback)| {
            let expected = diagnostics.hero_time + follower.time_offset_seconds;
            (playback.time_seconds - expected).abs()
        })
        .fold(0.0, f32::max);
}

fn update_overlay(
    diagnostics: Res<LabDiagnostics>,
    control: Res<LabControl>,
    mut overlay: Query<&mut Text, With<Overlay>>,
) {
    let Ok(mut text) = overlay.single_mut() else {
        return;
    };
    write_overlay(
        &mut text,
        "VAT crate-local lab",
        &format!(
            "hero clip: {} (# {})\nhero time: {:.2}\nplaying: {}\ninterpolation: {}\nlast event: {}\nfinishes: {}\ncrowd span: {:.2}\nfollowers: {} (max sync error {:.3})\nbounds probe visible: {}\ncrossfade active: {}\n\nauto: {}\nkeys: 1 idle, 2 gust, 3 burst, Space pause, I interpolation",
            diagnostics.hero_clip_name,
            diagnostics.hero_clip,
            diagnostics.hero_time,
            diagnostics.hero_playing,
            control.interpolation_enabled,
            if diagnostics.last_event.is_empty() {
                "none".into()
            } else {
                diagnostics.last_event.clone()
            },
            diagnostics.finish_count,
            diagnostics.crowd_phase_span,
            diagnostics.follower_count,
            diagnostics.follower_sync_error,
            diagnostics.bounds_probe_visible,
            diagnostics.crossfade_active,
            control.auto,
        ),
    );
}
