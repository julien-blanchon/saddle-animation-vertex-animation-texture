use bevy::prelude::*;
use saddle_bevy_e2e::action::Action;

use crate::{LabControl, LabDiagnostics};

pub fn request_clip_name(clip_name: &'static str) -> Action {
    Action::Custom(Box::new(move |world: &mut World| {
        let mut control = world.resource_mut::<LabControl>();
        control.auto = false;
        control.requested_clip_name = clip_name.to_owned();
        control.paused = false;
    }))
}

pub fn wait_for_clip_name(clip_name: &'static str, max_frames: u32) -> Action {
    Action::WaitUntil {
        label: format!("hero reached {clip_name} clip").into(),
        condition: Box::new(move |world: &World| {
            world
                .get_resource::<LabDiagnostics>()
                .is_some_and(|diagnostics| diagnostics.hero_clip_name == clip_name)
        }),
        max_frames,
    }
}

pub fn wait_for_crossfade_start(max_frames: u32) -> Action {
    Action::WaitUntil {
        label: "crossfade became active".into(),
        condition: Box::new(|world: &World| {
            world
                .get_resource::<LabDiagnostics>()
                .is_some_and(|diagnostics| diagnostics.crossfade_active)
        }),
        max_frames,
    }
}

pub fn wait_for_crossfade_finish(clip_name: &'static str, max_frames: u32) -> Action {
    Action::WaitUntil {
        label: format!("crossfade resolved to {clip_name}").into(),
        condition: Box::new(move |world: &World| {
            world
                .get_resource::<LabDiagnostics>()
                .is_some_and(|diagnostics| {
                    !diagnostics.crossfade_active && diagnostics.hero_clip_name == clip_name
                })
        }),
        max_frames,
    }
}

pub fn wait_for_paused_state(max_frames: u32) -> Action {
    Action::WaitUntil {
        label: "hero paused".into(),
        condition: Box::new(|world: &World| !world.resource::<LabDiagnostics>().hero_playing),
        max_frames,
    }
}

pub fn wait_for_resumed_state(max_frames: u32) -> Action {
    Action::WaitUntil {
        label: "hero resumed".into(),
        condition: Box::new(|world: &World| world.resource::<LabDiagnostics>().hero_playing),
        max_frames,
    }
}
