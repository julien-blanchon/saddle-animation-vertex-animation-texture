use std::collections::{HashMap, hash_map::Entry};

use bevy::{
    asset::AssetEvent, camera::visibility::NoFrustumCulling, mesh::MeshTag, prelude::*,
    render::storage::ShaderStorageBuffer,
};

use crate::{
    VatAnimationData, VatAnimationSource, VatBoundsMode, VatClipFinished, VatCrossfade,
    VatEventReached, VatLoopMode, VatMaterial, VatPlayback, VatPlaybackTweaks,
    material::VatGpuInstance,
    validation::{VatMeshValidationError, metadata_aabb, should_disable_frustum_culling},
};

#[derive(Resource, Default)]
pub(crate) struct VatRuntimeState {
    pub active: bool,
}

#[derive(Component, Debug)]
pub(crate) struct VatPlaybackRuntime {
    pub direction: f32,
    pub last_clip_index: usize,
    pub pending_events: Vec<PendingEvent>,
    pub pending_finishes: Vec<PendingFinish>,
}

impl Default for VatPlaybackRuntime {
    fn default() -> Self {
        Self {
            direction: 1.0,
            last_clip_index: 0,
            pending_events: Vec::new(),
            pending_finishes: Vec::new(),
        }
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct VatCrossfadeRuntime {
    pub source_clip: usize,
    pub source_time_seconds: f32,
    pub source_direction: f32,
}

#[derive(Component, Debug)]
pub(crate) struct VatBindingFailure {
    pub reason: String,
}

#[derive(Component, Debug, Default)]
pub(crate) struct VatBindingReady;

#[derive(Clone, Debug)]
pub(crate) struct PendingEvent {
    pub clip_index: usize,
    pub clip_name: String,
    pub event_name: String,
    pub clip_frame: u32,
    pub normalized_time: f32,
    pub reached_at_seconds: f32,
}

#[derive(Clone, Debug)]
pub(crate) struct PendingFinish {
    pub clip_index: usize,
    pub clip_name: String,
    pub finished_at_seconds: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TraversalSegment {
    start_seconds: f32,
    end_seconds: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct AdvanceResult {
    time_seconds: f32,
    direction: f32,
    finished_count: u32,
    should_pause: bool,
    segments: Vec<TraversalSegment>,
}

pub(crate) fn activate_runtime(mut runtime: ResMut<VatRuntimeState>) {
    runtime.active = true;
}

pub(crate) fn deactivate_runtime(mut runtime: ResMut<VatRuntimeState>) {
    runtime.active = false;
}

pub(crate) fn runtime_is_active(runtime: Res<VatRuntimeState>) -> bool {
    runtime.active
}

pub(crate) fn ensure_runtime_components(
    mut commands: Commands,
    query: Query<Entity, (With<VatPlayback>, Without<VatPlaybackRuntime>)>,
) {
    for entity in &query {
        commands
            .entity(entity)
            .insert(VatPlaybackRuntime::default());
    }
}

pub(crate) fn advance_playback(
    time: Res<Time>,
    animations: Res<Assets<VatAnimationData>>,
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &VatAnimationSource,
        &mut VatPlayback,
        Option<&VatCrossfade>,
        &mut VatPlaybackRuntime,
        Option<&mut VatCrossfadeRuntime>,
    )>,
) {
    let delta_seconds = time.delta_secs();

    for (entity, source, mut playback, crossfade, mut runtime, crossfade_runtime) in &mut query {
        runtime.pending_events.clear();
        runtime.pending_finishes.clear();

        let Some(animation) = animations.get(&source.animation) else {
            continue;
        };
        let Some(active_clip) = animation.clip(playback.active_clip) else {
            error!(
                "VAT entity {:?} requested clip index {} but metadata only has {} clips",
                entity,
                playback.active_clip,
                animation.clips.len()
            );
            playback.active_clip = 0;
            runtime.last_clip_index = 0;
            continue;
        };

        if runtime.last_clip_index != playback.active_clip {
            runtime.direction = 1.0;
            runtime.last_clip_index = playback.active_clip;
            playback.time_seconds = playback
                .time_seconds
                .clamp(0.0, clip_duration_seconds(animation, active_clip));
        }

        let effective_loop_mode = resolve_loop_mode(&playback, active_clip);
        let advance_result = advance_clip_time(
            playback.time_seconds,
            runtime.direction,
            playback.playing,
            playback.speed,
            delta_seconds,
            effective_loop_mode,
            clip_duration_seconds(animation, active_clip),
        );

        playback.time_seconds = advance_result.time_seconds;
        runtime.direction = advance_result.direction;

        if advance_result.should_pause {
            playback.playing = false;
        }

        let mut pending_events = Vec::new();
        let mut pending_finishes = Vec::new();
        enqueue_messages(
            animation,
            playback.active_clip,
            &advance_result,
            &mut pending_events,
            &mut pending_finishes,
        );
        runtime.pending_events.extend(pending_events);
        runtime.pending_finishes.extend(pending_finishes);

        if let Some(crossfade) = crossfade {
            let mut crossfade_runtime = match crossfade_runtime {
                Some(existing) => existing,
                None => {
                    commands.entity(entity).insert(VatCrossfadeRuntime {
                        source_clip: crossfade.from_clip,
                        source_time_seconds: playback.time_seconds,
                        source_direction: runtime.direction,
                    });
                    continue;
                }
            };

            if crossfade_runtime.source_clip != crossfade.from_clip {
                crossfade_runtime.source_clip = crossfade.from_clip;
                crossfade_runtime.source_time_seconds = playback.time_seconds;
                crossfade_runtime.source_direction = runtime.direction;
            }

            if playback.active_clip != crossfade.to_clip {
                crossfade_runtime.source_clip = playback.active_clip;
                crossfade_runtime.source_time_seconds = playback.time_seconds;
                crossfade_runtime.source_direction = runtime.direction;
                playback.active_clip = crossfade
                    .to_clip
                    .min(animation.clips.len().saturating_sub(1));
                playback.time_seconds = 0.0;
                runtime.direction = 1.0;
                runtime.last_clip_index = playback.active_clip;
            }

            if let Some(source_clip) = animation.clip(crossfade_runtime.source_clip) {
                let source_result = advance_clip_time(
                    crossfade_runtime.source_time_seconds,
                    crossfade_runtime.source_direction,
                    playback.playing,
                    playback.speed,
                    delta_seconds,
                    resolve_loop_mode(&playback, source_clip),
                    clip_duration_seconds(animation, source_clip),
                );
                crossfade_runtime.source_time_seconds = source_result.time_seconds;
                crossfade_runtime.source_direction = source_result.direction;
            }
        }
    }
}

pub(crate) fn resolve_crossfades(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut VatCrossfade, &VatPlayback)>,
) {
    for (entity, mut crossfade, playback) in &mut query {
        if playback.playing {
            crossfade.elapsed = (crossfade.elapsed + time.delta_secs()).min(crossfade.duration);
        }

        if crossfade.elapsed >= crossfade.duration {
            commands.entity(entity).remove::<VatCrossfade>();
            commands.entity(entity).remove::<VatCrossfadeRuntime>();
        }
    }
}

pub(crate) fn emit_messages(
    mut clip_finished: MessageWriter<VatClipFinished>,
    mut event_reached: MessageWriter<VatEventReached>,
    mut query: Query<(Entity, &mut VatPlaybackRuntime)>,
) {
    for (entity, mut runtime) in &mut query {
        for finish in runtime.pending_finishes.drain(..) {
            clip_finished.write(VatClipFinished {
                entity,
                clip_index: finish.clip_index,
                clip_name: finish.clip_name,
                finished_at_seconds: finish.finished_at_seconds,
            });
        }

        for event in runtime.pending_events.drain(..) {
            event_reached.write(VatEventReached {
                entity,
                clip_index: event.clip_index,
                clip_name: event.clip_name,
                event_name: event.event_name,
                clip_frame: event.clip_frame,
                normalized_time: event.normalized_time,
                reached_at_seconds: event.reached_at_seconds,
            });
        }
    }
}

pub(crate) fn validate_bindings_and_apply_bounds(
    mut commands: Commands,
    animations: Res<Assets<VatAnimationData>>,
    meshes: Res<Assets<Mesh>>,
    mut animation_events: MessageReader<AssetEvent<VatAnimationData>>,
    mut mesh_events: MessageReader<AssetEvent<Mesh>>,
    query: Query<(
        Entity,
        Ref<VatAnimationSource>,
        Ref<Mesh3d>,
        Has<NoFrustumCulling>,
        Has<VatBindingReady>,
        Option<&VatBindingFailure>,
    )>,
) {
    let assets_changed = animation_events.read().count() > 0 || mesh_events.read().count() > 0;

    for (entity, source, mesh_handle, has_no_frustum_culling, has_binding_ready, binding_failure) in
        &query
    {
        let source_changed = source.is_added() || source.is_changed();
        let mesh_changed = mesh_handle.is_added() || mesh_handle.is_changed();

        if !source_changed
            && !mesh_changed
            && !assets_changed
            && (has_binding_ready || binding_failure.is_some())
        {
            continue;
        }

        let Some(animation) = animations.get(&source.animation) else {
            continue;
        };
        let Some(mesh) = meshes.get(&*mesh_handle) else {
            continue;
        };

        match validate_mesh_for_animation(mesh, animation) {
            Ok(()) => {
                if binding_failure.is_some() {
                    commands.entity(entity).remove::<VatBindingFailure>();
                }
                if !has_binding_ready {
                    commands.entity(entity).insert(VatBindingReady);
                }
            }
            Err(error) => {
                let reason = error.to_string();
                error!(
                    "VAT binding validation failed for entity {:?}: {}",
                    entity, reason
                );
                if has_binding_ready {
                    commands.entity(entity).remove::<VatBindingReady>();
                }
                match binding_failure {
                    Some(existing) if existing.reason == reason => {}
                    _ => {
                        commands.entity(entity).insert(VatBindingFailure { reason });
                    }
                }
                continue;
            }
        }

        match source.bounds_mode {
            VatBoundsMode::UseMetadataAabb => {
                commands.entity(entity).insert(metadata_aabb(animation));
            }
            VatBoundsMode::KeepProxyAabb => {}
            VatBoundsMode::DisableFrustumCulling => {}
        }

        let disable_culling = should_disable_frustum_culling(animation, source.bounds_mode);
        if disable_culling && !has_no_frustum_culling {
            commands.entity(entity).insert(NoFrustumCulling);
        } else if !disable_culling && has_no_frustum_culling {
            commands.entity(entity).remove::<NoFrustumCulling>();
        }
    }
}

pub(crate) fn sync_gpu_state(
    mut commands: Commands,
    animations: Res<Assets<VatAnimationData>>,
    mut materials: ResMut<Assets<VatMaterial>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    query: Query<(
        Entity,
        &VatAnimationSource,
        &VatPlayback,
        Option<&VatCrossfade>,
        Option<&VatCrossfadeRuntime>,
        Option<&VatPlaybackTweaks>,
        &MeshMaterial3d<VatMaterial>,
        Option<&VatBindingFailure>,
    )>,
) {
    let mut grouped: HashMap<AssetId<VatMaterial>, Vec<(Entity, VatGpuInstance)>> = HashMap::new();

    for (
        entity,
        source,
        playback,
        crossfade,
        crossfade_runtime,
        tweaks,
        material_handle,
        binding_failure,
    ) in &query
    {
        if binding_failure.is_some() {
            continue;
        }
        let Some(animation) = animations.get(&source.animation) else {
            continue;
        };
        let Some(active_clip) = animation.clip(playback.active_clip) else {
            continue;
        };

        let disable_interpolation = tweaks.is_some_and(|tweaks| tweaks.disable_interpolation);
        let wrap_last_frame = matches!(resolve_loop_mode(playback, active_clip), VatLoopMode::Loop);

        let mut instance = sample_gpu_instance(
            animation,
            playback.active_clip,
            playback.time_seconds,
            disable_interpolation,
            wrap_last_frame,
        );

        if let (Some(crossfade), Some(crossfade_runtime)) = (crossfade, crossfade_runtime) {
            instance.options.y = crossfade.weight();
            if let Some(source_clip) = animation.clip(crossfade_runtime.source_clip) {
                let secondary = sample_frame_state(
                    animation,
                    source_clip,
                    crossfade_runtime.source_clip,
                    crossfade_runtime.source_time_seconds,
                    disable_interpolation,
                    matches!(resolve_loop_mode(playback, source_clip), VatLoopMode::Loop),
                );
                instance.secondary_frames =
                    Vec4::new(secondary.frame_a, secondary.frame_b, secondary.blend, 0.0);
                instance.options.z = 1.0;
            }
        }

        match grouped.entry(material_handle.id()) {
            Entry::Occupied(mut entry) => entry.get_mut().push((entity, instance)),
            Entry::Vacant(entry) => {
                entry.insert(vec![(entity, instance)]);
            }
        }
    }

    for (material_id, entries) in &mut grouped {
        entries.sort_by_key(|(entity, _)| entity.index());

        let Some(material) = materials.get_mut(*material_id) else {
            continue;
        };

        let data: Vec<VatGpuInstance> = entries.iter().map(|(_, instance)| *instance).collect();
        if let Some(buffer) = buffers.get_mut(&material.extension.instances) {
            buffer.set_data(data);
        } else {
            material.extension.instances = buffers.add(ShaderStorageBuffer::from(data));
        }

        for (index, (entity, _)) in entries.iter().enumerate() {
            commands.entity(*entity).insert(MeshTag(index as u32));
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct FrameState {
    frame_a: f32,
    frame_b: f32,
    blend: f32,
}

fn sample_gpu_instance(
    animation: &VatAnimationData,
    clip_index: usize,
    time_seconds: f32,
    disable_interpolation: bool,
    wrap_last_frame: bool,
) -> VatGpuInstance {
    let clip = animation
        .clip(clip_index)
        .expect("clip index should already be validated");
    let primary = sample_frame_state(
        animation,
        clip,
        clip_index,
        time_seconds,
        disable_interpolation,
        wrap_last_frame,
    );

    VatGpuInstance {
        primary_frames: Vec4::new(primary.frame_a, primary.frame_b, primary.blend, 0.0),
        secondary_frames: Vec4::ZERO,
        options: Vec4::new(if disable_interpolation { 0.0 } else { 1.0 }, 0.0, 0.0, 0.0),
    }
}

fn sample_frame_state(
    animation: &VatAnimationData,
    clip: &crate::VatClip,
    clip_index: usize,
    time_seconds: f32,
    disable_interpolation: bool,
    wrap_last_frame: bool,
) -> FrameState {
    let clip_frame_count = clip.frame_count();
    if clip_frame_count <= 1 {
        let frame = clip.start_frame as f32;
        return FrameState {
            frame_a: frame,
            frame_b: frame,
            blend: 0.0,
        };
    }

    let raw_position = (time_seconds.max(0.0) * animation.frames_per_second).max(0.0);
    let max_position = clip_frame_count as f32;
    let frame_position = if wrap_last_frame {
        raw_position.rem_euclid(max_position)
    } else {
        raw_position.min((clip_frame_count - 1) as f32)
    };

    let relative_frame_a = frame_position
        .floor()
        .clamp(0.0, (clip_frame_count - 1) as f32);
    let frame_a = clip.start_frame as f32 + relative_frame_a;
    let frame_b = if disable_interpolation {
        frame_a
    } else if wrap_last_frame {
        clip.start_frame as f32 + ((relative_frame_a as u32 + 1) % clip_frame_count) as f32
    } else {
        clip.start_frame as f32 + (relative_frame_a as u32 + 1).min(clip_frame_count - 1) as f32
    };

    let blend = if disable_interpolation || frame_a == frame_b {
        0.0
    } else {
        frame_position.fract()
    };

    let _ = clip_index;
    FrameState {
        frame_a,
        frame_b,
        blend,
    }
}

fn enqueue_messages(
    animation: &VatAnimationData,
    clip_index: usize,
    advance_result: &AdvanceResult,
    pending_events: &mut Vec<PendingEvent>,
    pending_finishes: &mut Vec<PendingFinish>,
) {
    let Some(clip) = animation.clip(clip_index) else {
        return;
    };

    for segment in &advance_result.segments {
        for event in &clip.events {
            let threshold = event.frame as f32 / animation.frames_per_second;
            if crosses_threshold(*segment, threshold) {
                pending_events.push(PendingEvent {
                    clip_index,
                    clip_name: clip.name.clone(),
                    event_name: event.name.clone(),
                    clip_frame: event.frame,
                    normalized_time: clip.normalized_time_for_frame(event.frame),
                    reached_at_seconds: threshold,
                });
            }
        }
    }

    for _ in 0..advance_result.finished_count {
        pending_finishes.push(PendingFinish {
            clip_index,
            clip_name: clip.name.clone(),
            finished_at_seconds: clip_duration_seconds(animation, clip),
        });
    }
}

fn resolve_loop_mode(playback: &VatPlayback, clip: &crate::VatClip) -> VatLoopMode {
    if playback.loop_mode == VatLoopMode::Loop {
        clip.default_loop_mode.unwrap_or(playback.loop_mode)
    } else {
        playback.loop_mode
    }
}

fn clip_duration_seconds(animation: &VatAnimationData, clip: &crate::VatClip) -> f32 {
    clip.frame_count() as f32 / animation.frames_per_second
}

fn crosses_threshold(segment: TraversalSegment, threshold: f32) -> bool {
    const EPSILON: f32 = 0.0001;
    if (segment.start_seconds - segment.end_seconds).abs() <= EPSILON {
        return false;
    }

    if segment.end_seconds > segment.start_seconds {
        threshold > segment.start_seconds + EPSILON && threshold <= segment.end_seconds + EPSILON
    } else {
        threshold < segment.start_seconds - EPSILON && threshold >= segment.end_seconds - EPSILON
    }
}

fn advance_clip_time(
    current_time_seconds: f32,
    current_direction: f32,
    playing: bool,
    speed: f32,
    delta_seconds: f32,
    loop_mode: VatLoopMode,
    duration_seconds: f32,
) -> AdvanceResult {
    const EPSILON: f32 = 0.0001;

    if !playing || speed.abs() <= EPSILON || duration_seconds <= EPSILON {
        return AdvanceResult {
            time_seconds: current_time_seconds.clamp(0.0, duration_seconds.max(0.0)),
            direction: current_direction.signum().max(1.0),
            finished_count: 0,
            should_pause: false,
            segments: Vec::new(),
        };
    }

    let mut time_seconds = current_time_seconds.clamp(0.0, duration_seconds);
    let mut direction = if current_direction.abs() <= EPSILON {
        1.0
    } else {
        current_direction.signum()
    };
    let mut remaining = if loop_mode == VatLoopMode::PingPong {
        delta_seconds * speed * direction
    } else {
        delta_seconds * speed
    };
    let mut finished_count = 0;
    let mut should_pause = false;
    let mut segments = Vec::new();

    for _ in 0..256 {
        if remaining.abs() <= EPSILON {
            break;
        }

        match loop_mode {
            VatLoopMode::Loop => {
                if remaining > 0.0 {
                    let distance_to_end = duration_seconds - time_seconds;
                    if distance_to_end > EPSILON && remaining < distance_to_end - EPSILON {
                        let end = time_seconds + remaining;
                        segments.push(TraversalSegment {
                            start_seconds: time_seconds,
                            end_seconds: end,
                        });
                        time_seconds = end;
                        remaining = 0.0;
                    } else {
                        if distance_to_end > EPSILON {
                            segments.push(TraversalSegment {
                                start_seconds: time_seconds,
                                end_seconds: duration_seconds,
                            });
                            remaining -= distance_to_end;
                        }
                        finished_count += 1;
                        time_seconds = 0.0;
                    }
                } else {
                    let distance_to_start = time_seconds;
                    if distance_to_start > EPSILON && -remaining < distance_to_start - EPSILON {
                        let end = time_seconds + remaining;
                        segments.push(TraversalSegment {
                            start_seconds: time_seconds,
                            end_seconds: end,
                        });
                        time_seconds = end;
                        remaining = 0.0;
                    } else {
                        if distance_to_start > EPSILON {
                            segments.push(TraversalSegment {
                                start_seconds: time_seconds,
                                end_seconds: 0.0,
                            });
                            remaining += distance_to_start;
                        }
                        finished_count += 1;
                        time_seconds = duration_seconds;
                    }
                }
            }
            VatLoopMode::Once | VatLoopMode::ClampForever => {
                let end = (time_seconds + remaining).clamp(0.0, duration_seconds);
                if (end - time_seconds).abs() > EPSILON {
                    segments.push(TraversalSegment {
                        start_seconds: time_seconds,
                        end_seconds: end,
                    });
                }
                let reached_boundary = (remaining > 0.0 && end >= duration_seconds - EPSILON)
                    || (remaining < 0.0 && end <= EPSILON);
                time_seconds = end;
                remaining = 0.0;
                if reached_boundary {
                    finished_count += 1;
                    should_pause = loop_mode == VatLoopMode::Once;
                }
            }
            VatLoopMode::PingPong => {
                let boundary = if remaining > 0.0 {
                    duration_seconds
                } else {
                    0.0
                };
                let distance = boundary - time_seconds;

                if distance.abs() > EPSILON && remaining.abs() < distance.abs() - EPSILON {
                    let end = time_seconds + remaining;
                    segments.push(TraversalSegment {
                        start_seconds: time_seconds,
                        end_seconds: end,
                    });
                    time_seconds = end;
                    remaining = 0.0;
                } else {
                    if distance.abs() > EPSILON {
                        segments.push(TraversalSegment {
                            start_seconds: time_seconds,
                            end_seconds: boundary,
                        });
                    }
                    time_seconds = boundary;
                    remaining = -(remaining - distance);
                    direction *= -1.0;
                    finished_count += 1;
                }
            }
        }
    }

    AdvanceResult {
        time_seconds,
        direction,
        finished_count,
        should_pause,
        segments,
    }
}

pub(crate) fn validate_mesh_for_animation(
    mesh: &Mesh,
    animation: &VatAnimationData,
) -> Result<(), VatMeshValidationError> {
    crate::validation::validate_mesh_for_animation(mesh, animation)
}

#[cfg(test)]
#[path = "systems_tests.rs"]
mod tests;
