use bevy::{
    app::PostStartup,
    ecs::{intern::Interned, schedule::ScheduleLabel},
    pbr::MaterialPlugin,
    prelude::*,
};

mod asset;
mod asset_loader;
mod components;
mod material;
mod messages;
mod systems;
mod validation;

pub use asset::{
    VatAnimationData, VatAnimationMode, VatAuxTextureDescriptor, VatAuxTextureSemantic, VatClip,
    VatClipEvent, VatCoordinateSystem, VatNormalEncoding, VatNormalTexture, VatPlaybackSpace,
    VatPositionEncoding, VatSourceFormat, VatTextureDescriptor, VatTexturePrecision,
    VatVertexIdAttribute,
};
pub use asset_loader::{
    VatAnimationDataLoader, VatMetadataLoadError, parse_vat_animation_data_bytes,
    parse_vat_animation_data_str,
};
pub use components::{
    VatAnimationBundle, VatAnimationSource, VatBoundsMode, VatCrossfade, VatLoopMode, VatPlayback,
    VatPlaybackFollower, VatPlaybackTweaks,
};
pub use material::{
    VatMaterial, VatMaterialBuildError, VatMaterialDefaults, VatMaterialExt, VatMaterialUniform,
    build_vat_material,
};
pub use messages::{VatClipFinished, VatEventReached};
pub use validation::{
    VatMeshValidationError, VatValidationError, configure_vat_data_image,
    convert_coordinate_system, decode_position_sample, make_linear_rgba8_image, metadata_aabb,
    should_disable_frustum_culling, valid_bounds, validate_animation_data,
    validate_mesh_for_animation,
};

#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum VatSystems {
    AdvancePlayback,
    SyncFollowers,
    ResolveTransitions,
    EmitMessages,
    SyncGpuState,
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct NeverDeactivateSchedule;

pub struct VertexAnimationTexturePlugin {
    pub activate_schedule: Interned<dyn ScheduleLabel>,
    pub deactivate_schedule: Interned<dyn ScheduleLabel>,
    pub update_schedule: Interned<dyn ScheduleLabel>,
}

impl VertexAnimationTexturePlugin {
    #[must_use]
    pub fn new(
        activate_schedule: impl ScheduleLabel,
        deactivate_schedule: impl ScheduleLabel,
        update_schedule: impl ScheduleLabel,
    ) -> Self {
        Self {
            activate_schedule: activate_schedule.intern(),
            deactivate_schedule: deactivate_schedule.intern(),
            update_schedule: update_schedule.intern(),
        }
    }

    #[must_use]
    pub fn always_on(update_schedule: impl ScheduleLabel) -> Self {
        Self::new(PostStartup, NeverDeactivateSchedule, update_schedule)
    }
}

impl Default for VertexAnimationTexturePlugin {
    fn default() -> Self {
        Self::always_on(Update)
    }
}

impl Plugin for VertexAnimationTexturePlugin {
    fn build(&self, app: &mut App) {
        if self.deactivate_schedule == NeverDeactivateSchedule.intern() {
            app.init_schedule(NeverDeactivateSchedule);
        }

        material::load_shaders(app);

        app.add_plugins(MaterialPlugin::<VatMaterial>::default())
            .init_asset::<VatAnimationData>()
            .init_asset_loader::<VatAnimationDataLoader>()
            .register_asset_reflect::<VatAnimationData>()
            .init_resource::<material::VatMaterialDefaults>()
            .init_resource::<systems::VatRuntimeState>()
            .add_message::<VatClipFinished>()
            .add_message::<VatEventReached>()
            .register_type::<VatAnimationData>()
            .register_type::<VatAnimationMode>()
            .register_type::<VatAnimationSource>()
            .register_type::<VatAuxTextureDescriptor>()
            .register_type::<VatBoundsMode>()
            .register_type::<VatClip>()
            .register_type::<VatClipEvent>()
            .register_type::<VatCoordinateSystem>()
            .register_type::<VatCrossfade>()
            .register_type::<VatLoopMode>()
            .register_type::<VatNormalEncoding>()
            .register_type::<VatNormalTexture>()
            .register_type::<VatPlayback>()
            .register_type::<VatPlaybackFollower>()
            .register_type::<VatPlaybackSpace>()
            .register_type::<VatPlaybackTweaks>()
            .register_type::<VatPositionEncoding>()
            .register_type::<VatSourceFormat>()
            .register_type::<VatTextureDescriptor>()
            .register_type::<VatTexturePrecision>()
            .register_type::<VatVertexIdAttribute>()
            .add_systems(self.activate_schedule, systems::activate_runtime)
            .add_systems(self.deactivate_schedule, systems::deactivate_runtime)
            .configure_sets(
                self.update_schedule,
                (
                    VatSystems::AdvancePlayback,
                    VatSystems::SyncFollowers,
                    VatSystems::ResolveTransitions,
                    VatSystems::EmitMessages,
                    VatSystems::SyncGpuState,
                )
                    .chain(),
            )
            .add_systems(
                self.update_schedule,
                (
                    systems::ensure_runtime_components,
                    systems::advance_playback,
                )
                    .chain()
                    .in_set(VatSystems::AdvancePlayback)
                    .run_if(systems::runtime_is_active),
            )
            .add_systems(
                self.update_schedule,
                systems::sync_playback_followers
                    .in_set(VatSystems::SyncFollowers)
                    .run_if(systems::runtime_is_active),
            )
            .add_systems(
                self.update_schedule,
                systems::resolve_crossfades
                    .in_set(VatSystems::ResolveTransitions)
                    .run_if(systems::runtime_is_active),
            )
            .add_systems(
                self.update_schedule,
                systems::emit_messages
                    .in_set(VatSystems::EmitMessages)
                    .run_if(systems::runtime_is_active),
            )
            .add_systems(
                self.update_schedule,
                (
                    systems::validate_bindings_and_apply_bounds,
                    systems::sync_gpu_state,
                )
                    .chain()
                    .in_set(VatSystems::SyncGpuState)
                    .run_if(systems::runtime_is_active),
            );
    }
}

#[cfg(test)]
#[path = "integration_tests.rs"]
mod integration_tests;
