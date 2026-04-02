use std::collections::HashSet;

use bevy::{
    asset::RenderAssetUsages,
    camera::primitives::Aabb,
    image::ImageSampler,
    mesh::{Mesh, VertexAttributeValues},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use thiserror::Error;

use crate::{
    VatAnimationData, VatAnimationMode, VatBoundsMode, VatCoordinateSystem, VatNormalTexture,
    VatPositionEncoding,
};

#[derive(Debug, Error, Clone, PartialEq)]
pub enum VatValidationError {
    #[error("frames_per_second must be greater than zero")]
    InvalidFramesPerSecond,
    #[error("frame_count must be greater than zero")]
    InvalidFrameCount,
    #[error("vertex_count must be greater than zero")]
    InvalidVertexCount,
    #[error("metadata must define at least one clip")]
    MissingClips,
    #[error("clip '{clip_name}' starts after it ends")]
    InvalidClipRange { clip_name: String },
    #[error("clip '{clip_name}' is outside the baked frame range")]
    ClipOutOfBounds { clip_name: String },
    #[error("clip '{clip_name}' contains event '{event_name}' outside the clip frame range")]
    EventOutOfBounds {
        clip_name: String,
        event_name: String,
    },
    #[error("position texture layout cannot store {vertex_count} vertices in {capacity} texels per frame")]
    PositionTextureTooSmall { vertex_count: u32, capacity: u32 },
    #[error(
        "position texture height/rows_per_frame do not cover all baked frames (height={height}, rows_per_frame={rows_per_frame}, frame_count={frame_count})"
    )]
    InvalidPositionTextureLayout {
        height: u32,
        rows_per_frame: u32,
        frame_count: u32,
    },
    #[error("unsupported coordinate system '{0}'")]
    UnsupportedCoordinateSystem(String),
    #[error("decode bounds are invalid or non-finite")]
    InvalidDecodeBounds,
    #[error("animation bounds are invalid or non-finite")]
    InvalidAnimationBounds,
    #[error("separate normal texture height/rows_per_frame do not match the frame count")]
    InvalidNormalTextureLayout,
    #[error("packed normals require a non-zero row_offset")]
    InvalidPackedNormalRowOffset,
    #[error("rigid-body VAT metadata is present, but the v0.1 runtime only supports fixed-topology soft-body VAT")]
    UnsupportedAnimationMode,
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum VatMeshValidationError {
    #[error("VAT playback requires Mesh::ATTRIBUTE_POSITION")]
    MissingPosition,
    #[error("VAT playback requires Mesh::ATTRIBUTE_NORMAL")]
    MissingNormal,
    #[error("VAT playback requires Mesh::ATTRIBUTE_UV_0 for StandardMaterial compatibility")]
    MissingUv0,
    #[error("VAT playback requires Mesh::ATTRIBUTE_UV_1 (the DCC UV2 channel) for vertex lookup")]
    MissingUv1,
    #[error("mesh vertex count {mesh_vertex_count} does not match metadata vertex count {metadata_vertex_count}")]
    VertexCountMismatch {
        mesh_vertex_count: usize,
        metadata_vertex_count: u32,
    },
    #[error("mesh UV1 channel is malformed or not vec2 data")]
    InvalidUv1Format,
    #[error("mesh UV1 channel does not map to {metadata_vertex_count} unique VAT texels")]
    VertexIdMismatch { metadata_vertex_count: u32 },
}

pub fn validate_animation_data(animation: &VatAnimationData) -> Result<(), VatValidationError> {
    if !animation.supports_v1_runtime() {
        return Err(VatValidationError::UnsupportedAnimationMode);
    }
    if animation.frames_per_second <= 0.0 {
        return Err(VatValidationError::InvalidFramesPerSecond);
    }
    if animation.frame_count == 0 {
        return Err(VatValidationError::InvalidFrameCount);
    }
    if animation.vertex_count == 0 {
        return Err(VatValidationError::InvalidVertexCount);
    }
    if animation.clips.is_empty() {
        return Err(VatValidationError::MissingClips);
    }
    if !valid_bounds(animation.decode_bounds_min, animation.decode_bounds_max) {
        return Err(VatValidationError::InvalidDecodeBounds);
    }
    if !valid_bounds(animation.animation_bounds_min, animation.animation_bounds_max) {
        return Err(VatValidationError::InvalidAnimationBounds);
    }
    if animation.position_capacity_per_frame() < animation.vertex_count {
        return Err(VatValidationError::PositionTextureTooSmall {
            vertex_count: animation.vertex_count,
            capacity: animation.position_capacity_per_frame(),
        });
    }
    if animation.position_texture.rows_per_frame == 0
        || animation.position_texture.width == 0
        || animation.position_texture.height
            < animation.position_texture.rows_per_frame * animation.frame_count
    {
        return Err(VatValidationError::InvalidPositionTextureLayout {
            height: animation.position_texture.height,
            rows_per_frame: animation.position_texture.rows_per_frame,
            frame_count: animation.frame_count,
        });
    }

    for clip in &animation.clips {
        if clip.start_frame > clip.end_frame {
            return Err(VatValidationError::InvalidClipRange {
                clip_name: clip.name.clone(),
            });
        }
        if clip.end_frame >= animation.frame_count {
            return Err(VatValidationError::ClipOutOfBounds {
                clip_name: clip.name.clone(),
            });
        }
        for event in &clip.events {
            if event.frame >= clip.frame_count() {
                return Err(VatValidationError::EventOutOfBounds {
                    clip_name: clip.name.clone(),
                    event_name: event.name.clone(),
                });
            }
        }
    }

    match &animation.normal_texture {
        VatNormalTexture::None => {}
        VatNormalTexture::PackedInPositionTexture { row_offset, .. } => {
            if *row_offset == 0 {
                return Err(VatValidationError::InvalidPackedNormalRowOffset);
            }
        }
        VatNormalTexture::Separate { texture, .. } => {
            if texture.rows_per_frame == 0
                || texture.width == 0
                || texture.height < texture.rows_per_frame * animation.frame_count
            {
                return Err(VatValidationError::InvalidNormalTextureLayout);
            }
        }
    }

    Ok(())
}

pub fn validate_mesh_for_animation(
    mesh: &Mesh,
    animation: &VatAnimationData,
) -> Result<(), VatMeshValidationError> {
    if !mesh.contains_attribute(Mesh::ATTRIBUTE_POSITION) {
        return Err(VatMeshValidationError::MissingPosition);
    }
    if !mesh.contains_attribute(Mesh::ATTRIBUTE_NORMAL) {
        return Err(VatMeshValidationError::MissingNormal);
    }
    if !mesh.contains_attribute(Mesh::ATTRIBUTE_UV_0) {
        return Err(VatMeshValidationError::MissingUv0);
    }
    if !mesh.contains_attribute(Mesh::ATTRIBUTE_UV_1) {
        return Err(VatMeshValidationError::MissingUv1);
    }

    let vertex_count = mesh.count_vertices();
    if vertex_count != animation.vertex_count as usize {
        return Err(VatMeshValidationError::VertexCountMismatch {
            mesh_vertex_count: vertex_count,
            metadata_vertex_count: animation.vertex_count,
        });
    }

    let uv1 = mesh.attribute(Mesh::ATTRIBUTE_UV_1);
    let Some(VertexAttributeValues::Float32x2(values)) = uv1 else {
        return Err(VatMeshValidationError::InvalidUv1Format);
    };

    let width = animation.position_texture.width as f32;
    let height = animation.position_texture.height as f32;
    let mut unique_texels = HashSet::with_capacity(values.len());
    for uv in values {
        let texel_x = ((uv[0] * width) - 0.5).round() as i32;
        let texel_y = ((uv[1] * height) - 0.5).round() as i32;
        unique_texels.insert((texel_x, texel_y));
    }

    if unique_texels.len() != animation.vertex_count as usize {
        return Err(VatMeshValidationError::VertexIdMismatch {
            metadata_vertex_count: animation.vertex_count,
        });
    }

    Ok(())
}

#[must_use]
pub fn metadata_aabb(animation: &VatAnimationData) -> Aabb {
    let min = convert_coordinate_system(animation.animation_bounds_min, animation.coordinate_system);
    let max = convert_coordinate_system(animation.animation_bounds_max, animation.coordinate_system);
    Aabb::from_min_max(min, max)
}

#[must_use]
pub fn convert_coordinate_system(value: Vec3, coordinate_system: VatCoordinateSystem) -> Vec3 {
    match coordinate_system {
        VatCoordinateSystem::YUpRightHanded => value,
        VatCoordinateSystem::ZUpRightHanded => Vec3::new(value.x, value.z, -value.y),
    }
}

#[must_use]
pub fn configure_vat_data_image(mut image: Image) -> Image {
    image.sampler = ImageSampler::nearest();
    image
}

#[must_use]
pub fn make_linear_rgba8_image(size: UVec2, data: Vec<u8>) -> Image {
    configure_vat_data_image(Image::new(
        Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    ))
}

#[must_use]
pub fn decode_position_sample(
    encoded: Vec3,
    animation: &VatAnimationData,
    proxy_position: Vec3,
) -> Vec3 {
    let decoded_source =
        animation.decode_bounds_min + encoded * (animation.decode_bounds_max - animation.decode_bounds_min);
    let decoded = convert_coordinate_system(decoded_source, animation.coordinate_system);
    match animation.position_encoding {
        VatPositionEncoding::AbsoluteNormalizedBounds => decoded,
        VatPositionEncoding::OffsetNormalizedBounds => proxy_position + decoded,
    }
}

#[must_use]
pub fn valid_bounds(min: Vec3, max: Vec3) -> bool {
    min.is_finite() && max.is_finite() && min.cmple(max).all()
}

#[must_use]
pub fn should_disable_frustum_culling(
    animation: &VatAnimationData,
    bounds_mode: VatBoundsMode,
) -> bool {
    matches!(bounds_mode, VatBoundsMode::DisableFrustumCulling)
        || matches!(animation.playback_space, crate::VatPlaybackSpace::World)
        || matches!(animation.animation_mode, VatAnimationMode::RigidBody)
}
