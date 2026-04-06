use bevy::prelude::*;

use crate::VatAnimationData;

#[derive(Bundle, Clone, Debug, Default)]
pub struct VatAnimationBundle {
    pub source: VatAnimationSource,
    pub playback: VatPlayback,
}

#[derive(Component, Reflect, Clone, Debug, PartialEq)]
#[reflect(Component, Default)]
pub struct VatAnimationSource {
    pub animation: Handle<VatAnimationData>,
    pub bounds_mode: VatBoundsMode,
}

impl VatAnimationSource {
    #[must_use]
    pub fn new(animation: Handle<VatAnimationData>) -> Self {
        Self {
            animation,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn with_bounds_mode(mut self, bounds_mode: VatBoundsMode) -> Self {
        self.bounds_mode = bounds_mode;
        self
    }
}

impl Default for VatAnimationSource {
    fn default() -> Self {
        Self {
            animation: Handle::default(),
            bounds_mode: VatBoundsMode::UseMetadataAabb,
        }
    }
}

#[derive(Component, Reflect, Clone, Debug, PartialEq)]
#[reflect(Component, Default)]
pub struct VatPlayback {
    pub time_seconds: f32,
    pub speed: f32,
    pub active_clip: Option<usize>,
    pub startup_clip: VatClipSelection,
    pub invalid_clip_fallback: VatInvalidClipFallback,
    pub loop_mode: VatLoopMode,
    pub playing: bool,
}

impl VatPlayback {
    #[must_use]
    pub fn with_clip(mut self, clip_index: usize) -> Self {
        self.active_clip = Some(clip_index);
        self.startup_clip = VatClipSelection::Index(clip_index);
        self
    }

    #[must_use]
    pub fn with_clip_index(self, clip_index: usize) -> Self {
        self.with_clip(clip_index)
    }

    #[must_use]
    pub fn with_clip_name(mut self, clip_name: impl Into<String>) -> Self {
        self.active_clip = None;
        self.startup_clip = VatClipSelection::Name(clip_name.into());
        self
    }

    #[must_use]
    pub fn with_metadata_default_clip(mut self) -> Self {
        self.active_clip = None;
        self.startup_clip = VatClipSelection::MetadataDefault;
        self
    }

    #[must_use]
    pub fn with_invalid_clip_fallback(
        mut self,
        invalid_clip_fallback: VatInvalidClipFallback,
    ) -> Self {
        self.invalid_clip_fallback = invalid_clip_fallback;
        self
    }

    #[must_use]
    pub fn with_loop_mode(mut self, loop_mode: VatLoopMode) -> Self {
        self.loop_mode = loop_mode;
        self
    }

    #[must_use]
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    #[must_use]
    pub fn with_time_seconds(mut self, time_seconds: f32) -> Self {
        self.time_seconds = time_seconds.max(0.0);
        self
    }

    #[must_use]
    pub fn paused(mut self) -> Self {
        self.playing = false;
        self
    }

    pub fn play_clip_named(
        &mut self,
        animation: &VatAnimationData,
        clip_name: &str,
    ) -> Result<usize, crate::VatClipResolveError> {
        let clip_index =
            animation.resolve_clip_selection(&VatClipSelection::Name(clip_name.to_owned()))?;
        self.active_clip = Some(clip_index);
        self.startup_clip = VatClipSelection::Name(clip_name.to_owned());
        Ok(clip_index)
    }

    pub fn play_clip_index(
        &mut self,
        animation: &VatAnimationData,
        clip_index: usize,
    ) -> Result<usize, crate::VatClipResolveError> {
        let clip_index = animation.resolve_clip_selection(&VatClipSelection::Index(clip_index))?;
        self.active_clip = Some(clip_index);
        self.startup_clip = VatClipSelection::Index(clip_index);
        Ok(clip_index)
    }

    #[must_use]
    pub fn active_clip_name<'a>(&self, animation: &'a VatAnimationData) -> Option<&'a str> {
        self.active_clip
            .and_then(|clip_index| animation.clip(clip_index))
            .map(|clip| clip.name.as_str())
    }
}

impl Default for VatPlayback {
    fn default() -> Self {
        Self {
            time_seconds: 0.0,
            speed: 1.0,
            active_clip: None,
            startup_clip: VatClipSelection::MetadataDefault,
            invalid_clip_fallback: VatInvalidClipFallback::StartupClipThenFirstValid,
            loop_mode: VatLoopMode::Loop,
            playing: true,
        }
    }
}

#[derive(Component, Reflect, Clone, Debug, PartialEq)]
#[reflect(Component)]
pub struct VatCrossfade {
    pub from_clip: usize,
    pub to_clip: usize,
    pub elapsed: f32,
    pub duration: f32,
}

impl VatCrossfade {
    #[must_use]
    pub fn new(from_clip: usize, to_clip: usize, duration: f32) -> Self {
        Self {
            from_clip,
            to_clip,
            elapsed: 0.0,
            duration: duration.max(0.0001),
        }
    }

    #[must_use]
    pub fn weight(&self) -> f32 {
        (self.elapsed / self.duration).clamp(0.0, 1.0)
    }

    pub fn between_clip_names(
        animation: &VatAnimationData,
        from_clip_name: &str,
        to_clip_name: &str,
        duration: f32,
    ) -> Result<Self, crate::VatClipResolveError> {
        let from_clip =
            animation.resolve_clip_selection(&VatClipSelection::Name(from_clip_name.to_owned()))?;
        let to_clip =
            animation.resolve_clip_selection(&VatClipSelection::Name(to_clip_name.to_owned()))?;
        Ok(Self::new(from_clip, to_clip, duration))
    }

    pub fn to_clip_name(
        animation: &VatAnimationData,
        playback: &VatPlayback,
        to_clip_name: &str,
        duration: f32,
    ) -> Result<Self, crate::VatClipResolveError> {
        let from_clip = playback
            .active_clip
            .ok_or(crate::VatClipResolveError::UnresolvedPlaybackClip)?;
        let to_clip =
            animation.resolve_clip_selection(&VatClipSelection::Name(to_clip_name.to_owned()))?;
        Ok(Self::new(from_clip, to_clip, duration))
    }
}

#[derive(Component, Reflect, Clone, Debug, Default, PartialEq)]
#[reflect(Component, Default)]
pub struct VatPlaybackTweaks {
    pub disable_interpolation: bool,
}

#[derive(Component, Reflect, Clone, Debug, PartialEq)]
#[reflect(Component)]
pub struct VatPlaybackFollower {
    pub leader: Entity,
    pub time_offset_seconds: f32,
    pub mirror_loop_mode: bool,
    pub mirror_crossfade: bool,
}

impl VatPlaybackFollower {
    #[must_use]
    pub fn new(leader: Entity) -> Self {
        Self {
            leader,
            time_offset_seconds: 0.0,
            mirror_loop_mode: true,
            mirror_crossfade: true,
        }
    }

    #[must_use]
    pub fn with_time_offset_seconds(mut self, time_offset_seconds: f32) -> Self {
        self.time_offset_seconds = time_offset_seconds;
        self
    }

    #[must_use]
    pub fn without_loop_mode_sync(mut self) -> Self {
        self.mirror_loop_mode = false;
        self
    }

    #[must_use]
    pub fn without_crossfade_sync(mut self) -> Self {
        self.mirror_crossfade = false;
        self
    }
}

#[derive(Clone, Copy, Debug, Reflect, Default, PartialEq, Eq, Hash)]
pub enum VatLoopMode {
    #[default]
    Loop,
    Once,
    PingPong,
    ClampForever,
}

#[derive(Clone, Debug, Reflect, Default, PartialEq, Eq, Hash)]
pub enum VatClipSelection {
    #[default]
    MetadataDefault,
    Index(usize),
    Name(String),
}

#[derive(Clone, Copy, Debug, Reflect, Default, PartialEq, Eq, Hash)]
pub enum VatInvalidClipFallback {
    #[default]
    StartupClipThenFirstValid,
    FirstValid,
    KeepCurrent,
}

#[derive(Clone, Copy, Debug, Reflect, Default, PartialEq, Eq, Hash)]
pub enum VatBoundsMode {
    #[default]
    UseMetadataAabb,
    KeepProxyAabb,
    DisableFrustumCulling,
}
