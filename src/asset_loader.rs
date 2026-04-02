use std::collections::BTreeMap;

use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::*,
    reflect::TypePath,
    tasks::ConditionalSendFuture,
};
use serde::Deserialize;
use thiserror::Error;

use crate::{
    VatAnimationData, VatAnimationMode, VatAuxTextureDescriptor, VatAuxTextureSemantic, VatClip,
    VatClipEvent, VatNormalEncoding, VatNormalTexture, VatPositionEncoding, VatSourceFormat,
    VatTextureDescriptor, VatTexturePrecision, VatValidationError, VatVertexIdAttribute,
};

#[derive(Default, TypePath)]
pub struct VatAnimationDataLoader;

#[derive(Debug, Error)]
pub enum VatMetadataLoadError {
    #[error("failed to read VAT metadata: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to decode VAT metadata JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid VAT metadata: {0}")]
    Validation(#[from] VatValidationError),
    #[error("unsupported VAT metadata format: {0}")]
    UnsupportedFormat(String),
}

impl AssetLoader for VatAnimationDataLoader {
    type Asset = VatAnimationData;
    type Settings = ();
    type Error = VatMetadataLoadError;

    fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext,
    ) -> impl ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            parse_vat_animation_data_bytes(&bytes)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["vatanim.json", "vat.json"]
    }
}

pub fn parse_vat_animation_data_bytes(
    bytes: &[u8],
) -> Result<VatAnimationData, VatMetadataLoadError> {
    let value = serde_json::from_slice::<serde_json::Value>(bytes)?;
    parse_vat_animation_data_value(value)
}

pub fn parse_vat_animation_data_str(text: &str) -> Result<VatAnimationData, VatMetadataLoadError> {
    parse_vat_animation_data_bytes(text.as_bytes())
}

fn parse_vat_animation_data_value(
    value: serde_json::Value,
) -> Result<VatAnimationData, VatMetadataLoadError> {
    if value.get("os-remap").is_some() {
        let raw = serde_json::from_value::<RawOpenVatMetadata>(value)?;
        let animation = raw.into_asset()?;
        animation.validate()?;
        return Ok(animation);
    }

    if value.get("vertex_count").is_some() || value.get("format").is_some() {
        let raw = serde_json::from_value::<RawCanonicalVatMetadata>(value)?;
        let animation = raw.into_asset()?;
        animation.validate()?;
        return Ok(animation);
    }

    Err(VatMetadataLoadError::UnsupportedFormat(
        "expected canonical VAT metadata or OpenVAT-compatible remap metadata".into(),
    ))
}

#[derive(Deserialize)]
struct RawCanonicalVatMetadata {
    format: Option<String>,
    animation_mode: Option<String>,
    vertex_count: u32,
    frame_count: u32,
    frames_per_second: Option<f32>,
    seconds_per_frame: Option<f32>,
    decode_bounds: RawBounds,
    animation_bounds: Option<RawBounds>,
    clips: Vec<RawClip>,
    position_texture: RawTexture,
    #[serde(default)]
    normal_texture: Option<RawNormalTexture>,
    #[serde(default)]
    rotation_texture: Option<RawTexture>,
    #[serde(default)]
    auxiliary_textures: Vec<RawAuxTexture>,
    coordinate_system: Option<String>,
    playback_space: Option<String>,
    vertex_id_attribute: Option<String>,
    position_encoding: Option<String>,
}

impl RawCanonicalVatMetadata {
    fn into_asset(self) -> Result<VatAnimationData, VatMetadataLoadError> {
        if let Some(format) = self.format.as_deref() {
            match format {
                "vertex_animation_texture@1" | "vertex_animation_texture" => {}
                other => {
                    return Err(VatMetadataLoadError::UnsupportedFormat(format!(
                        "unsupported canonical metadata format '{other}'"
                    )));
                }
            }
        }

        let frames_per_second = match (self.frames_per_second, self.seconds_per_frame) {
            (Some(fps), _) if fps > 0.0 => fps,
            (_, Some(spf)) if spf > 0.0 => 1.0 / spf,
            _ => {
                return Err(VatMetadataLoadError::Validation(
                    VatValidationError::InvalidFramesPerSecond,
                ));
            }
        };

        let animation_mode = match self.animation_mode.as_deref() {
            None | Some("soft_body_fixed_topology") | Some("soft_body") => {
                VatAnimationMode::SoftBodyFixedTopology
            }
            Some("rigid_body") => VatAnimationMode::RigidBody,
            Some(other) => {
                return Err(VatMetadataLoadError::UnsupportedFormat(format!(
                    "unsupported animation_mode '{other}'"
                )));
            }
        };

        let coordinate_system = parse_coordinate_system(self.coordinate_system.as_deref())?;
        let playback_space = parse_playback_space(self.playback_space.as_deref())?;
        let position_encoding = parse_position_encoding(self.position_encoding.as_deref())?;
        let vertex_id_attribute = parse_vertex_id_attribute(self.vertex_id_attribute.as_deref())?;

        let decode_bounds = self.decode_bounds.into_bounds();
        let animation_bounds = self
            .animation_bounds
            .unwrap_or(self.decode_bounds)
            .into_bounds();

        Ok(VatAnimationData {
            source_format: VatSourceFormat::Canonical,
            animation_mode,
            vertex_count: self.vertex_count,
            frame_count: self.frame_count,
            frames_per_second,
            decode_bounds_min: decode_bounds.0,
            decode_bounds_max: decode_bounds.1,
            animation_bounds_min: animation_bounds.0,
            animation_bounds_max: animation_bounds.1,
            clips: self
                .clips
                .into_iter()
                .map(RawClip::into_clip)
                .collect::<Result<Vec<_>, _>>()?,
            position_texture: self.position_texture.into_texture()?,
            normal_texture: self
                .normal_texture
                .map(RawNormalTexture::into_texture)
                .transpose()?
                .unwrap_or(VatNormalTexture::None),
            rotation_texture: self
                .rotation_texture
                .map(RawTexture::into_texture)
                .transpose()?,
            auxiliary_textures: self
                .auxiliary_textures
                .into_iter()
                .map(RawAuxTexture::into_aux_texture)
                .collect::<Result<Vec<_>, _>>()?,
            coordinate_system,
            playback_space,
            vertex_id_attribute,
            position_encoding,
        })
    }
}

#[derive(Deserialize)]
struct RawOpenVatMetadata {
    #[serde(rename = "os-remap")]
    os_remap: RawOpenVatRemap,
    #[serde(default)]
    animations: BTreeMap<String, RawOpenVatClip>,
    vertex_count: Option<u32>,
    texture_width: Option<u32>,
    texture_height: Option<u32>,
    rows_per_frame: Option<u32>,
    #[serde(default)]
    packed_normals: bool,
    normal_row_offset: Option<u32>,
    coordinate_system: Option<String>,
    playback_space: Option<String>,
}

impl RawOpenVatMetadata {
    fn into_asset(self) -> Result<VatAnimationData, VatMetadataLoadError> {
        let vertex_count = self.vertex_count.ok_or_else(|| {
            VatMetadataLoadError::UnsupportedFormat(
                "OpenVAT-compatible metadata requires 'vertex_count'".into(),
            )
        })?;
        let texture_width = self.texture_width.ok_or_else(|| {
            VatMetadataLoadError::UnsupportedFormat(
                "OpenVAT-compatible metadata requires 'texture_width'".into(),
            )
        })?;
        let texture_height = self.texture_height.ok_or_else(|| {
            VatMetadataLoadError::UnsupportedFormat(
                "OpenVAT-compatible metadata requires 'texture_height'".into(),
            )
        })?;
        let rows_per_frame = self.rows_per_frame.unwrap_or(1);
        let clips = if self.animations.is_empty() {
            vec![VatClip {
                name: "default".into(),
                start_frame: 0,
                end_frame: self.os_remap.frames.saturating_sub(1),
                default_loop_mode: Some(crate::VatLoopMode::Loop),
                events: Vec::new(),
            }]
        } else {
            self.animations
                .into_iter()
                .map(|(name, clip)| VatClip {
                    name,
                    start_frame: clip.start_frame,
                    end_frame: clip.end_frame,
                    default_loop_mode: Some(if clip.looping {
                        crate::VatLoopMode::Loop
                    } else {
                        crate::VatLoopMode::Once
                    }),
                    events: Vec::new(),
                })
                .collect()
        };

        let normal_texture = if self.packed_normals {
            VatNormalTexture::PackedInPositionTexture {
                row_offset: self
                    .normal_row_offset
                    .unwrap_or(texture_height.saturating_div(2)),
                encoding: VatNormalEncoding::SignedNormalized,
            }
        } else {
            VatNormalTexture::None
        };

        Ok(VatAnimationData {
            source_format: VatSourceFormat::OpenVat,
            animation_mode: VatAnimationMode::SoftBodyFixedTopology,
            vertex_count,
            frame_count: self.os_remap.frames,
            frames_per_second: 24.0,
            decode_bounds_min: Vec3::from_array(self.os_remap.min),
            decode_bounds_max: Vec3::from_array(self.os_remap.max),
            animation_bounds_min: Vec3::from_array(self.os_remap.min),
            animation_bounds_max: Vec3::from_array(self.os_remap.max),
            clips,
            position_texture: VatTextureDescriptor {
                relative_path: None,
                width: texture_width,
                height: texture_height,
                rows_per_frame,
                precision: VatTexturePrecision::Png16,
            },
            normal_texture,
            rotation_texture: None,
            auxiliary_textures: Vec::new(),
            coordinate_system: parse_coordinate_system(self.coordinate_system.as_deref())?,
            playback_space: parse_playback_space(self.playback_space.as_deref())?,
            vertex_id_attribute: VatVertexIdAttribute::Uv1,
            position_encoding: VatPositionEncoding::AbsoluteNormalizedBounds,
        })
    }
}

#[derive(Clone, Copy, Deserialize)]
struct RawBounds {
    min: [f32; 3],
    max: [f32; 3],
}

impl RawBounds {
    fn into_bounds(self) -> (Vec3, Vec3) {
        (Vec3::from_array(self.min), Vec3::from_array(self.max))
    }
}

#[derive(Deserialize)]
struct RawClip {
    name: String,
    start_frame: u32,
    end_frame: u32,
    #[serde(default)]
    default_loop_mode: Option<String>,
    #[serde(default)]
    events: Vec<RawClipEvent>,
}

impl RawClip {
    fn into_clip(self) -> Result<VatClip, VatMetadataLoadError> {
        Ok(VatClip {
            name: self.name,
            start_frame: self.start_frame,
            end_frame: self.end_frame,
            default_loop_mode: match self.default_loop_mode.as_deref() {
                Some(value) => Some(parse_loop_mode(value)?),
                None => None,
            },
            events: self
                .events
                .into_iter()
                .map(|event| VatClipEvent {
                    name: event.name,
                    frame: event.frame,
                })
                .collect(),
        })
    }
}

#[derive(Deserialize)]
struct RawClipEvent {
    name: String,
    frame: u32,
}

#[derive(Deserialize)]
struct RawTexture {
    relative_path: Option<String>,
    width: u32,
    height: u32,
    rows_per_frame: Option<u32>,
    precision: Option<String>,
}

impl RawTexture {
    fn into_texture(self) -> Result<VatTextureDescriptor, VatMetadataLoadError> {
        Ok(VatTextureDescriptor {
            relative_path: self.relative_path,
            width: self.width,
            height: self.height,
            rows_per_frame: self.rows_per_frame.unwrap_or(1),
            precision: parse_precision(self.precision.as_deref())?,
        })
    }
}

#[derive(Deserialize)]
struct RawNormalTexture {
    mode: String,
    row_offset: Option<u32>,
    texture: Option<RawTexture>,
    encoding: Option<String>,
}

impl RawNormalTexture {
    fn into_texture(self) -> Result<VatNormalTexture, VatMetadataLoadError> {
        let encoding = parse_normal_encoding(self.encoding.as_deref())?;
        match self.mode.as_str() {
            "none" => Ok(VatNormalTexture::None),
            "packed_in_position_texture" | "packed" => {
                Ok(VatNormalTexture::PackedInPositionTexture {
                    row_offset: self.row_offset.unwrap_or(0),
                    encoding,
                })
            }
            "separate" | "separate_texture" => Ok(VatNormalTexture::Separate {
                texture: self
                    .texture
                    .ok_or_else(|| {
                        VatMetadataLoadError::UnsupportedFormat(
                            "normal_texture.mode='separate' requires a texture descriptor".into(),
                        )
                    })?
                    .into_texture()?,
                encoding,
            }),
            other => Err(VatMetadataLoadError::UnsupportedFormat(format!(
                "unsupported normal_texture.mode '{other}'"
            ))),
        }
    }
}

#[derive(Deserialize)]
struct RawAuxTexture {
    semantic: Option<String>,
    texture: RawTexture,
}

impl RawAuxTexture {
    fn into_aux_texture(self) -> Result<VatAuxTextureDescriptor, VatMetadataLoadError> {
        Ok(VatAuxTextureDescriptor {
            semantic: match self.semantic.as_deref() {
                Some("emission") => VatAuxTextureSemantic::Emission,
                Some("opacity") => VatAuxTextureSemantic::Opacity,
                Some("scalar") | None => VatAuxTextureSemantic::Scalar,
                Some(other) => {
                    return Err(VatMetadataLoadError::UnsupportedFormat(format!(
                        "unsupported auxiliary_textures.semantic '{other}'"
                    )));
                }
            },
            texture: self.texture.into_texture()?,
        })
    }
}

#[derive(Deserialize)]
struct RawOpenVatRemap {
    #[serde(rename = "Min")]
    min: [f32; 3],
    #[serde(rename = "Max")]
    max: [f32; 3],
    #[serde(rename = "Frames")]
    frames: u32,
}

#[derive(Deserialize)]
struct RawOpenVatClip {
    #[serde(rename = "startFrame")]
    #[serde(alias = "start_frame")]
    start_frame: u32,
    #[serde(rename = "endFrame")]
    #[serde(alias = "end_frame")]
    end_frame: u32,
    #[serde(rename = "framerate")]
    #[allow(dead_code)]
    frame_rate: Option<f32>,
    #[serde(default)]
    looping: bool,
}

fn parse_loop_mode(value: &str) -> Result<crate::VatLoopMode, VatMetadataLoadError> {
    match value {
        "loop" => Ok(crate::VatLoopMode::Loop),
        "once" => Ok(crate::VatLoopMode::Once),
        "ping_pong" | "pingpong" => Ok(crate::VatLoopMode::PingPong),
        "clamp_forever" | "clamp" => Ok(crate::VatLoopMode::ClampForever),
        other => Err(VatMetadataLoadError::UnsupportedFormat(format!(
            "unsupported clip default_loop_mode '{other}'"
        ))),
    }
}

fn parse_coordinate_system(
    value: Option<&str>,
) -> Result<crate::VatCoordinateSystem, VatMetadataLoadError> {
    match value.unwrap_or("y_up_right_handed") {
        "y_up_right_handed" | "bevy" => Ok(crate::VatCoordinateSystem::YUpRightHanded),
        "z_up_right_handed" | "openvat_blender" | "blender" => {
            Ok(crate::VatCoordinateSystem::ZUpRightHanded)
        }
        other => Err(VatMetadataLoadError::Validation(
            VatValidationError::UnsupportedCoordinateSystem(other.into()),
        )),
    }
}

fn parse_playback_space(
    value: Option<&str>,
) -> Result<crate::VatPlaybackSpace, VatMetadataLoadError> {
    match value.unwrap_or("local") {
        "local" => Ok(crate::VatPlaybackSpace::Local),
        "world" => Ok(crate::VatPlaybackSpace::World),
        other => Err(VatMetadataLoadError::UnsupportedFormat(format!(
            "unsupported playback_space '{other}'"
        ))),
    }
}

fn parse_position_encoding(
    value: Option<&str>,
) -> Result<crate::VatPositionEncoding, VatMetadataLoadError> {
    match value.unwrap_or("absolute_normalized_bounds") {
        "absolute_normalized_bounds" | "absolute" => {
            Ok(crate::VatPositionEncoding::AbsoluteNormalizedBounds)
        }
        "offset_normalized_bounds" | "offset" => {
            Ok(crate::VatPositionEncoding::OffsetNormalizedBounds)
        }
        other => Err(VatMetadataLoadError::UnsupportedFormat(format!(
            "unsupported position_encoding '{other}'"
        ))),
    }
}

fn parse_vertex_id_attribute(
    value: Option<&str>,
) -> Result<crate::VatVertexIdAttribute, VatMetadataLoadError> {
    match value.unwrap_or("uv1") {
        "uv1" | "uv2" => Ok(crate::VatVertexIdAttribute::Uv1),
        other => Err(VatMetadataLoadError::UnsupportedFormat(format!(
            "unsupported vertex_id_attribute '{other}'"
        ))),
    }
}

fn parse_normal_encoding(
    value: Option<&str>,
) -> Result<crate::VatNormalEncoding, VatMetadataLoadError> {
    match value.unwrap_or("signed_normalized") {
        "signed_normalized" | "snorm" => Ok(crate::VatNormalEncoding::SignedNormalized),
        other => Err(VatMetadataLoadError::UnsupportedFormat(format!(
            "unsupported normal_texture.encoding '{other}'"
        ))),
    }
}

fn parse_precision(
    value: Option<&str>,
) -> Result<crate::VatTexturePrecision, VatMetadataLoadError> {
    match value.unwrap_or("exr_half") {
        "exr_half" => Ok(crate::VatTexturePrecision::ExrHalf),
        "png16" => Ok(crate::VatTexturePrecision::Png16),
        "png8" => Ok(crate::VatTexturePrecision::Png8),
        other => Err(VatMetadataLoadError::UnsupportedFormat(format!(
            "unsupported texture precision '{other}'"
        ))),
    }
}

#[cfg(test)]
#[path = "asset_loader_tests.rs"]
mod tests;
