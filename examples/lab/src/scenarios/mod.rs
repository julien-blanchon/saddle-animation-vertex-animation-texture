use bevy::prelude::*;
use bevy_e2e::{action::Action, actions::assertions, scenario::Scenario};

use crate::{BoundsProbe, CrowdMember, Hero, LabControl, LabDiagnostics};

pub fn list_scenarios() -> Vec<&'static str> {
    vec![
        "vat_smoke",
        "vat_multi_clip",
        "vat_crowd",
        "vat_bounds_regression",
        "vat_crossfade",
    ]
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "vat_smoke" => Some(build_smoke()),
        "vat_multi_clip" => Some(build_multi_clip()),
        "vat_crowd" => Some(build_crowd()),
        "vat_bounds_regression" => Some(build_bounds_regression()),
        "vat_crossfade" => Some(build_crossfade()),
        _ => None,
    }
}

fn request_clip(clip: usize) -> Action {
    Action::Custom(Box::new(move |world: &mut World| {
        let mut control = world.resource_mut::<LabControl>();
        control.auto = false;
        control.requested_clip = clip;
        control.paused = false;
    }))
}

fn build_smoke() -> Scenario {
    Scenario::builder("vat_smoke")
        .description(
            "Verify the hero animates, crowd phases diverge, and capture two checkpoints so the motion is visibly non-static.",
        )
        .then(Action::WaitFrames(40))
        .then(assertions::entity_exists::<Hero>("hero exists"))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "hero and crowd diagnostics are live",
            |diagnostics| {
                diagnostics.hero_time > 0.05
                    && diagnostics.hero_playing
                    && diagnostics.crowd_phase_span > 0.2
            },
        ))
        .then(Action::Screenshot("vat_smoke_start".into()))
        .then(Action::WaitFrames(24))
        .then(Action::Screenshot("vat_smoke_late".into()))
        .then(assertions::log_summary("vat_smoke summary"))
        .build()
}

fn build_multi_clip() -> Scenario {
    Scenario::builder("vat_multi_clip")
        .description(
            "Force the one-shot burst clip, assert the hero changes clips and emits at least one event, then capture the burst and the return.",
        )
        .then(Action::WaitFrames(20))
        .then(request_clip(2))
        .then(Action::WaitUntil {
            label: "hero entered burst clip".into(),
            condition: Box::new(|world: &World| {
                world
                    .get_resource::<LabDiagnostics>()
                    .is_some_and(|diagnostics| diagnostics.hero_clip == 2)
            }),
            max_frames: 90,
        })
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "hero switched to burst clip",
            |diagnostics| diagnostics.hero_clip == 2,
        ))
        .then(Action::Screenshot("vat_multi_clip_burst".into()))
        .then(Action::WaitUntil {
            label: "burst emitted an event".into(),
            condition: Box::new(|world: &World| {
                world
                    .get_resource::<LabDiagnostics>()
                    .is_some_and(|diagnostics| diagnostics.event_count >= 1)
            }),
            max_frames: 120,
        })
        .then(request_clip(0))
        .then(Action::WaitFrames(20))
        .then(Action::Screenshot("vat_multi_clip_idle".into()))
        .then(assertions::log_summary("vat_multi_clip summary"))
        .build()
}

fn build_crowd() -> Scenario {
    Scenario::builder("vat_crowd")
        .description(
            "Exercise the shared-material crowd path, assert the expected population count and desynchronised phases, then capture two wide shots.",
        )
        .then(Action::WaitFrames(35))
        .then(assertions::entity_count_range::<CrowdMember>(
            "crowd population",
            20,
            30,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "crowd phase offsets diverged",
            |diagnostics| diagnostics.crowd_phase_span > 0.2,
        ))
        .then(Action::Screenshot("vat_crowd_start".into()))
        .then(Action::WaitFrames(18))
        .then(Action::Screenshot("vat_crowd_late".into()))
        .then(assertions::log_summary("vat_crowd summary"))
        .build()
}

fn build_bounds_regression() -> Scenario {
    Scenario::builder("vat_bounds_regression")
        .description(
            "Keep the bounds probe near the edge of the camera framing, assert it remains visible through the gust motion, and capture both checkpoints.",
        )
        .then(Action::WaitFrames(30))
        .then(assertions::entity_exists::<BoundsProbe>("bounds probe exists"))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "bounds probe is visible",
            |diagnostics| diagnostics.bounds_probe_visible,
        ))
        .then(Action::Screenshot("vat_bounds_regression_start".into()))
        .then(Action::WaitFrames(32))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "bounds probe stayed visible",
            |diagnostics| diagnostics.bounds_probe_visible,
        ))
        .then(Action::Screenshot("vat_bounds_regression_late".into()))
        .then(assertions::log_summary("vat_bounds_regression summary"))
        .build()
}

fn build_crossfade() -> Scenario {
    Scenario::builder("vat_crossfade")
        .description(
            "Trigger a hero crossfade, assert the transition becomes active and then resolves, and capture entry and mid-transition screenshots.",
        )
        .then(Action::WaitFrames(20))
        .then(request_clip(1))
        .then(Action::WaitUntil {
            label: "crossfade became active".into(),
            condition: Box::new(|world: &World| {
                world
                    .get_resource::<LabDiagnostics>()
                    .is_some_and(|diagnostics| diagnostics.crossfade_active)
            }),
            max_frames: 60,
        })
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "crossfade is active",
            |diagnostics| diagnostics.crossfade_active,
        ))
        .then(Action::Screenshot("vat_crossfade_start".into()))
        .then(Action::WaitFrames(8))
        .then(Action::Screenshot("vat_crossfade_mid".into()))
        .then(Action::WaitUntil {
            label: "crossfade resolved".into(),
            condition: Box::new(|world: &World| {
                world
                    .get_resource::<LabDiagnostics>()
                    .is_some_and(|diagnostics| !diagnostics.crossfade_active && diagnostics.hero_clip == 1)
            }),
            max_frames: 90,
        })
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "hero reached gust clip",
            |diagnostics| !diagnostics.crossfade_active && diagnostics.hero_clip == 1,
        ))
        .then(assertions::log_summary("vat_crossfade summary"))
        .build()
}
