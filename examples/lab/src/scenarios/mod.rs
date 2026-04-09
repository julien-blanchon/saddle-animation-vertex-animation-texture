use bevy::prelude::*;
use saddle_bevy_e2e::{action::Action, actions::{assertions, inspect}, scenario::Scenario};

use crate::{BoundsProbe, CrowdMember, Hero, LabControl, LabDiagnostics};
use crate::lab_e2e_support::{
    request_clip_name, wait_for_clip_name, wait_for_crossfade_finish, wait_for_crossfade_start,
    wait_for_paused_state, wait_for_resumed_state,
};

pub fn list_scenarios() -> Vec<&'static str> {
    vec![
        "vat_metadata_default",
        "vat_smoke",
        "vat_multi_clip",
        "vat_crowd",
        "vat_modular_sync",
        "vat_bounds_regression",
        "vat_crossfade",
        "vat_debug_controls",
    ]
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "vat_metadata_default" => Some(build_metadata_default()),
        "vat_smoke" => Some(build_smoke()),
        "vat_multi_clip" => Some(build_multi_clip()),
        "vat_crowd" => Some(build_crowd()),
        "vat_modular_sync" => Some(build_modular_sync()),
        "vat_bounds_regression" => Some(build_bounds_regression()),
        "vat_crossfade" => Some(build_crossfade()),
        "vat_debug_controls" => Some(build_debug_controls()),
        _ => None,
    }
}

fn build_metadata_default() -> Scenario {
    Scenario::builder("vat_metadata_default")
        .description(
            "Verify the hero boots on the metadata-declared default clip without any explicit numeric clip selection.",
        )
        .then(Action::WaitFrames(12))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "hero started on metadata default clip",
            |diagnostics| diagnostics.hero_clip_name == "idle" && diagnostics.hero_playing,
        ))
        .then(Action::Screenshot("vat_metadata_default".into()))
        .then(assertions::log_summary("vat_metadata_default summary"))
        .build()
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
        .then(request_clip_name("burst"))
        .then(wait_for_clip_name("burst", 90))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "hero switched to burst clip",
            |diagnostics| diagnostics.hero_clip_name == "burst",
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
        .then(request_clip_name("idle"))
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

fn build_modular_sync() -> Scenario {
    Scenario::builder("vat_modular_sync")
        .description(
            "Verify modular follower meshes stay phase-aligned with the hero while preserving their configured time offsets.",
        )
        .then(Action::WaitFrames(28))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "modular followers exist and stay tightly synced",
            |diagnostics| diagnostics.follower_count >= 2 && diagnostics.follower_sync_error < 0.025,
        ))
        .then(Action::Screenshot("vat_modular_sync".into()))
        .then(assertions::log_summary("vat_modular_sync summary"))
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
        .then(request_clip_name("gust"))
        .then(wait_for_crossfade_start(60))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "crossfade is active",
            |diagnostics| diagnostics.crossfade_active,
        ))
        .then(Action::Screenshot("vat_crossfade_start".into()))
        .then(Action::WaitFrames(8))
        .then(Action::Screenshot("vat_crossfade_mid".into()))
        .then(wait_for_crossfade_finish("gust", 90))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "hero reached gust clip",
            |diagnostics| !diagnostics.crossfade_active && diagnostics.hero_clip_name == "gust",
        ))
        .then(assertions::log_summary("vat_crossfade summary"))
        .build()
}

fn build_debug_controls() -> Scenario {
    Scenario::builder("vat_debug_controls")
        .description(
            "Drive the lab through its interactive debug shortcuts, verifying pause, interpolation toggles, and clip selection through real key input.",
        )
        .then(Action::WaitFrames(30))
        .then(Action::HoldKey {
            key: KeyCode::Space,
            frames: 1,
        })
        .then(wait_for_paused_state(60))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "hero playback paused",
            |diagnostics| !diagnostics.hero_playing,
        ))
        .then(inspect::log_resource::<LabDiagnostics>(
            "vat_debug_controls_paused",
        ))
        .then(Action::Screenshot("vat_debug_controls_paused".into()))
        .then(Action::WaitFrames(1))
        .then(Action::HoldKey {
            key: KeyCode::Space,
            frames: 1,
        })
        .then(wait_for_resumed_state(60))
        .then(Action::HoldKey {
            key: KeyCode::KeyI,
            frames: 1,
        })
        .then(Action::WaitUntil {
            label: "interpolation disabled".into(),
            condition: Box::new(|world: &World| !world.resource::<LabControl>().interpolation_enabled),
            max_frames: 60,
        })
        .then(assertions::resource_satisfies::<LabControl>(
            "interpolation shortcut toggled off",
            |control| !control.interpolation_enabled,
        ))
        .then(inspect::log_resource::<LabDiagnostics>(
            "vat_debug_controls_resumed",
        ))
        .then(Action::Screenshot("vat_debug_controls_stepped".into()))
        .then(Action::WaitFrames(1))
        .then(Action::HoldKey {
            key: KeyCode::Digit2,
            frames: 1,
        })
        .then(Action::WaitUntil {
            label: "gust clip active".into(),
            condition: Box::new(|world: &World| {
                let diagnostics = world.resource::<LabDiagnostics>();
                diagnostics.hero_clip_name == "gust" && diagnostics.crossfade_active
            }),
            max_frames: 120,
        })
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "gust crossfade activated",
            |diagnostics| diagnostics.hero_clip_name == "gust" && diagnostics.crossfade_active,
        ))
        .then(Action::Screenshot("vat_debug_controls_transition".into()))
        .then(Action::WaitUntil {
            label: "gust crossfade resolved".into(),
            condition: Box::new(|world: &World| {
                let diagnostics = world.resource::<LabDiagnostics>();
                diagnostics.hero_clip_name == "gust" && !diagnostics.crossfade_active
            }),
            max_frames: 120,
        })
        .then(inspect::log_resource::<LabDiagnostics>(
            "vat_debug_controls_final",
        ))
        .then(Action::Screenshot("vat_debug_controls_final".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("vat_debug_controls"))
        .build()
}
