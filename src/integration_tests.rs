use bevy::{
    asset::RenderAssetUsages,
    ecs::schedule::ScheduleLabel,
    mesh::Indices,
    prelude::*,
    render::{render_resource::PrimitiveTopology, storage::ShaderStorageBuffer},
};

use crate::{
    VatAnimationData, VatAnimationMode, VatAnimationSource, VatBoundsMode, VatClip,
    VatCoordinateSystem, VatMaterial, VatMaterialDefaults, VatNormalTexture, VatPlayback,
    VatPlaybackSpace, VatPositionEncoding, VatSourceFormat, VatTextureDescriptor,
    VatTexturePrecision, VatVertexIdAttribute, VertexAnimationTexturePlugin, build_vat_material,
    make_linear_rgba8_image, systems::{VatBindingFailure, VatBindingReady},
};

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct TestActivate;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct TestDeactivate;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct TestUpdate;

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None,
                ..default()
            })
            .disable::<bevy::winit::WinitPlugin>()
            .disable::<bevy::log::LogPlugin>(),
    );
    app
}

fn make_animation(vertex_count: u32) -> VatAnimationData {
    VatAnimationData {
        source_format: VatSourceFormat::Canonical,
        animation_mode: VatAnimationMode::SoftBodyFixedTopology,
        vertex_count,
        frame_count: 8,
        frames_per_second: 8.0,
        decode_bounds_min: Vec3::new(-1.0, 0.0, -0.25),
        decode_bounds_max: Vec3::new(1.0, 1.0, 0.25),
        animation_bounds_min: Vec3::new(-1.0, 0.0, -0.25),
        animation_bounds_max: Vec3::new(1.0, 1.0, 0.25),
        clips: vec![VatClip {
            name: "loop".into(),
            start_frame: 0,
            end_frame: 7,
            default_loop_mode: Some(crate::VatLoopMode::Loop),
            events: Vec::new(),
        }],
        position_texture: VatTextureDescriptor {
            relative_path: None,
            width: vertex_count,
            height: 8,
            rows_per_frame: 1,
            precision: VatTexturePrecision::Png8,
        },
        normal_texture: VatNormalTexture::Separate {
            texture: VatTextureDescriptor {
                relative_path: None,
                width: vertex_count,
                height: 8,
                rows_per_frame: 1,
                precision: VatTexturePrecision::Png8,
            },
            encoding: crate::VatNormalEncoding::SignedNormalized,
        },
        rotation_texture: None,
        auxiliary_textures: Vec::new(),
        coordinate_system: VatCoordinateSystem::YUpRightHanded,
        playback_space: VatPlaybackSpace::Local,
        vertex_id_attribute: VatVertexIdAttribute::Uv1,
        position_encoding: VatPositionEncoding::AbsoluteNormalizedBounds,
    }
}

fn make_textures(vertex_count: u32) -> (Image, Image) {
    let mut position_data = Vec::with_capacity(vertex_count as usize * 8 * 4);
    let mut normal_data = Vec::with_capacity(vertex_count as usize * 8 * 4);

    for frame in 0..8 {
        for vertex in 0..vertex_count {
            let x = vertex as f32 / (vertex_count.saturating_sub(1).max(1)) as f32;
            let y = frame as f32 / 7.0;
            position_data.extend_from_slice(&[
                (x * 255.0).round() as u8,
                (y * 255.0).round() as u8,
                128,
                255,
            ]);
            normal_data.extend_from_slice(&[128, 128, 255, 255]);
        }
    }

    (
        make_linear_rgba8_image(UVec2::new(vertex_count, 8), position_data),
        make_linear_rgba8_image(UVec2::new(vertex_count, 8), normal_data),
    )
}

fn make_mesh(include_uv1: bool) -> Mesh {
    let positions = vec![
        [-0.5, 0.0, 0.0],
        [0.5, 0.0, 0.0],
        [-0.5, 1.0, 0.0],
        [0.5, 1.0, 0.0],
    ];
    let normals = vec![[0.0, 0.0, 1.0]; 4];
    let uv0 = vec![[0.0, 1.0], [1.0, 1.0], [0.0, 0.0], [1.0, 0.0]];

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv0);
    if include_uv1 {
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_UV_1,
            vec![
                [0.125, 0.0625],
                [0.375, 0.0625],
                [0.625, 0.0625],
                [0.875, 0.0625],
            ],
        );
    }
    mesh.insert_indices(Indices::U32(vec![0, 2, 1, 1, 2, 3]));
    mesh
}

#[test]
fn plugin_builds_and_registers_material_assets() {
    let mut app = test_app();
    app.add_plugins(VertexAnimationTexturePlugin::default());
    assert!(app.world().contains_resource::<Assets<VatAnimationData>>());
    assert!(app.world().contains_resource::<Assets<VatMaterial>>());
}

#[test]
fn injectable_schedules_are_honored() {
    let mut app = test_app();
    app.add_plugins(VertexAnimationTexturePlugin::new(
        TestActivate,
        TestDeactivate,
        TestUpdate,
    ));

    let runtime = app.world().resource::<crate::systems::VatRuntimeState>();
    assert!(!runtime.active);
    app.world_mut().run_schedule(TestActivate);
    assert!(app.world().resource::<crate::systems::VatRuntimeState>().active);
    app.world_mut().run_schedule(TestDeactivate);
    assert!(!app.world().resource::<crate::systems::VatRuntimeState>().active);
}

#[test]
fn multiple_entities_can_animate_independently() {
    let mut app = test_app();
    app.add_plugins(VertexAnimationTexturePlugin::new(
        TestActivate,
        TestDeactivate,
        TestUpdate,
    ));

    let animation = make_animation(4);
    let (position_texture, normal_texture) = make_textures(4);
    let animation_handle = app.world_mut().resource_mut::<Assets<VatAnimationData>>().add(animation.clone());
    let position_texture_handle = app.world_mut().resource_mut::<Assets<Image>>().add(position_texture);
    let normal_texture_handle = app.world_mut().resource_mut::<Assets<Image>>().add(normal_texture);
    let mesh_handle = app.world_mut().resource_mut::<Assets<Mesh>>().add(make_mesh(true));

    let defaults = app.world().resource::<VatMaterialDefaults>().clone();
    let material = {
        let mut buffers = app.world_mut().resource_mut::<Assets<ShaderStorageBuffer>>();
        build_vat_material(
            StandardMaterial::default(),
            &animation,
            position_texture_handle,
            Some(normal_texture_handle),
            &defaults,
            &mut buffers,
        )
        .unwrap()
    };
    let material_handle = app.world_mut().resource_mut::<Assets<VatMaterial>>().add(material);

    app.world_mut().spawn((
        Mesh3d(mesh_handle.clone()),
        MeshMaterial3d(material_handle.clone()),
        VatAnimationSource::new(animation_handle.clone()),
        VatPlayback::default().with_time_seconds(0.1),
    ));
    app.world_mut().spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(material_handle.clone()),
        VatAnimationSource::new(animation_handle),
        VatPlayback::default().with_time_seconds(0.45),
    ));

    app.world_mut().run_schedule(TestActivate);
    app.world_mut().run_schedule(TestUpdate);

    let tags: Vec<u32> = {
        let world = app.world_mut();
        let mut query = world.query::<&bevy::mesh::MeshTag>();
        query.iter(world).map(|tag| tag.0).collect()
    };
    assert_eq!(tags.len(), 2);
    assert_eq!(tags, vec![0, 1]);

    let material = app.world().resource::<Assets<VatMaterial>>().get(&material_handle).unwrap();
    let buffer = app.world().resource::<Assets<ShaderStorageBuffer>>().get(&material.extension.instances).unwrap();
    assert!(buffer.data.as_ref().is_some_and(|bytes| !bytes.is_empty()));
}

#[test]
fn invalid_asset_combinations_fail_clearly() {
    let mut app = test_app();
    app.add_plugins(VertexAnimationTexturePlugin::new(
        TestActivate,
        TestDeactivate,
        TestUpdate,
    ));

    let animation = make_animation(4);
    let (position_texture, normal_texture) = make_textures(4);
    let animation_handle = app.world_mut().resource_mut::<Assets<VatAnimationData>>().add(animation.clone());
    let position_texture_handle = app.world_mut().resource_mut::<Assets<Image>>().add(position_texture);
    let normal_texture_handle = app.world_mut().resource_mut::<Assets<Image>>().add(normal_texture);
    let mesh_handle = app.world_mut().resource_mut::<Assets<Mesh>>().add(make_mesh(false));

    let defaults = app.world().resource::<VatMaterialDefaults>().clone();
    let material = {
        let mut buffers = app.world_mut().resource_mut::<Assets<ShaderStorageBuffer>>();
        build_vat_material(
            StandardMaterial::default(),
            &animation,
            position_texture_handle,
            Some(normal_texture_handle),
            &defaults,
            &mut buffers,
        )
        .unwrap()
    };
    let material_handle = app.world_mut().resource_mut::<Assets<VatMaterial>>().add(material);

    let entity = app.world_mut().spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(material_handle),
        VatAnimationSource::new(animation_handle).with_bounds_mode(VatBoundsMode::UseMetadataAabb),
        VatPlayback::default(),
    )).id();

    app.world_mut().run_schedule(TestActivate);
    app.world_mut().run_schedule(TestUpdate);

    assert!(app.world().entity(entity).contains::<VatBindingFailure>());
}

#[test]
fn cleanup_path_does_not_panic() {
    let mut app = test_app();
    app.add_plugins(VertexAnimationTexturePlugin::new(
        TestActivate,
        TestDeactivate,
        TestUpdate,
    ));

    let animation = make_animation(4);
    let (position_texture, normal_texture) = make_textures(4);
    let animation_handle = app.world_mut().resource_mut::<Assets<VatAnimationData>>().add(animation.clone());
    let position_texture_handle = app.world_mut().resource_mut::<Assets<Image>>().add(position_texture);
    let normal_texture_handle = app.world_mut().resource_mut::<Assets<Image>>().add(normal_texture);
    let mesh_handle = app.world_mut().resource_mut::<Assets<Mesh>>().add(make_mesh(true));

    let defaults = app.world().resource::<VatMaterialDefaults>().clone();
    let material = {
        let mut buffers = app.world_mut().resource_mut::<Assets<ShaderStorageBuffer>>();
        build_vat_material(
            StandardMaterial::default(),
            &animation,
            position_texture_handle,
            Some(normal_texture_handle),
            &defaults,
            &mut buffers,
        )
        .unwrap()
    };
    let material_handle = app.world_mut().resource_mut::<Assets<VatMaterial>>().add(material);

    let entity = app.world_mut().spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(material_handle),
        VatAnimationSource::new(animation_handle),
        VatPlayback::default(),
    )).id();

    app.world_mut().run_schedule(TestActivate);
    app.world_mut().run_schedule(TestUpdate);
    app.world_mut().despawn(entity);
    app.world_mut().run_schedule(TestUpdate);
}

#[test]
fn bindings_become_ready_after_assets_arrive_late() {
    let mut app = test_app();
    app.add_plugins(VertexAnimationTexturePlugin::new(
        TestActivate,
        TestDeactivate,
        TestUpdate,
    ));

    let animation = make_animation(4);
    let (position_texture, normal_texture) = make_textures(4);
    let mesh = make_mesh(true);

    let animation_handle = {
        let animations = app.world().resource::<Assets<VatAnimationData>>();
        animations.reserve_handle()
    };
    let mesh_handle = {
        let meshes = app.world().resource::<Assets<Mesh>>();
        meshes.reserve_handle()
    };
    let position_texture_handle = {
        let images = app.world().resource::<Assets<Image>>();
        images.reserve_handle()
    };
    let normal_texture_handle = {
        let images = app.world().resource::<Assets<Image>>();
        images.reserve_handle()
    };

    let defaults = app.world().resource::<VatMaterialDefaults>().clone();
    let material = {
        let mut buffers = app.world_mut().resource_mut::<Assets<ShaderStorageBuffer>>();
        build_vat_material(
            StandardMaterial::default(),
            &animation,
            position_texture_handle.clone(),
            Some(normal_texture_handle.clone()),
            &defaults,
            &mut buffers,
        )
        .unwrap()
    };
    let material_handle = app.world_mut().resource_mut::<Assets<VatMaterial>>().add(material);

    let entity = app
        .world_mut()
        .spawn((
            Mesh3d(mesh_handle.clone()),
            MeshMaterial3d(material_handle),
            VatAnimationSource::new(animation_handle.clone()),
            VatPlayback::default(),
        ))
        .id();

    app.world_mut().run_schedule(TestActivate);
    app.world_mut().run_schedule(TestUpdate);
    assert!(!app.world().entity(entity).contains::<VatBindingReady>());

    app.world_mut()
        .resource_mut::<Assets<VatAnimationData>>()
        .insert(animation_handle.id(), animation)
        .unwrap();
    app.world_mut()
        .resource_mut::<Assets<Mesh>>()
        .insert(mesh_handle.id(), mesh)
        .unwrap();
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .insert(position_texture_handle.id(), position_texture)
        .unwrap();
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .insert(normal_texture_handle.id(), normal_texture)
        .unwrap();

    app.world_mut().run_schedule(TestUpdate);

    assert!(app.world().entity(entity).contains::<VatBindingReady>());
    assert!(app.world().entity(entity).contains::<bevy::mesh::MeshTag>());
}

#[test]
fn completed_crossfade_is_cleaned_up() {
    let mut app = test_app();
    app.add_plugins(VertexAnimationTexturePlugin::new(
        TestActivate,
        TestDeactivate,
        TestUpdate,
    ));

    let entity = app
        .world_mut()
        .spawn((
            VatPlayback::default(),
            crate::VatCrossfade {
                from_clip: 0,
                to_clip: 1,
                elapsed: 0.5,
                duration: 0.5,
            },
            crate::systems::VatCrossfadeRuntime {
                source_clip: 0,
                source_time_seconds: 0.2,
                source_direction: 1.0,
            },
        ))
        .id();

    app.world_mut().run_schedule(TestActivate);
    app.world_mut().run_schedule(TestUpdate);

    assert!(!app.world().entity(entity).contains::<crate::VatCrossfade>());
    assert!(!app.world().entity(entity).contains::<crate::systems::VatCrossfadeRuntime>());
}
