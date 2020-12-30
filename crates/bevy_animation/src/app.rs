use bevy_app::AppBuilder;
use bevy_asset::{Asset, Handle};
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;
use std::{
    any::{type_name, TypeId},
    borrow::Cow,
    mem::drop,
};

use crate::{
    animator::AnimatorRegistry, blending::Blend, lerping::Lerp, reflect, stage,
    AnimatorPropertyRegistry,
};

pub trait AddAnimated {
    fn register_animated_property_type<T: Lerp + Blend + Clone + 'static>(&mut self) -> &mut Self;
    fn register_animated_component<T: Component + Struct + Default>(&mut self) -> &mut Self;
    fn register_animated_asset<T: Asset + Struct + Default>(&mut self) -> &mut Self;
}

impl AddAnimated for AppBuilder {
    fn register_animated_property_type<T: Lerp + Blend + Clone + 'static>(&mut self) -> &mut Self {
        let mut property_registry = self
            .resources_mut()
            .get_or_insert_with(AnimatorPropertyRegistry::default);

        property_registry.register::<T>();
        drop(property_registry);

        self
    }

    fn register_animated_component<T: Component + Struct + Default>(&mut self) -> &mut Self {
        let mut registry = self
            .resources_mut()
            .get_or_insert_with(AnimatorRegistry::default);

        if registry.targets.insert(TypeId::of::<T>()) {
            let component = T::default();
            let descriptor = reflect::AnimatorDescriptor::<T>::from_component(&component);

            registry.static_properties.extend(
                descriptor
                    .properties()
                    .map(|(name, type_id)| (Cow::Owned(name.to_string()), type_id)),
            );

            drop(registry);

            self.add_resource(descriptor);
            self.add_system_to_stage(stage::ANIMATE, reflect::animate_component_system::<T>);
            self
        } else {
            panic!(
                "animated component `{}` already registered",
                type_name::<T>()
            );
        }
    }

    fn register_animated_asset<T: Asset + Struct + Default>(&mut self) -> &mut Self {
        let mut registry = self
            .resources_mut()
            .get_or_insert_with(AnimatorRegistry::default);

        if registry.targets.insert(TypeId::of::<T>()) {
            let asset = T::default();
            let descriptor = reflect::AnimatorDescriptor::<T>::from_asset(&asset);

            registry.static_properties.extend(
                descriptor
                    .properties()
                    .map(|(name, type_id)| (Cow::Owned(name.to_string()), type_id)),
            );

            drop(registry);

            // Register property animators that might be useful
            self.register_animated_property_type::<Handle<T>>();
            self.register_animated_property_type::<Option<Handle<T>>>();

            self.add_resource(descriptor);
            self.add_system_to_stage(stage::ANIMATE, reflect::animate_asset_system::<T>);
            self
        } else {
            panic!("animated asset `{}` already registered", type_name::<T>());
        }
    }
}
