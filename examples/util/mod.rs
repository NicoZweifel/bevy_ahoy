//! Common functionality for the examples. This is just aesthetic stuff, you don't need to copy any of this into your own projects.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_ahoy::{kcc::CharacterControllerState, prelude::*};
use bevy_ecs::world::FilteredEntityRef;
use bevy_enhanced_input::prelude::{Release, *};
use bevy_mod_mipmap_generator::{MipmapGeneratorPlugin, generate_mipmaps};

pub(super) struct ExampleUtilPlugin;

impl Plugin for ExampleUtilPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MipmapGeneratorPlugin)
            .add_systems(Startup, setup_ui)
            .add_systems(
                Update,
                (update_debug_text, generate_mipmaps::<StandardMaterial>),
            )
            .add_observer(reset_player)
            .add_input_context::<DebugInput>();
    }
}

fn update_debug_text(
    mut text: Single<&mut Text, With<DebugText>>,
    kcc: Single<
        (
            &CharacterControllerState,
            &LinearVelocity,
            &CollidingEntities,
            &ColliderAabb,
        ),
        With<CharacterController>,
    >,
    camera: Single<&Transform, With<Camera>>,
    names: Query<NameOrEntity>,
) {
    let (state, velocity, colliding_entities, aabb) = kcc.into_inner();
    let velocity = **velocity;
    let speed = velocity.length();
    let horizontal_speed = velocity.xz().length();
    let camera_position = camera.translation;
    let collisions = names
        .iter_many(state.touching_entities.iter())
        .map(|name| {
            name.name
                .map(|n| format!("{} ({})", name.entity, n))
                .unwrap_or_else(|| format!("{}", name.entity))
        })
        .collect::<Vec<_>>();
    let real_collisions = names
        .iter_many(colliding_entities.iter())
        .map(|name| {
            name.name
                .map(|n| format!("{} ({})", name.entity, n))
                .unwrap_or_else(|| format!("{}", name.entity))
        })
        .collect::<Vec<_>>();
    let ground = state
        .grounded
        .and_then(|ground| names.get(ground.entity).ok())
        .map(|name| {
            name.name
                .map(|n| format!("{} ({})", name.entity, n))
                .unwrap_or(format!("{}", name.entity))
        });
    text.0 = format!(
        "Speed: {speed:.3}\nHorizontal Speed: {horizontal_speed:.3}\nVelocity: [{:.3}, {:.3}, {:.3}]\nCamera Position: [{:.3}, {:.3}, {:.3}]\nCollider Aabb:\n  min:[{:.3}, {:.3}, {:.3}]\n  max:[{:.3}, {:.3}, {:.3}]\nReal Collisions: {:#?}\nCollisions: {:#?}\nGround: {:?}",
        velocity.x,
        velocity.y,
        velocity.z,
        camera_position.x,
        camera_position.y,
        camera_position.z,
        aabb.min.x,
        aabb.min.y,
        aabb.min.z,
        aabb.max.x,
        aabb.max.y,
        aabb.max.z,
        real_collisions,
        collisions,
        ground
    );
}

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub(crate) struct DebugText;

fn setup_ui(mut commands: Commands) {
    commands.spawn((Node::default(), Text::default(), DebugText));
    commands.spawn((
        Node {
            justify_self: JustifySelf::End,
            justify_content: JustifyContent::End,
            ..default()
        },
        Text::new(
            "Controls:\nWASD: move\nSpace: jump\nSpace (hold): autohop\nCtrl: crouch\nEsc: free mouse\nR: reset position",
        ),
    ));
    commands.spawn((
        DebugInput,
        actions!(
            DebugInput[(
                Action::<Reset>::new(),
                bindings![KeyCode::KeyR, GamepadButton::Select],
                Release::default(),
            )]
        ),
    ));
}

#[derive(Component, Default)]
struct DebugInput;

#[derive(Debug, InputAction)]
#[action_output(Vec2)]
struct Reset;

fn reset_player(_fire: On<Fire<Reset>>, mut commands: Commands) {
    commands.run_system_cached(reset_player_inner);
}

fn reset_player_inner(
    world: &mut World,
    mut player: Local<QueryState<(&mut Transform, &mut LinearVelocity), With<CharacterController>>>,
    mut spawner: Local<QueryState<&Transform, Without<CharacterController>>>,
) {
    let component_id = {
        let type_registry = world.resource::<AppTypeRegistry>().read();
        let Some(registration) = type_registry.get_with_short_type_path("SpawnPlayer") else {
            return;
        };
        let type_id = registration.type_id();
        let Some(component_id) = world.components().get_id(type_id) else {
            return;
        };
        component_id
    };
    let mut query = QueryBuilder::<FilteredEntityRef>::new(world)
        .ref_id(component_id)
        .build();
    let Some(spawn_entity) = query.iter(world).map(|e| e.entity()).next() else {
        return;
    };
    let Ok(spawner_transform) = spawner.get(world, spawn_entity).copied() else {
        return;
    };

    let Ok((mut transform, mut velocity)) = player.single_mut(world) else {
        return;
    };
    **velocity = Vec3::ZERO;
    transform.translation = spawner_transform.translation;
}
