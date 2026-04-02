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
    pub active_clip: usize,
    pub loop_mode: VatLoopMode,
    pub playing: bool,
}

impl VatPlayback {
    #[must_use]
    pub fn with_clip(mut self, clip_index: usize) -> Self {
        self.active_clip = clip_index;
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
}

impl Default for VatPlayback {
    fn default() -> Self {
        Self {
            time_seconds: 0.0,
            speed: 1.0,
            active_clip: 0,
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
}

#[derive(Component, Reflect, Clone, Debug, Default, PartialEq)]
#[reflect(Component, Default)]
pub struct VatPlaybackTweaks {
    pub disable_interpolation: bool,
}

#[derive(Clone, Copy, Debug, Reflect, Default, PartialEq, Eq, Hash)]
pub enum VatLoopMode {
    #[default]
    Loop,
    Once,
    PingPong,
    ClampForever,
}

#[derive(Clone, Copy, Debug, Reflect, Default, PartialEq, Eq, Hash)]
pub enum VatBoundsMode {
    #[default]
    UseMetadataAabb,
    KeepProxyAabb,
    DisableFrustumCulling,
}
