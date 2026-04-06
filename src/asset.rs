use bevy::prelude::*;
use thiserror::Error;

use crate::{VatClipSelection, VatLoopMode, validation::VatValidationError};

#[derive(Asset, Reflect, Clone, Debug, PartialEq)]
pub struct VatAnimationData {
    pub source_format: VatSourceFormat,
    pub animation_mode: VatAnimationMode,
    pub vertex_count: u32,
    pub frame_count: u32,
    pub frames_per_second: f32,
    pub decode_bounds_min: Vec3,
    pub decode_bounds_max: Vec3,
    pub animation_bounds_min: Vec3,
    pub animation_bounds_max: Vec3,
    pub default_clip: Option<String>,
    pub clips: Vec<VatClip>,
    pub position_texture: VatTextureDescriptor,
    pub normal_texture: VatNormalTexture,
    pub rotation_texture: Option<VatTextureDescriptor>,
    pub auxiliary_textures: Vec<VatAuxTextureDescriptor>,
    pub coordinate_system: VatCoordinateSystem,
    pub playback_space: VatPlaybackSpace,
    pub vertex_id_attribute: VatVertexIdAttribute,
    pub position_encoding: VatPositionEncoding,
}

impl VatAnimationData {
    pub fn validate(&self) -> Result<(), VatValidationError> {
        crate::validation::validate_animation_data(self)
    }

    #[must_use]
    pub fn clip(&self, clip_index: usize) -> Option<&VatClip> {
        self.clips.get(clip_index)
    }

    #[must_use]
    pub fn clip_index_by_name(&self, name: &str) -> Option<usize> {
        self.clips.iter().position(|clip| clip.name == name)
    }

    #[must_use]
    pub fn default_clip_index(&self) -> Option<usize> {
        self.default_clip
            .as_deref()
            .and_then(|clip_name| self.clip_index_by_name(clip_name))
    }

    pub fn resolve_clip_selection(
        &self,
        selection: &VatClipSelection,
    ) -> Result<usize, VatClipResolveError> {
        if self.clips.is_empty() {
            return Err(VatClipResolveError::NoClips);
        }

        match selection {
            VatClipSelection::MetadataDefault => {
                self.default_clip_index()
                    .ok_or_else(|| match self.default_clip.as_deref() {
                        Some(clip_name) => VatClipResolveError::UnknownClipName {
                            clip_name: clip_name.to_owned(),
                        },
                        None => VatClipResolveError::MissingMetadataDefaultClip,
                    })
            }
            VatClipSelection::Index(clip_index) => self
                .clip(*clip_index)
                .map(|_| *clip_index)
                .ok_or(VatClipResolveError::InvalidClipIndex {
                    clip_index: *clip_index,
                    clip_count: self.clips.len(),
                }),
            VatClipSelection::Name(clip_name) => {
                self.clip_index_by_name(clip_name).ok_or_else(|| {
                    VatClipResolveError::UnknownClipName {
                        clip_name: clip_name.clone(),
                    }
                })
            }
        }
    }

    #[must_use]
    pub fn clip_duration_seconds(&self, clip_index: usize) -> Option<f32> {
        self.clip(clip_index)
            .map(|clip| clip.frame_count() as f32 / self.frames_per_second)
    }

    #[must_use]
    pub fn position_capacity_per_frame(&self) -> u32 {
        self.position_texture.width * self.position_texture.rows_per_frame
    }

    #[must_use]
    pub fn uses_world_space(&self) -> bool {
        matches!(self.playback_space, VatPlaybackSpace::World)
    }

    #[must_use]
    pub fn supports_v1_runtime(&self) -> bool {
        matches!(self.animation_mode, VatAnimationMode::SoftBodyFixedTopology)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum VatClipResolveError {
    #[error("metadata does not declare a default clip")]
    MissingMetadataDefaultClip,
    #[error("unknown clip name '{clip_name}'")]
    UnknownClipName { clip_name: String },
    #[error("clip index {clip_index} is invalid for metadata with {clip_count} clips")]
    InvalidClipIndex {
        clip_index: usize,
        clip_count: usize,
    },
    #[error("playback has no resolved active clip yet")]
    UnresolvedPlaybackClip,
    #[error("metadata must define at least one clip")]
    NoClips,
}

#[derive(Reflect, Clone, Debug, PartialEq)]
pub struct VatClip {
    pub name: String,
    pub start_frame: u32,
    pub end_frame: u32,
    pub default_loop_mode: Option<VatLoopMode>,
    pub events: Vec<VatClipEvent>,
}

impl VatClip {
    #[must_use]
    pub fn frame_count(&self) -> u32 {
        self.end_frame - self.start_frame + 1
    }

    #[must_use]
    pub fn normalized_time_for_frame(&self, frame_in_clip: u32) -> f32 {
        if self.frame_count() <= 1 {
            0.0
        } else {
            frame_in_clip as f32 / (self.frame_count() - 1) as f32
        }
    }
}

#[derive(Reflect, Clone, Debug, PartialEq)]
pub struct VatClipEvent {
    pub name: String,
    pub frame: u32,
}

#[derive(Reflect, Clone, Debug, PartialEq)]
pub struct VatTextureDescriptor {
    pub relative_path: Option<String>,
    pub width: u32,
    pub height: u32,
    pub rows_per_frame: u32,
    pub precision: VatTexturePrecision,
}

#[derive(Reflect, Clone, Debug, PartialEq)]
pub enum VatNormalTexture {
    None,
    PackedInPositionTexture {
        row_offset: u32,
        encoding: VatNormalEncoding,
    },
    Separate {
        texture: VatTextureDescriptor,
        encoding: VatNormalEncoding,
    },
}

#[derive(Reflect, Clone, Debug, PartialEq)]
pub struct VatAuxTextureDescriptor {
    pub semantic: VatAuxTextureSemantic,
    pub texture: VatTextureDescriptor,
}

#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VatSourceFormat {
    #[default]
    Canonical,
    OpenVat,
}

#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VatAnimationMode {
    #[default]
    SoftBodyFixedTopology,
    RigidBody,
}

#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VatCoordinateSystem {
    #[default]
    YUpRightHanded,
    ZUpRightHanded,
}

#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VatPlaybackSpace {
    #[default]
    Local,
    World,
}

#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VatVertexIdAttribute {
    #[default]
    Uv1,
}

#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VatPositionEncoding {
    #[default]
    AbsoluteNormalizedBounds,
    OffsetNormalizedBounds,
}

#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VatNormalEncoding {
    #[default]
    SignedNormalized,
}

#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VatTexturePrecision {
    #[default]
    ExrHalf,
    Png16,
    Png8,
}

#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VatAuxTextureSemantic {
    Emission,
    Opacity,
    #[default]
    Scalar,
}
