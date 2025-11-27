use avian3d::prelude::*;
use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    gltf::GltfPlugin,
    input::common_conditions::input_just_pressed,
    light::CascadeShadowConfigBuilder,
    log::{LogPlugin, tracing_subscriber::field::MakeExt},
    pbr::Atmosphere,
    prelude::*,
    scene::SceneInstanceReady,
    window::{CursorGrabMode, CursorOptions, WindowResolution},
};
use bevy_ahoy::{kcc::CharacterControllerState, prelude::*};
use bevy_enhanced_input::prelude::{Release, *};
use bevy_mod_mipmap_generator::{MipmapGeneratorPlugin, generate_mipmaps};
use bevy_trenchbroom::{physics::SceneCollidersReady, prelude::*};
use bevy_trenchbroom_avian::AvianPhysicsBackend;
use core::ops::Deref;

use crate::util::ExampleUtilPlugin;

mod util;

fn main() -> AppExit {
    App::new()
        .add_plugins((
            DefaultPlugins.set(GltfPlugin {
                use_model_forward_direction: true,
                ..default()
            }),
            PhysicsPlugins::default(),
            EnhancedInputPlugin,
            AhoyPlugin::default(),
            TrenchBroomPlugins(
                TrenchBroomConfig::new("bevy_ahoy").default_solid_spawn_hooks(|| {
                    SpawnHooks::new()
                        .convex_collider()
                        .smooth_by_default_angle()
                }),
            ),
            TrenchBroomPhysicsPlugin::new(AvianPhysicsBackend),
            ExampleUtilPlugin,
        ))
        .add_input_context::<PlayerInput>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                capture_cursor.run_if(input_just_pressed(MouseButton::Left)),
                release_cursor.run_if(input_just_pressed(KeyCode::Escape)),
            ),
        )
        .add_observer(spawn_player)
        .run()
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    commands
        .spawn(SceneRoot(assets.load("maps/playground.map#Scene")))
        .observe(tweak_materials);
    commands.spawn((
        Camera3d::default(),
        EnvironmentMapLight {
            diffuse_map: assets.load("environment_maps/voortrekker_interior_1k_diffuse.ktx2"),
            specular_map: assets.load("environment_maps/voortrekker_interior_1k_specular.ktx2"),
            intensity: 2000.0,
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: 70.0_f32.to_radians(),
            ..default()
        }),
        Atmosphere::EARTH,
    ));
    commands.spawn((
        Transform::from_xyz(0.0, 1.0, 0.0).looking_at(vec3(1.0, -2.0, -2.0), Vec3::Y),
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            maximum_distance: 500.0,
            overlap_proportion: 0.5,
            ..default()
        }
        .build(),
    ));
}

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
struct Player;

#[point_class(base(Transform, Visibility))]
struct SpawnPlayer;

fn spawn_player(
    insert: On<Insert, SpawnPlayer>,
    players: Query<Entity, With<Player>>,
    spawner: Query<&Transform>,
    camera: Single<Entity, With<Camera3d>>,
    mut commands: Commands,
) {
    for player in players {
        // Respawn the player on hot-reloads
        commands.entity(player).despawn();
    }
    let Ok(transform) = spawner.get(insert.entity).copied() else {
        return;
    };
    let player = commands
        .spawn((
            Player,
            transform,
            PlayerInput,
            CharacterController::default(),
            RigidBody::Kinematic,
            Collider::cylinder(0.7, 1.8),
            // For debugging
            CollidingEntities::default(),
        ))
        .id();
    commands
        .entity(camera.into_inner())
        .insert(CharacterControllerCameraOf(player));
}

fn tweak_materials(
    ready: On<SceneInstanceReady>,
    children: Query<&Children>,
    materials: Query<&MeshMaterial3d<StandardMaterial>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
) {
    for mat in materials.iter_many(children.iter_descendants(ready.entity)) {
        let mat = material_assets.get_mut(mat).unwrap();
        mat.perceptual_roughness = 0.9;
    }
}

#[derive(Component, Default)]
#[component(on_add = PlayerInput::on_add)]
pub(crate) struct PlayerInput;

impl PlayerInput {
    fn on_add(mut world: DeferredWorld, ctx: HookContext) {
        world
            .commands()
            .entity(ctx.entity)
            .insert(actions!(PlayerInput[
                (
                    Action::<Movement>::new(),
                    DeadZone::default(),
                    Bindings::spawn((
                        Cardinal::wasd_keys(),
                        Axial::left_stick()
                    ))
                ),
                (
                    Action::<Jump>::new(),
                    bindings![KeyCode::Space,  GamepadButton::South],
                ),
                (
                    Action::<Crouch>::new(),
                    bindings![KeyCode::ControlLeft, GamepadButton::LeftTrigger],
                ),
                (
                    Action::<RotateCamera>::new(),
                    Scale::splat(0.1),
                    Bindings::spawn((
                        Spawn(Binding::mouse_motion()),
                        Axial::right_stick()
                    ))
                ),
            ]));
    }
}

#[point_class(base(Transform, Visibility), model("models/cube.glb"))]
#[component(on_add = on_add_prop::<Self>)]
#[derive(Default, Deref)]
struct Cube {
    dynamic: bool,
}

#[point_class(base(Transform, Visibility), model("models/cone.glb"))]
#[component(on_add = on_add_prop::<Self>)]
#[derive(Default, Deref)]
struct Cone {
    dynamic: bool,
}

#[point_class(base(Transform, Visibility), model("models/cylinder.glb"))]
#[component(on_add = on_add_prop::<Self>)]
#[derive(Default, Deref)]
struct Cylinder {
    dynamic: bool,
}

#[point_class(base(Transform, Visibility), model("models/capsule.glb"))]
#[component(on_add = on_add_prop::<Self>)]
#[derive(Default, Deref)]
struct Capsule {
    dynamic: bool,
}

#[point_class(base(Transform, Visibility), model("models/sphere.glb"))]
#[component(on_add = on_add_prop::<Self>)]
#[derive(Default, Deref)]
struct Sphere {
    dynamic: bool,
}

fn on_add_prop<T: QuakeClass + Deref<Target = bool>>(mut world: DeferredWorld, ctx: HookContext) {
    if world.is_scene_world() {
        return;
    }
    let dynamic = *world.get::<T>(ctx.entity).unwrap().deref();
    let assets = world.resource::<AssetServer>().clone();
    world
        .commands()
        .entity(ctx.entity)
        .insert((
            SceneRoot(
                assets
                    .load(GltfAssetLabel::Scene(0).from_asset(T::CLASS_INFO.model_path().unwrap())),
            ),
            ColliderConstructorHierarchy::new(ColliderConstructor::ConvexHullFromMesh),
            if dynamic {
                RigidBody::Dynamic
            } else {
                RigidBody::Static
            },
            TransformInterpolation,
        ))
        .observe(|ready: On<SceneCollidersReady>, mut commands: Commands| {
            for collider in &ready.collider_entities {
                commands.entity(*collider).insert(ColliderDensity(100.0));
            }
        });
}

fn capture_cursor(mut cursor: Single<&mut CursorOptions>) {
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
}

fn release_cursor(mut cursor: Single<&mut CursorOptions>) {
    cursor.visible = true;
    cursor.grab_mode = CursorGrabMode::None;
}
