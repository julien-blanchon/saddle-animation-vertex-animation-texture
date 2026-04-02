use bevy::{
    asset::{RenderAssetUsages, load_internal_asset, uuid_handle},
    pbr::{ExtendedMaterial, MaterialExtension, MaterialExtensionPipeline, StandardMaterial},
    prelude::*,
    reflect::TypePath,
    render::{
        render_resource::{
            AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
        },
        storage::ShaderStorageBuffer,
    },
    shader::{Shader, ShaderRef},
};
use thiserror::Error;

use crate::{
    VatAnimationData, VatCoordinateSystem, VatNormalTexture, VatPlaybackSpace, VatPositionEncoding,
    configure_vat_data_image,
};

pub type VatMaterial = ExtendedMaterial<StandardMaterial, VatMaterialExt>;

pub(crate) const VAT_FORWARD_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("e26c71d7-1c9a-4b43-8fa8-597dd59f2f42");
pub(crate) const VAT_PREPASS_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("55e90846-f96a-4d98-a775-08ab24513c29");

#[derive(Clone, Copy, Debug, ShaderType)]
pub(crate) struct VatGpuInstance {
    pub primary_frames: Vec4,
    pub secondary_frames: Vec4,
    pub options: Vec4,
}

impl Default for VatGpuInstance {
    fn default() -> Self {
        Self {
            primary_frames: Vec4::ZERO,
            secondary_frames: Vec4::ZERO,
            options: Vec4::new(1.0, 0.0, 0.0, 0.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct VatMaterialUniform {
    pub decode_min: Vec4,
    pub decode_extent: Vec4,
    pub texture_layout: UVec4,
    pub normal_layout: UVec4,
    pub modes: UVec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct VatMaterialExt {
    #[texture(100)]
    #[sampler(101)]
    pub position_texture: Handle<Image>,
    #[texture(102)]
    #[sampler(103)]
    pub normal_texture: Handle<Image>,
    #[uniform(104)]
    pub uniform: VatMaterialUniform,
    #[storage(105, read_only)]
    pub instances: Handle<ShaderStorageBuffer>,
}

impl MaterialExtension for VatMaterialExt {
    fn vertex_shader() -> ShaderRef {
        VAT_FORWARD_SHADER_HANDLE.into()
    }

    fn prepass_vertex_shader() -> ShaderRef {
        VAT_PREPASS_SHADER_HANDLE.into()
    }

    fn deferred_vertex_shader() -> ShaderRef {
        VAT_PREPASS_SHADER_HANDLE.into()
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        _descriptor: &mut RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        Ok(())
    }
}

#[derive(Resource, Clone, Debug)]
pub struct VatMaterialDefaults {
    pub flat_normal_texture: Handle<Image>,
}

impl FromWorld for VatMaterialDefaults {
    fn from_world(world: &mut World) -> Self {
        let mut images = world.resource_mut::<Assets<Image>>();
        let flat_normal_texture = images.add(configure_vat_data_image(Image::new_fill(
            bevy::render::render_resource::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            &[128, 128, 255, 255],
            bevy::render::render_resource::TextureFormat::Rgba8Unorm,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        )));

        Self {
            flat_normal_texture,
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum VatMaterialBuildError {
    #[error(
        "VAT metadata requests a separate normal texture, but no normal texture handle was provided"
    )]
    MissingSeparateNormalTexture,
}

impl VatMaterialExt {
    pub fn from_animation(
        animation: &VatAnimationData,
        position_texture: Handle<Image>,
        normal_texture: Option<Handle<Image>>,
        defaults: &VatMaterialDefaults,
        buffers: &mut Assets<ShaderStorageBuffer>,
    ) -> Result<Self, VatMaterialBuildError> {
        let (normal_texture_handle, normal_layout) = match &animation.normal_texture {
            VatNormalTexture::None => {
                (defaults.flat_normal_texture.clone(), UVec4::new(1, 1, 0, 0))
            }
            VatNormalTexture::PackedInPositionTexture { row_offset, .. } => (
                defaults.flat_normal_texture.clone(),
                UVec4::new(1, 1, 0, *row_offset),
            ),
            VatNormalTexture::Separate { texture, .. } => (
                normal_texture.ok_or(VatMaterialBuildError::MissingSeparateNormalTexture)?,
                UVec4::new(texture.width, texture.height, texture.rows_per_frame, 0),
            ),
        };

        let uniform = VatMaterialUniform {
            decode_min: animation.decode_bounds_min.extend(0.0),
            decode_extent: (animation.decode_bounds_max - animation.decode_bounds_min).extend(0.0),
            texture_layout: UVec4::new(
                animation.position_texture.width,
                animation.position_texture.height,
                animation.position_texture.rows_per_frame,
                animation.frame_count,
            ),
            normal_layout,
            modes: UVec4::new(
                match animation.position_encoding {
                    VatPositionEncoding::AbsoluteNormalizedBounds => 0,
                    VatPositionEncoding::OffsetNormalizedBounds => 1,
                },
                match animation.coordinate_system {
                    VatCoordinateSystem::YUpRightHanded => 0,
                    VatCoordinateSystem::ZUpRightHanded => 1,
                },
                match animation.playback_space {
                    VatPlaybackSpace::Local => 0,
                    VatPlaybackSpace::World => 1,
                },
                match animation.normal_texture {
                    VatNormalTexture::None => 0,
                    VatNormalTexture::PackedInPositionTexture { .. } => 1,
                    VatNormalTexture::Separate { .. } => 2,
                },
            ),
        };

        Ok(Self {
            position_texture,
            normal_texture: normal_texture_handle,
            uniform,
            instances: buffers.add(ShaderStorageBuffer::from(vec![VatGpuInstance::default()])),
        })
    }
}

pub fn build_vat_material(
    base: StandardMaterial,
    animation: &VatAnimationData,
    position_texture: Handle<Image>,
    normal_texture: Option<Handle<Image>>,
    defaults: &VatMaterialDefaults,
    buffers: &mut Assets<ShaderStorageBuffer>,
) -> Result<VatMaterial, VatMaterialBuildError> {
    Ok(VatMaterial {
        base,
        extension: VatMaterialExt::from_animation(
            animation,
            position_texture,
            normal_texture,
            defaults,
            buffers,
        )?,
    })
}

pub(crate) fn load_shaders(app: &mut App) {
    load_internal_asset!(
        app,
        VAT_FORWARD_SHADER_HANDLE,
        "../assets/shaders/vat.wgsl",
        Shader::from_wgsl
    );
    load_internal_asset!(
        app,
        VAT_PREPASS_SHADER_HANDLE,
        "../assets/shaders/vat_prepass.wgsl",
        Shader::from_wgsl
    );
}
