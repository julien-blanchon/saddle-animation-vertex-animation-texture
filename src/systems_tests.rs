use approx::assert_abs_diff_eq;

use super::*;
use crate::{
    VatAnimationData, VatAnimationMode, VatClip, VatClipEvent, VatCoordinateSystem,
    VatNormalTexture, VatPlaybackSpace, VatPositionEncoding, VatSourceFormat,
    VatTextureDescriptor, VatTexturePrecision, VatVertexIdAttribute, decode_position_sample,
    parse_vat_animation_data_str,
};

fn demo_animation() -> VatAnimationData {
    parse_vat_animation_data_str(include_str!("../assets/demo/wave.vatanim.json"))
        .expect("demo metadata should parse")
}

fn sample_animation_for_events() -> VatAnimationData {
    VatAnimationData {
        source_format: VatSourceFormat::Canonical,
        animation_mode: VatAnimationMode::SoftBodyFixedTopology,
        vertex_count: 4,
        frame_count: 10,
        frames_per_second: 10.0,
        decode_bounds_min: Vec3::splat(-1.0),
        decode_bounds_max: Vec3::splat(1.0),
        animation_bounds_min: Vec3::splat(-1.0),
        animation_bounds_max: Vec3::splat(1.0),
        clips: vec![VatClip {
            name: "test".into(),
            start_frame: 0,
            end_frame: 9,
            default_loop_mode: Some(VatLoopMode::Loop),
            events: vec![
                VatClipEvent {
                    name: "early".into(),
                    frame: 1,
                },
                VatClipEvent {
                    name: "middle".into(),
                    frame: 5,
                },
            ],
        }],
        position_texture: VatTextureDescriptor {
            relative_path: None,
            width: 4,
            height: 10,
            rows_per_frame: 1,
            precision: VatTexturePrecision::Png8,
        },
        normal_texture: VatNormalTexture::None,
        rotation_texture: None,
        auxiliary_textures: Vec::new(),
        coordinate_system: VatCoordinateSystem::YUpRightHanded,
        playback_space: VatPlaybackSpace::Local,
        vertex_id_attribute: VatVertexIdAttribute::Uv1,
        position_encoding: VatPositionEncoding::AbsoluteNormalizedBounds,
    }
}

#[test]
fn playback_advances_by_delta_times_speed() {
    let result = advance_clip_time(0.2, 1.0, true, 2.0, 0.25, VatLoopMode::Loop, 1.0);
    assert_abs_diff_eq!(result.time_seconds, 0.7, epsilon = 0.0001);
    assert_eq!(result.finished_count, 0);
}

#[test]
fn loop_wraps_correctly() {
    let result = advance_clip_time(0.9, 1.0, true, 1.0, 0.25, VatLoopMode::Loop, 1.0);
    assert_abs_diff_eq!(result.time_seconds, 0.15, epsilon = 0.0001);
    assert_eq!(result.finished_count, 1);
}

#[test]
fn once_clamps_and_pauses() {
    let result = advance_clip_time(0.9, 1.0, true, 1.0, 0.25, VatLoopMode::Once, 1.0);
    assert_abs_diff_eq!(result.time_seconds, 1.0, epsilon = 0.0001);
    assert!(result.should_pause);
    assert_eq!(result.finished_count, 1);
}

#[test]
fn ping_pong_reverses_correctly() {
    let result = advance_clip_time(0.9, 1.0, true, 1.0, 0.25, VatLoopMode::PingPong, 1.0);
    assert_abs_diff_eq!(result.time_seconds, 0.85, epsilon = 0.0001);
    assert_abs_diff_eq!(result.direction, -1.0, epsilon = 0.0001);
    assert_eq!(result.finished_count, 1);
}

#[test]
fn clamp_forever_stays_pinned_without_pausing() {
    let result = advance_clip_time(
        0.95,
        1.0,
        true,
        1.0,
        0.25,
        VatLoopMode::ClampForever,
        1.0,
    );
    assert_abs_diff_eq!(result.time_seconds, 1.0, epsilon = 0.0001);
    assert!(!result.should_pause);
    assert_eq!(result.finished_count, 1);
}

#[test]
fn negative_speed_wraps_in_loop_mode() {
    let result = advance_clip_time(0.2, 1.0, true, -0.5, 1.0, VatLoopMode::Loop, 1.0);
    assert_abs_diff_eq!(result.time_seconds, 0.7, epsilon = 0.0001);
    assert_eq!(result.finished_count, 1);
}

#[test]
fn zero_speed_keeps_playback_static() {
    let result = advance_clip_time(0.4, 1.0, true, 0.0, 1.0, VatLoopMode::Loop, 1.0);
    assert_abs_diff_eq!(result.time_seconds, 0.4, epsilon = 0.0001);
    assert!(result.segments.is_empty());
}

#[test]
fn large_delta_stays_in_bounds() {
    let result = advance_clip_time(0.0, 1.0, true, 1.0, 3.6, VatLoopMode::Loop, 1.0);
    assert_abs_diff_eq!(result.time_seconds, 0.6, epsilon = 0.0001);
    assert_eq!(result.finished_count, 3);
}

#[test]
fn crossfade_weight_progresses_linearly() {
    let mut crossfade = crate::VatCrossfade::new(0, 1, 0.6);
    crossfade.elapsed = 0.3;
    assert_abs_diff_eq!(crossfade.weight(), 0.5, epsilon = 0.0001);
}

#[test]
fn clip_finished_message_fires_once_per_wrap() {
    let animation = demo_animation();
    let clip = animation.clip(0).unwrap();
    let result = advance_clip_time(
        clip_duration_seconds(&animation, clip) - 0.05,
        1.0,
        true,
        1.0,
        0.1,
        VatLoopMode::Loop,
        clip_duration_seconds(&animation, clip),
    );

    let mut events = Vec::new();
    let mut finishes = Vec::new();
    enqueue_messages(&animation, 0, &result, &mut events, &mut finishes);

    assert!(events.is_empty());
    assert_eq!(finishes.len(), 1);
    assert_eq!(finishes[0].clip_name, "idle");
}

#[test]
fn timeline_event_fires_once_per_threshold_crossing() {
    let animation = sample_animation_for_events();
    let result = advance_clip_time(0.4, 1.0, true, 1.0, 0.2, VatLoopMode::Loop, 1.0);
    let mut events = Vec::new();
    let mut finishes = Vec::new();
    enqueue_messages(&animation, 0, &result, &mut events, &mut finishes);

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_name, "middle");
}

#[test]
fn ping_pong_event_detects_reverse_crossing() {
    let animation = sample_animation_for_events();
    let result = advance_clip_time(0.6, -1.0, true, 1.0, 0.2, VatLoopMode::PingPong, 1.0);
    let mut events = Vec::new();
    let mut finishes = Vec::new();
    enqueue_messages(&animation, 0, &result, &mut events, &mut finishes);

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_name, "middle");
}

#[test]
fn clip_frame_sampling_uses_absolute_frame_range() {
    let animation = demo_animation();
    let clip = animation.clip(1).unwrap();
    let frame = sample_frame_state(&animation, clip, 1, 0.4375, false, true);
    assert_abs_diff_eq!(frame.frame_a, 34.0, epsilon = 0.0001);
    assert_abs_diff_eq!(frame.frame_b, 35.0, epsilon = 0.0001);
    assert_abs_diff_eq!(frame.blend, 0.5, epsilon = 0.0001);
}

#[test]
fn bounds_decode_round_trips() {
    let animation = demo_animation();
    let target = Vec3::new(0.25, 1.0, -0.12);
    let encoded =
        (target - animation.decode_bounds_min) / (animation.decode_bounds_max - animation.decode_bounds_min);
    let decoded = decode_position_sample(encoded, &animation, Vec3::ZERO);
    assert_abs_diff_eq!(decoded.x, target.x, epsilon = 0.01);
    assert_abs_diff_eq!(decoded.y, target.y, epsilon = 0.01);
    assert_abs_diff_eq!(decoded.z, target.z, epsilon = 0.01);
}
