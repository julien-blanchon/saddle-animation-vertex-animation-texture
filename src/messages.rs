use bevy::prelude::*;

#[derive(Message, Reflect, Clone, Debug, PartialEq)]
pub struct VatClipFinished {
    pub entity: Entity,
    pub clip_index: usize,
    pub clip_name: String,
    pub finished_at_seconds: f32,
}

#[derive(Message, Reflect, Clone, Debug, PartialEq)]
pub struct VatEventReached {
    pub entity: Entity,
    pub clip_index: usize,
    pub clip_name: String,
    pub event_name: String,
    pub clip_frame: u32,
    pub normalized_time: f32,
    pub reached_at_seconds: f32,
}
