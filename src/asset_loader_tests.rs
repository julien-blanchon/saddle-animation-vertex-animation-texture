use super::*;

#[test]
fn canonical_metadata_normalizes_successfully() {
    let animation = parse_vat_animation_data_str(include_str!("../assets/demo/wave.vatanim.json"))
        .expect("canonical demo metadata should parse");

    assert_eq!(animation.source_format, crate::VatSourceFormat::Canonical);
    assert_eq!(animation.frame_count, 72);
    assert_eq!(animation.position_texture.rows_per_frame, 1);
    assert_eq!(animation.clips.len(), 3);
    assert!(matches!(
        animation.normal_texture,
        crate::VatNormalTexture::Separate { .. }
    ));
}

#[test]
fn openvat_subset_normalizes_successfully() {
    let animation =
        parse_vat_animation_data_str(include_str!("../assets/demo/openvat_like.vat.json"))
            .expect("openvat-like demo metadata should parse");

    assert_eq!(animation.source_format, crate::VatSourceFormat::OpenVat);
    assert_eq!(animation.vertex_count, 81);
    assert_eq!(animation.frame_count, 72);
    assert_eq!(animation.clips.len(), 3);
}

#[test]
fn unsupported_texture_precision_fails_loudly() {
    let error = parse_vat_animation_data_str(
        r#"{
            "format": "vertex_animation_texture@1",
            "animation_mode": "soft_body_fixed_topology",
            "vertex_count": 4,
            "frame_count": 2,
            "frames_per_second": 24.0,
            "decode_bounds": { "min": [-1.0, -1.0, -1.0], "max": [1.0, 1.0, 1.0] },
            "clips": [{ "name": "idle", "start_frame": 0, "end_frame": 1 }],
            "position_texture": {
                "width": 4,
                "height": 2,
                "rows_per_frame": 1,
                "precision": "rgb10"
            },
            "coordinate_system": "y_up_right_handed",
            "playback_space": "local",
            "vertex_id_attribute": "uv1",
            "position_encoding": "absolute_normalized_bounds"
        }"#,
    )
    .expect_err("unknown precision should fail");

    assert!(
        error
            .to_string()
            .contains("unsupported texture precision 'rgb10'"),
        "expected actionable precision error, got: {error}"
    );
}

#[test]
fn missing_separate_normal_descriptor_fails_loudly() {
    let error = parse_vat_animation_data_str(
        r#"{
            "format": "vertex_animation_texture@1",
            "animation_mode": "soft_body_fixed_topology",
            "vertex_count": 4,
            "frame_count": 2,
            "frames_per_second": 24.0,
            "decode_bounds": { "min": [-1.0, -1.0, -1.0], "max": [1.0, 1.0, 1.0] },
            "clips": [{ "name": "idle", "start_frame": 0, "end_frame": 1 }],
            "position_texture": {
                "width": 4,
                "height": 2,
                "rows_per_frame": 1,
                "precision": "png8"
            },
            "normal_texture": {
                "mode": "separate",
                "encoding": "signed_normalized"
            },
            "coordinate_system": "y_up_right_handed",
            "playback_space": "local",
            "vertex_id_attribute": "uv1",
            "position_encoding": "absolute_normalized_bounds"
        }"#,
    )
    .expect_err("missing separate texture descriptor should fail");

    assert!(
        error.to_string().contains("requires a texture descriptor"),
        "expected actionable missing normal texture error, got: {error}"
    );
}

#[test]
fn unsupported_normal_mode_fails_loudly() {
    let error = parse_vat_animation_data_str(
        r#"{
            "format": "vertex_animation_texture@1",
            "animation_mode": "soft_body_fixed_topology",
            "vertex_count": 4,
            "frame_count": 2,
            "frames_per_second": 24.0,
            "decode_bounds": { "min": [-1.0, -1.0, -1.0], "max": [1.0, 1.0, 1.0] },
            "clips": [{ "name": "idle", "start_frame": 0, "end_frame": 1 }],
            "position_texture": {
                "width": 4,
                "height": 2,
                "rows_per_frame": 1,
                "precision": "png8"
            },
            "normal_texture": {
                "mode": "octahedral"
            },
            "coordinate_system": "y_up_right_handed",
            "playback_space": "local",
            "vertex_id_attribute": "uv1",
            "position_encoding": "absolute_normalized_bounds"
        }"#,
    )
    .expect_err("unknown normal mode should fail");

    assert!(
        error
            .to_string()
            .contains("unsupported normal_texture.mode 'octahedral'"),
        "expected actionable normal mode error, got: {error}"
    );
}

#[test]
fn invalid_position_texture_layout_fails_validation() {
    let error = parse_vat_animation_data_str(
        r#"{
            "format": "vertex_animation_texture@1",
            "animation_mode": "soft_body_fixed_topology",
            "vertex_count": 4,
            "frame_count": 3,
            "frames_per_second": 24.0,
            "decode_bounds": { "min": [-1.0, -1.0, -1.0], "max": [1.0, 1.0, 1.0] },
            "clips": [{ "name": "idle", "start_frame": 0, "end_frame": 2 }],
            "position_texture": {
                "width": 4,
                "height": 2,
                "rows_per_frame": 1,
                "precision": "png8"
            },
            "coordinate_system": "y_up_right_handed",
            "playback_space": "local",
            "vertex_id_attribute": "uv1",
            "position_encoding": "absolute_normalized_bounds"
        }"#,
    )
    .expect_err("insufficient position texture height should fail");

    assert!(
        error
            .to_string()
            .contains("position texture height/rows_per_frame do not cover all baked frames"),
        "expected actionable position layout error, got: {error}"
    );
}
