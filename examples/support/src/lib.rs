use std::{f32::consts::TAU, thread, time::Duration};

use bevy::{
    app::AppExit,
    asset::RenderAssetUsages,
    color::LinearRgba,
    mesh::Indices,
    prelude::*,
    render::{render_resource::PrimitiveTopology, storage::ShaderStorageBuffer},
};
use saddle_animation_vertex_animation_texture::{
    VatAnimationData, VatAnimationSource, VatMaterial, VatMaterialDefaults, VatPlayback,
    VertexAnimationTexturePlugin, build_vat_material, make_linear_rgba8_image,
    parse_vat_animation_data_str,
};

pub const AUTO_EXIT_ENV: &str = "VAT_AUTO_EXIT_SECONDS";
pub const DEMO_COLUMNS: usize = 8;
pub const DEMO_ROWS: usize = 8;
pub const DEMO_FRAMES_PER_CLIP: usize = 24;
pub const DEMO_FRAME_COUNT: usize = 72;

#[derive(Clone)]
pub struct VatDemoAssets {
    pub animation: Handle<VatAnimationData>,
    pub mesh: Handle<Mesh>,
    pub material: Handle<VatMaterial>,
}

#[derive(Component)]
pub struct DemoSpinner {
    pub axis: Dir3,
    pub speed: f32,
}

#[allow(dead_code)]
#[derive(Component)]
pub struct DemoOverlay;

#[derive(Resource)]
struct AutoExitAfter(Timer);

pub fn demo_app(title: &str) -> App {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.06, 0.07, 0.10)));
    app.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 1200.0,
        affects_lightmapped_meshes: true,
    });
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: title.into(),
            resolution: (1440, 900).into(),
            ..default()
        }),
        ..default()
    }));
    app.add_plugins(VertexAnimationTexturePlugin::default());
    install_auto_exit(&mut app);
    app
}

pub fn spawn_demo_camera(commands: &mut Commands) {
    commands.spawn((
        Name::new("Demo Camera"),
        Camera3d::default(),
        Transform::from_xyz(-2.8, 1.8, 5.6).looking_at(Vec3::new(0.0, 0.8, 0.0), Vec3::Y),
    ));
}

pub fn spawn_demo_environment(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.spawn((
        Name::new("Key Light"),
        DirectionalLight {
            illuminance: 22_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 5.0).looking_at(Vec3::new(0.0, 0.7, 0.0), Vec3::Y),
        DemoSpinner {
            axis: Dir3::Y,
            speed: 0.15,
        },
    ));

    commands.spawn((
        Name::new("Accent Light"),
        PointLight {
            intensity: 3_500.0,
            range: 12.0,
            color: Color::srgb(0.3, 0.7, 1.0),
            ..default()
        },
        Transform::from_xyz(-2.0, 1.3, 2.5),
    ));

    commands.spawn((
        Name::new("Ground"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(18.0, 18.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.10, 0.12, 0.16),
            perceptual_roughness: 0.95,
            metallic: 0.02,
            ..default()
        })),
    ));

    commands.spawn((
        Name::new("Backdrop"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(10.0, 4.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.12, 0.16, 0.24),
            emissive: LinearRgba::rgb(0.02, 0.03, 0.05),
            cull_mode: None,
            unlit: false,
            ..default()
        })),
        Transform::from_translation(Vec3::new(0.0, 1.5, -2.6))
            .with_rotation(Quat::from_rotation_x(-0.05)),
    ));
}

pub fn load_demo_assets(
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<VatMaterial>,
    buffers: &mut Assets<ShaderStorageBuffer>,
    animations: &mut Assets<VatAnimationData>,
    defaults: &VatMaterialDefaults,
    color: Color,
) -> VatDemoAssets {
    let animation =
        parse_vat_animation_data_str(include_str!("../../../assets/demo/wave.vatanim.json"))
            .expect("demo metadata should be valid");
    let position_texture = images.add(build_position_texture(&animation));
    let normal_texture = images.add(build_normal_texture(&animation));
    let material = materials.add(
        build_vat_material(
            StandardMaterial {
                base_color: color,
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
            defaults,
            buffers,
        )
        .expect("demo material should build"),
    );

    VatDemoAssets {
        animation: animations.add(animation),
        mesh: meshes.add(build_demo_mesh()),
        material,
    }
}

pub fn spawn_vat_actor(
    commands: &mut Commands,
    name: &str,
    assets: &VatDemoAssets,
    playback: VatPlayback,
    translation: Vec3,
    scale: Vec3,
) -> Entity {
    commands
        .spawn((
            Name::new(name.to_owned()),
            Mesh3d(assets.mesh.clone()),
            MeshMaterial3d(assets.material.clone()),
            VatAnimationSource::new(assets.animation.clone()),
            playback,
            Transform::from_translation(translation).with_scale(scale),
        ))
        .id()
}

#[allow(dead_code)]
pub fn spawn_overlay(commands: &mut Commands, title: &str) -> Entity {
    let root = commands
        .spawn((
            Name::new("Debug Overlay"),
            Node {
                position_type: PositionType::Absolute,
                left: px(24.0),
                top: px(20.0),
                padding: UiRect::all(px(12.0)),
                border_radius: BorderRadius::all(px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.04, 0.07, 0.82)),
            DemoOverlay,
        ))
        .id();
    let text = commands
        .spawn((
            Name::new("Debug Overlay Text"),
            Text::new(title),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ))
        .id();
    commands.entity(root).add_child(text);
    text
}

#[allow(dead_code)]
pub fn write_overlay(text: &mut Text, title: &str, body: &str) {
    *text = Text::new(format!("{title}\n\n{body}"));
}

pub fn spin_demo_lights(time: Res<Time>, mut query: Query<(&DemoSpinner, &mut Transform)>) {
    for (spinner, mut transform) in &mut query {
        transform.rotate_axis(spinner.axis, spinner.speed * time.delta_secs());
    }
}

fn build_demo_mesh() -> Mesh {
    let vertex_count_x = DEMO_COLUMNS + 1;
    let vertex_count_y = DEMO_ROWS + 1;
    let total_vertices = vertex_count_x * vertex_count_y;

    let mut positions = Vec::with_capacity(total_vertices);
    let mut normals = Vec::with_capacity(total_vertices);
    let mut uv0 = Vec::with_capacity(total_vertices);
    let mut uv1 = Vec::with_capacity(total_vertices);
    let mut indices = Vec::with_capacity(DEMO_COLUMNS * DEMO_ROWS * 6);

    for row in 0..vertex_count_y {
        let v = row as f32 / DEMO_ROWS as f32;
        for column in 0..vertex_count_x {
            let u = column as f32 / DEMO_COLUMNS as f32;
            let x = u - 0.5;
            let y = v * 1.6;
            let vertex_index = (row * vertex_count_x + column) as f32;

            positions.push([x, y, 0.0]);
            normals.push([0.0, 0.0, 1.0]);
            uv0.push([u, 1.0 - v]);
            uv1.push([
                (vertex_index + 0.5) / total_vertices as f32,
                0.5 / DEMO_FRAME_COUNT as f32,
            ]);
        }
    }

    for row in 0..DEMO_ROWS {
        for column in 0..DEMO_COLUMNS {
            let top_left = (row * vertex_count_x + column) as u32;
            let top_right = top_left + 1;
            let bottom_left = top_left + vertex_count_x as u32;
            let bottom_right = bottom_left + 1;
            indices.extend_from_slice(&[
                top_left,
                bottom_left,
                top_right,
                top_right,
                bottom_left,
                bottom_right,
            ]);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv0);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, uv1);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn build_position_texture(animation: &VatAnimationData) -> Image {
    let frame_positions = generate_frame_positions();
    let bounds_min = animation.decode_bounds_min;
    let bounds_extent = animation.decode_bounds_max - animation.decode_bounds_min;

    let mut data = Vec::with_capacity(DEMO_FRAME_COUNT * frame_positions[0].len() * 4);
    for frame in &frame_positions {
        for position in frame {
            let encoded = ((*position - bounds_min) / bounds_extent).clamp(Vec3::ZERO, Vec3::ONE);
            data.extend_from_slice(&[
                (encoded.x * 255.0).round() as u8,
                (encoded.y * 255.0).round() as u8,
                (encoded.z * 255.0).round() as u8,
                255,
            ]);
        }
    }

    make_linear_rgba8_image(
        UVec2::new(frame_positions[0].len() as u32, DEMO_FRAME_COUNT as u32),
        data,
    )
}

fn build_normal_texture(animation: &VatAnimationData) -> Image {
    let frame_positions = generate_frame_positions();
    let frame_normals = generate_frame_normals(&frame_positions);
    let _ = animation;

    let mut data = Vec::with_capacity(DEMO_FRAME_COUNT * frame_normals[0].len() * 4);
    for frame in &frame_normals {
        for normal in frame {
            let encoded = (*normal * 0.5 + Vec3::splat(0.5)).clamp(Vec3::ZERO, Vec3::ONE);
            data.extend_from_slice(&[
                (encoded.x * 255.0).round() as u8,
                (encoded.y * 255.0).round() as u8,
                (encoded.z * 255.0).round() as u8,
                255,
            ]);
        }
    }

    make_linear_rgba8_image(
        UVec2::new(frame_normals[0].len() as u32, DEMO_FRAME_COUNT as u32),
        data,
    )
}

fn generate_frame_positions() -> Vec<Vec<Vec3>> {
    let vertex_count_x = DEMO_COLUMNS + 1;
    let vertex_count_y = DEMO_ROWS + 1;
    let mut frames = Vec::with_capacity(DEMO_FRAME_COUNT);

    for frame_index in 0..DEMO_FRAME_COUNT {
        let clip_index = frame_index / DEMO_FRAMES_PER_CLIP;
        let clip_phase = (frame_index % DEMO_FRAMES_PER_CLIP) as f32 / DEMO_FRAMES_PER_CLIP as f32;
        let phase = clip_phase * TAU;
        let mut frame = Vec::with_capacity(vertex_count_x * vertex_count_y);

        for row in 0..vertex_count_y {
            let v = row as f32 / DEMO_ROWS as f32;
            for column in 0..vertex_count_x {
                let u = column as f32 / DEMO_COLUMNS as f32;
                let base = Vec3::new(u - 0.5, v * 1.6, 0.0);
                frame.push(deform_position(base, clip_index, phase));
            }
        }

        frames.push(frame);
    }

    frames
}

fn generate_frame_normals(frame_positions: &[Vec<Vec3>]) -> Vec<Vec<Vec3>> {
    let vertex_count_x = DEMO_COLUMNS + 1;
    let vertex_count_y = DEMO_ROWS + 1;
    let mut frames = Vec::with_capacity(frame_positions.len());

    for frame in frame_positions {
        let mut normals = Vec::with_capacity(frame.len());
        for row in 0..vertex_count_y {
            for column in 0..vertex_count_x {
                let left = frame[row * vertex_count_x + column.saturating_sub(1)];
                let right = frame[row * vertex_count_x + (column + 1).min(DEMO_COLUMNS)];
                let down = frame[row.saturating_sub(1) * vertex_count_x + column];
                let up = frame[(row + 1).min(DEMO_ROWS) * vertex_count_x + column];
                let tangent_x = (right - left).normalize_or_zero();
                let tangent_y = (up - down).normalize_or_zero();
                let normal = tangent_x.cross(tangent_y).normalize_or_zero();
                normals.push(if normal.length_squared() > 0.0 {
                    normal
                } else {
                    Vec3::Z
                });
            }
        }
        frames.push(normals);
    }

    frames
}

fn deform_position(base: Vec3, clip_index: usize, phase: f32) -> Vec3 {
    let sway_weight = base.y / 1.6;
    match clip_index {
        0 => {
            let z = (phase + base.y * 2.6).sin() * (0.08 + sway_weight * 0.12);
            let x = base.x + (phase * 0.5 + base.y * 1.4).cos() * 0.04 * sway_weight;
            Vec3::new(x, base.y, z)
        }
        1 => {
            let z = (phase * 1.8 + base.y * 4.2).sin() * (0.12 + sway_weight * 0.18);
            let x = base.x + (phase * 1.2 + base.y * 1.7).cos() * 0.10 * sway_weight;
            let y = base.y + (phase * 1.4 + base.x * 4.0).sin() * 0.05 * sway_weight;
            Vec3::new(x, y, z)
        }
        _ => {
            let swell = (phase * 0.5).sin() * 0.12;
            let z = (phase * 2.5 + base.x * 5.0).sin() * (0.05 + sway_weight * 0.22) + swell;
            let x = base.x * (1.0 + 0.18 * (phase * 0.75).sin() * sway_weight);
            let y = base.y + (phase * 1.8).sin() * 0.12 * sway_weight;
            Vec3::new(x, y, z)
        }
    }
}

fn install_auto_exit(app: &mut App) {
    let duration = std::env::var(AUTO_EXIT_ENV)
        .ok()
        .and_then(|value| value.parse::<f32>().ok());

    if let Some(seconds) = duration.filter(|seconds| *seconds > 0.0) {
        app.insert_resource(AutoExitAfter(Timer::from_seconds(seconds, TimerMode::Once)));
        app.add_systems(Update, auto_exit_after_timer);
    }
}

fn auto_exit_after_timer(
    time: Res<Time>,
    mut timer: ResMut<AutoExitAfter>,
    mut exit: MessageWriter<AppExit>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        exit.write(AppExit::Success);
        thread::sleep(Duration::from_millis(50));
    }
}
