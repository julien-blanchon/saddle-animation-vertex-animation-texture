use saddle_animation_vertex_animation_texture_example_support as support;

use bevy::{color::LinearRgba, prelude::*};
use saddle_animation_vertex_animation_texture::{
    VatAnimationData, VatAnimationSource, VatMaterial, VatMaterialDefaults, VatPlayback,
    VatPlaybackFollower, VatPlaybackTweaks, VertexAnimationTexturePlugin, build_vat_material,
    parse_vat_animation_data_str,
};
use support::{VatFollowerOffsetScale, VatPaneControlled, spin_demo_lights};

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.06, 0.07, 0.10)));
    app.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 1200.0,
        affects_lightmapped_meshes: true,
    });
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "vertex_animation_texture modular sync".into(),
            resolution: (1440, 900).into(),
            ..default()
        }),
        ..default()
    }));
    app.init_resource::<support::VatExamplePane>();
    app.add_plugins((
        bevy_flair::FlairPlugin,
        bevy_input_focus::InputDispatchPlugin,
        bevy_ui_widgets::UiWidgetsPlugins,
        bevy_input_focus::tab_navigation::TabNavigationPlugin,
        saddle_pane::PanePlugin,
    ))
    .register_pane::<support::VatExamplePane>();
    app.add_plugins(VertexAnimationTexturePlugin::default());
    app.add_systems(PreUpdate, support::sync_vat_pane);
    app.add_systems(PostUpdate, support::reflect_vat_pane);
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
    mut animations: ResMut<Assets<VatAnimationData>>,
) {
    // -- Camera --
    commands.spawn((
        Name::new("Demo Camera"),
        Camera3d::default(),
        Transform::from_xyz(-2.8, 1.8, 5.6).looking_at(Vec3::new(0.0, 0.8, 0.0), Vec3::Y),
    ));

    // -- Environment (lights, ground) --
    support::spawn_demo_environment(&mut commands, &mut meshes, &mut standard_materials);

    // -- Load VAT animation data --
    let animation =
        parse_vat_animation_data_str(include_str!("../../../assets/demo/wave.vatanim.json"))
            .expect("demo metadata should be valid");

    // -- Build textures and material --
    let position_texture = images.add(support::build_position_texture(&animation));
    let normal_texture = images.add(support::build_normal_texture(&animation));
    let material = materials.add(
        build_vat_material(
            StandardMaterial {
                base_color: Color::srgb(0.88, 0.96, 0.78),
                emissive: LinearRgba::rgb(0.02, 0.02, 0.03),
                perceptual_roughness: 0.82,
                metallic: 0.0,
                cull_mode: None,
                double_sided: true,
                ..default()
            },
            &animation,
            position_texture,
            Some(normal_texture),
            &defaults,
            &mut buffers,
        )
        .expect("demo material should build"),
    );
    let mesh = meshes.add(support::build_demo_mesh());
    let animation_handle = animations.add(animation);

    // -- Spawn the leader actor --
    let leader_scale = Vec3::splat(2.25);
    let leader = commands
        .spawn((
            Name::new("Sync Leader"),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            VatAnimationSource::new(animation_handle.clone()),
            VatPlaybackTweaks::default(),
            VatPaneControlled::new(1.0, leader_scale).with_clip_sync(),
            VatPlayback::default().with_clip(1),
            Transform::from_translation(Vec3::new(-2.4, 0.0, 0.0)).with_scale(leader_scale),
        ))
        .id();

    // -- Spawn followers that track the leader's playback with time offsets --
    for (index, x) in [-0.6, 1.2, 3.0].into_iter().enumerate() {
        let follower_scale = Vec3::splat(1.8 - index as f32 * 0.12);
        commands.spawn((
            Name::new(format!("Follower {}", index + 1)),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            VatAnimationSource::new(animation_handle.clone()),
            VatPlaybackTweaks::default(),
            VatPaneControlled::new(1.0, follower_scale),
            VatPlayback::default(),
            // VatPlaybackFollower makes this entity mirror the leader's playback
            // with an adjustable time offset.
            VatPlaybackFollower::new(leader),
            VatFollowerOffsetScale(index as f32 + 1.0),
            Transform::from_translation(Vec3::new(x, 0.0, -0.25 * index as f32))
                .with_scale(follower_scale),
        ));
    }
}
