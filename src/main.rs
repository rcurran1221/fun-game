use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use std::sync::atomic::{AtomicU64, Ordering};

// ── Constants ─────────────────────────────────────────────────────────────────
const WINDOW_W: f32 = 1280.0;
const WINDOW_H: f32 = 720.0;
const BOUNDS_X: f32 = 27.0;
const BOUNDS_Z_MIN: f32 = -33.0;
const BOUNDS_Z_MAX: f32 = 24.0;
const EYE_HEIGHT: f32 = 1.5;
const PLAYER_RADIUS: f32 = 0.38;
const PLAYER_SPEED: f32 = 5.5;
const MOUSE_SENS: f32 = 0.0018;
const EXTRACT_RADIUS: f32 = 2.5;
const EXTRACT_TIME: f32 = 3.5;
const ENEMY_DETECT: f32 = 9.5;
const ENEMY_LOSE: f32 = 18.0;
const ENEMY_MELEE_RANGE: f32 = 1.6;
const ENEMY_ATK_CD: f32 = 1.4;
const ENEMY_DMG: f32 = 8.0;
const ENEMY_SPEED: f32 = 3.2;
const ENEMY_HP: f32 = 60.0;
const FLASH_DURATION: f32 = 0.18;

// Simple PCG-style RNG (no dependencies needed)
fn rand_f32() -> f32 {
    static SEED: AtomicU64 = AtomicU64::new(987654321);
    let s = SEED
        .load(Ordering::Relaxed)
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    SEED.store(s, Ordering::Relaxed);
    (s >> 33) as f32 / (u32::MAX as f32)
}

// ── Weapon ────────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum WeaponType {
    #[default]
    Sword,
    Bow,
    Magic,
}
impl WeaponType {
    fn name(self) -> &'static str {
        match self {
            Self::Sword => "Sword & Shield",
            Self::Bow => "Bow & Arrow",
            Self::Magic => "Magic Staff",
        }
    }
    fn desc(self) -> &'static str {
        match self {
            Self::Sword => "Melee  12-24 dmg  0.65s cd  Range 2.3m",
            Self::Bow => "Ranged 10-20 dmg  1.1s  cd  Range 40m",
            Self::Magic => "Ranged 16-28 dmg  1.4s  cd  AoE 2.2m",
        }
    }
    fn dmg_range(self) -> (f32, f32) {
        match self {
            Self::Sword => (12.0, 24.0),
            Self::Bow => (10.0, 20.0),
            Self::Magic => (16.0, 28.0),
        }
    }
    fn cooldown(self) -> f32 {
        match self {
            Self::Sword => 0.65,
            Self::Bow => 1.1,
            Self::Magic => 1.4,
        }
    }
    fn is_ranged(self) -> bool {
        !matches!(self, Self::Sword)
    }
    fn proj_speed(self) -> f32 {
        match self {
            Self::Bow => 38.0,
            Self::Magic => 28.0,
            _ => 0.0,
        }
    }
    fn aoe(self) -> f32 {
        match self {
            Self::Magic => 2.2,
            _ => 0.0,
        }
    }
    fn color(self) -> Color {
        match self {
            Self::Sword => Color::srgb(0.75, 0.75, 0.82),
            Self::Bow => Color::srgb(0.55, 0.35, 0.12),
            Self::Magic => Color::srgb(0.45, 0.10, 0.95),
        }
    }
}

// ── Armor ─────────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum ArmorType {
    #[default]
    Steel,
    Dragonhide,
    MageRobes,
}
impl ArmorType {
    fn name(self) -> &'static str {
        match self {
            Self::Steel => "Steel Plate",
            Self::Dragonhide => "Dragonhide",
            Self::MageRobes => "Mage Robes",
        }
    }
    fn desc(self) -> &'static str {
        match self {
            Self::Steel => "100 HP  45% dmg reduction  Best melee def",
            Self::Dragonhide => " 90 HP  30% dmg reduction  Best ranged def",
            Self::MageRobes => " 80 HP  15% dmg reduction  +25% magic dmg",
        }
    }
    fn max_hp(self) -> f32 {
        match self {
            Self::Steel => 100.0,
            Self::Dragonhide => 90.0,
            Self::MageRobes => 80.0,
        }
    }
    fn dmg_taken(self) -> f32 {
        match self {
            Self::Steel => 0.55,
            Self::Dragonhide => 0.70,
            Self::MageRobes => 0.85,
        }
    }
    fn magic_bonus(self) -> f32 {
        match self {
            Self::MageRobes => 1.25,
            _ => 1.0,
        }
    }
    fn color(self) -> Color {
        match self {
            Self::Steel => Color::srgb(0.72, 0.72, 0.78),
            Self::Dragonhide => Color::srgb(0.22, 0.62, 0.16),
            Self::MageRobes => Color::srgb(0.52, 0.06, 0.62),
        }
    }
}

// ── Game phase ────────────────────────────────────────────────────────────────
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
enum GamePhase {
    #[default]
    LoadoutSelect,
    Playing,
    Dead,
    Extracted,
}

// ── Resources ─────────────────────────────────────────────────────────────────
#[derive(Resource, Default)]
struct PlayerLook {
    yaw: f32,
    pitch: f32,
}

#[derive(Resource, Default)]
struct LoadoutChoice {
    weapon: WeaponType,
    armor: ArmorType,
    weapon_idx: usize,
    armor_idx: usize,
}

#[derive(Resource, Default)]
struct ShouldReset(bool);

#[derive(Resource, Default)]
struct KillCount(u32);

#[derive(Resource, Default)]
struct ExtractionState {
    timer: f32,
    active: bool,
}

// ── Events ────────────────────────────────────────────────────────────────────
#[derive(Event)]
struct DamageEvent {
    target: Entity,
    amount: f32,
}

// ── Components ────────────────────────────────────────────────────────────────
#[derive(Component)]
struct Player;

#[derive(Component)]
struct FpsCameraArm; // child of Player; holds pitch rotation

#[derive(Component)]
struct WeaponVisual; // child of Camera; the held weapon mesh

#[derive(Component)]
struct GameEntity; // tagged on everything reset despawns

#[derive(Component)]
struct ExtractionZone;

#[derive(Component, Clone)]
struct Health {
    cur: f32,
    max: f32,
}
impl Health {
    fn new(max: f32) -> Self {
        Self { cur: max, max }
    }
    fn frac(&self) -> f32 {
        (self.cur / self.max).clamp(0.0, 1.0)
    }
}

#[derive(Component, Default)]
struct WeaponCooldown(f32);

#[derive(Component, Clone, Copy, PartialEq, Default)]
enum EnemyState {
    #[default]
    Patrol,
    Chase,
}

#[derive(Component)]
struct Enemy {
    state: EnemyState,
    patrol_origin: Vec3,
    patrol_target: Vec3,
    patrol_timer: f32,
    attack_cd: f32,
}

#[derive(Component)]
struct EnemyDying {
    timer: f32,
}

#[derive(Component)]
struct Projectile {
    vel: Vec3,
    damage: f32,
    aoe_radius: f32,
    lifetime: f32,
}

#[derive(Component, Clone, Copy)]
enum Collider {
    Circle(f32),
    Obb { half_x: f32, half_z: f32 },
}

#[derive(Component)]
struct DamageFlash {
    timer: f32,
}

#[derive(Component, Default, Clone, Copy, PartialEq)]
enum AnimState {
    #[default]
    Idle,
    Walking,
}

#[derive(Component)]
struct AnimTimer(f32);

#[derive(Component)]
struct PlayerLimbs {
    left_arm: Entity,
    right_arm: Entity,
    left_leg: Entity,
    right_leg: Entity,
}

// ── HUD markers ───────────────────────────────────────────────────────────────
#[derive(Component)]
struct HudHpText;
#[derive(Component)]
struct HudHpBarFill;
#[derive(Component)]
struct HudWeaponText;
#[derive(Component)]
struct HudKillText;
#[derive(Component)]
struct HudExtractText;
#[derive(Component)]
struct HudExtractBarFill;
#[derive(Component)]
struct HudGameOverlay;
#[derive(Component)]
struct HudGameOverText;

// ── Loadout UI markers ────────────────────────────────────────────────────────
#[derive(Component)]
struct LoadoutUiRoot;
#[derive(Component)]
struct LoadoutWeaponLabel(usize);
#[derive(Component)]
struct LoadoutArmorLabel(usize);

// ─────────────────────────────────────────────────────────────────────────────
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "OSRS FPS — Extraction".into(),
                resolution: (WINDOW_W, WINDOW_H).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(GamePhase::default())
        .insert_resource(PlayerLook::default())
        .insert_resource(LoadoutChoice::default())
        .insert_resource(ShouldReset::default())
        .insert_resource(KillCount::default())
        .insert_resource(ExtractionState::default())
        .add_event::<DamageEvent>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                reset_game,
                loadout_input,
                update_loadout_ui,
                confirm_loadout,
                cursor_management,
                fps_look,
                fps_movement,
                fps_combat,
                projectile_update,
                ai_update,
                apply_damage,
                check_deaths,
                enemy_dying_update,
                extraction_update,
                damage_flash_update,
                update_hud,
                handle_keys,
            )
                .chain(),
        )
        .run();
}

// ─────────────────────────────────────────────────────────────────────────────
//  Stub systems (filled in subsequent chunks)
// ─────────────────────────────────────────────────────────────────────────────
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Ambient + directional light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 250.0,
    });
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.95, 0.85),
            illuminance: 7000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.3, 0.0)),
    ));

    // Torch point lights at pillar positions
    for [tx, tz] in [
        [-15.0f32, 3.0],
        [15.0, 4.0],
        [-16.0, -6.0],
        [16.0, -5.0],
        [-14.0, -14.0],
        [14.0, -15.0],
        [-9.0, 14.0],
        [9.0, 15.0],
    ] {
        commands.spawn((
            PointLight {
                color: Color::srgb(1.0, 0.68, 0.28),
                intensity: 2000.0,
                radius: 8.0,
                range: 12.0,
                ..default()
            },
            Transform::from_xyz(tx, 2.8, tz),
        ));
    }

    spawn_world(&mut commands, &mut meshes, &mut materials);
    spawn_player(&mut commands);
    spawn_enemies(&mut commands, &mut meshes, &mut materials);
    spawn_loadout_ui(&mut commands);
    spawn_hud(&mut commands);
}

// ── Spawn world ───────────────────────────────────────────────────────────────
fn spawn_world(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    // Ground
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(90.0, 90.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.22, 0.21, 0.19),
            perceptual_roughness: 0.94,
            ..default()
        })),
        GameEntity,
    ));

    // Stone tile path (north-south centre strip)
    let path_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.50, 0.47, 0.42),
        perceptual_roughness: 0.80,
        ..default()
    });
    for zi in -34..=25i32 {
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(2.4, 1.1))),
            MeshMaterial3d(path_mat.clone()),
            Transform::from_xyz(0.0, 0.01, zi as f32),
            GameEntity,
        ));
    }

    // Wall material
    let wall_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.44, 0.40, 0.33),
        perceptual_roughness: 0.88,
        ..default()
    });

    // Outer south wall
    for i in -13..=13i32 {
        let x = i as f32 * 2.2;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(2.1, 3.5, 0.7))),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_xyz(x, 1.75, -35.2),
            GameEntity,
        ));
    }
    // Outer east/west walls
    for i in -16..=12i32 {
        let z = i as f32 * 2.2;
        for sx in [-29.5f32, 29.5] {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.7, 3.5, 2.1))),
                MeshMaterial3d(wall_mat.clone()),
                Transform::from_xyz(sx, 1.75, z),
                GameEntity,
            ));
        }
    }
    // Outer north wall (gap in centre for entrance)
    for i in -13..=13i32 {
        let x = i as f32 * 2.2;
        if x.abs() < 1.5 {
            continue;
        }
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(2.1, 3.5, 0.7))),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_xyz(x, 1.75, 26.2),
            GameEntity,
        ));
    }
    // Inner side walls
    for i in -12..=9i32 {
        let z = i as f32 * 2.2;
        for sx in [-22.5f32, 22.5] {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.7, 3.5, 2.1))),
                MeshMaterial3d(wall_mat.clone()),
                Transform::from_xyz(sx, 1.75, z),
                GameEntity,
            ));
        }
    }
    // Inner north wall
    for i in -9..=9i32 {
        let x = i as f32 * 2.2;
        if x.abs() < 1.5 {
            continue;
        }
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(2.1, 3.5, 0.7))),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_xyz(x, 1.75, 20.5),
            GameEntity,
        ));
    }

    // Invisible boundary colliders (solid walls for player)
    for (x, z, hx, hz) in [
        (0.0f32, -34.5, 30.0, 0.9), // south outer
        (0.0, 26.5, 30.0, 0.9),     // north outer
        (-30.0, -4.0, 0.9, 31.0),   // west outer
        (30.0, -4.0, 0.9, 31.0),    // east outer
        (0.0, 21.0, 23.0, 0.9),     // inner north
        (-23.0, 4.0, 0.9, 25.0),    // inner west
        (23.0, 4.0, 0.9, 25.0),     // inner east
    ] {
        commands.spawn((
            Transform::from_xyz(x, 1.0, z),
            Collider::Obb {
                half_x: hx,
                half_z: hz,
            },
            GameEntity,
        ));
    }

    // Mine support pillars
    let pillar_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.33, 0.30, 0.26),
        perceptual_roughness: 0.90,
        ..default()
    });
    let pillar_cap_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.48, 0.43, 0.36),
        perceptual_roughness: 0.82,
        ..default()
    });
    for [tx, tz] in [
        [-15.0f32, 3.0],
        [-16.0, -6.0],
        [-14.0, -14.0],
        [15.0, 4.0],
        [16.0, -5.0],
        [14.0, -15.0],
        [-9.0, 14.0],
        [9.0, 15.0],
        [-6.0, 10.0],
        [6.0, 11.0],
        [-12.0, -20.0],
        [12.0, -20.0],
        [-17.0, 8.0],
        [17.0, 8.0],
        [0.0, 15.0],
        [-20.0, 0.0],
        [20.0, 0.0],
    ] {
        let ph = 3.2f32;
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.22, ph).mesh().resolution(12))),
            MeshMaterial3d(pillar_mat.clone()),
            Transform::from_xyz(tx, ph / 2.0, tz),
            Collider::Circle(0.45),
            GameEntity,
        ));
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.58, 0.20, 0.58))),
            MeshMaterial3d(pillar_cap_mat.clone()),
            Transform::from_xyz(tx, ph + 0.10, tz),
            GameEntity,
        ));
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.52, 0.14, 0.52))),
            MeshMaterial3d(pillar_cap_mat.clone()),
            Transform::from_xyz(tx, 0.07, tz),
            GameEntity,
        ));
    }

    // Cover crates
    let crate_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.46, 0.32, 0.14),
        perceptual_roughness: 0.88,
        ..default()
    });
    for [cx, cz, cs] in [
        [-7.0f32, -3.0, 0.7],
        [6.0, -6.0, 0.6],
        [-8.0, -12.0, 0.65],
        [9.0, 2.0, 0.55],
        [-3.0, 7.0, 0.5],
        [3.0, -17.0, 0.60],
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(cs, cs * 0.9, cs))),
            MeshMaterial3d(crate_mat.clone()),
            Transform::from_xyz(cx, cs * 0.45, cz),
            Collider::Circle(cs * 0.72),
            GameEntity,
        ));
    }

    // Ruined wall sections (low cover)
    let ruin_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.50, 0.44, 0.32),
        perceptual_roughness: 0.85,
        ..default()
    });
    for [rx, rz, rw, ra] in [
        [-11.0f32, -7.0, 2.2, 0.3],
        [9.0, -11.0, 1.8, -0.4],
        [-4.0, 5.0, 2.5, 0.1],
        [5.0, 1.0, 1.5, 0.8],
        [-7.0, -18.0, 2.0, 0.5],
        [7.0, -14.0, 1.8, -0.2],
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(rw, 1.2, 0.3))),
            MeshMaterial3d(ruin_mat.clone()),
            Transform::from_xyz(rx, 0.6, rz).with_rotation(Quat::from_rotation_y(ra)),
            Collider::Obb {
                half_x: rw / 2.0 + 0.05,
                half_z: 0.28,
            },
            GameEntity,
        ));
    }

    // Ore rocks (visual cover — no mining in FPS mode)
    let rock_positions: &[(f32, f32, f32)] = &[
        (-11.0, -1.0, 0.55),
        (-13.0, -4.0, 0.55),
        (-12.0, -8.0, 0.65),
        (-7.0, 7.0, 0.55),
        (5.0, -3.0, 0.55),
        (-5.0, -6.0, 0.55),
        (10.0, -9.0, 0.65),
        (13.0, -5.0, 0.65),
        (0.5, -15.0, 0.78),
        (5.0, -20.0, 0.78),
        (-9.0, -18.0, 0.78),
        (8.0, 12.0, 0.65),
        (-14.0, 10.0, 0.55),
        (-21.0, -24.0, 0.75),
        (20.0, -27.0, 0.90),
    ];
    let rock_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.24, 0.20),
        perceptual_roughness: 0.92,
        ..default()
    });
    for &(rx, rz, r) in rock_positions {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(r * 1.8, r * 1.4, r * 1.5))),
            MeshMaterial3d(rock_mat.clone()),
            Transform::from_xyz(rx, r * 0.7, rz).with_rotation(Quat::from_rotation_y(rx * 0.5)),
            Collider::Circle(r + 0.15),
            GameEntity,
        ));
    }

    // Extraction zones
    let zone_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.1, 0.9, 0.2, 0.5),
        emissive: LinearRgba::new(0.0, 0.6, 0.1, 1.0),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let pillar_glow = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.85, 0.25),
        emissive: LinearRgba::new(0.0, 0.8, 0.15, 1.0),
        ..default()
    });
    for ex in [-18.0f32, 18.0] {
        let ez = -30.0f32;
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(EXTRACT_RADIUS, 0.08).mesh().resolution(32))),
            MeshMaterial3d(zone_mat.clone()),
            Transform::from_xyz(ex, 0.04, ez),
            ExtractionZone,
            GameEntity,
        ));
        for [px, pz] in [[-1.8f32, -1.8], [1.8, -1.8], [-1.8, 1.8], [1.8, 1.8f32]] {
            commands.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.12, 2.8))),
                MeshMaterial3d(pillar_glow.clone()),
                Transform::from_xyz(ex + px, 1.4, ez + pz),
                GameEntity,
            ));
        }
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(3.2, 0.4, 0.12))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.12, 0.80, 0.22),
                emissive: LinearRgba::new(0.0, 0.5, 0.1, 1.0),
                ..default()
            })),
            Transform::from_xyz(ex, 2.9, ez - 1.9),
            GameEntity,
        ));
    }
}

// ── Spawn player + FPS camera hierarchy ──────────────────────────────────────
fn spawn_player(commands: &mut Commands) {
    // Player root: position + yaw
    let player = commands
        .spawn((
            Transform::from_xyz(0.0, 0.0, 18.0)
                .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
            Visibility::default(),
            Player,
            Health::new(100.0),
            WeaponCooldown::default(),
            AnimState::default(),
            GameEntity,
        ))
        .id();

    // Camera arm: eye-height pivot for pitch
    let arm = commands
        .spawn((
            Transform::from_xyz(0.0, EYE_HEIGHT, 0.0),
            Visibility::default(),
            FpsCameraArm,
            GameEntity,
        ))
        .id();
    commands.entity(arm).set_parent(player);

    // Camera
    let cam = commands
        .spawn((Camera3d::default(), Transform::default(), GameEntity))
        .id();
    commands.entity(cam).set_parent(arm);
}

// ── Spawn enemies ─────────────────────────────────────────────────────────────
fn spawn_enemies(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let spawns: &[(f32, f32)] = &[
        (-8.0, -5.0),
        (7.0, -8.0),
        (0.0, -20.0),
        (-15.0, -15.0),
        (12.0, -20.0),
        (-5.0, 10.0),
        (10.0, 8.0),
        (0.0, -28.0),
    ];

    let skin_color = Color::srgb(0.25, 0.52, 0.18);
    let armor_color = Color::srgb(0.48, 0.36, 0.18);
    let dark = Color::srgb(0.14, 0.12, 0.08);

    for &(ex, ez) in spawns {
        let skin = materials.add(StandardMaterial {
            base_color: skin_color,
            perceptual_roughness: 0.80,
            ..default()
        });
        let armor = materials.add(StandardMaterial {
            base_color: armor_color,
            perceptual_roughness: 0.85,
            ..default()
        });
        let dark_mat = materials.add(StandardMaterial {
            base_color: dark,
            perceptual_roughness: 0.90,
            ..default()
        });

        let (torso, la, ra, ll, rl) = build_humanoid(commands, meshes, &skin, &armor, &dark_mat);
        let origin = Vec3::new(ex, 0.0, ez);

        let root = commands
            .spawn((
                Transform::from_xyz(ex, 0.0, ez),
                Visibility::default(),
                Enemy {
                    state: EnemyState::Patrol,
                    patrol_origin: origin,
                    patrol_target: origin,
                    patrol_timer: 0.0,
                    attack_cd: 0.0,
                },
                Health::new(ENEMY_HP),
                AnimState::default(),
                AnimTimer(0.0),
                PlayerLimbs {
                    left_arm: la,
                    right_arm: ra,
                    left_leg: ll,
                    right_leg: rl,
                },
                GameEntity,
            ))
            .id();

        commands.entity(torso).set_parent(root);
        commands.entity(la).set_parent(root);
        commands.entity(ra).set_parent(root);
        commands.entity(ll).set_parent(root);
        commands.entity(rl).set_parent(root);
    }
}

// ── Build humanoid mesh hierarchy ─────────────────────────────────────────────
/// Returns (torso, left_arm, right_arm, left_leg, right_leg) entity IDs.
/// Caller must set_parent all five onto the enemy root.
fn build_humanoid(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    skin: &Handle<StandardMaterial>,
    body: &Handle<StandardMaterial>,
    dark: &Handle<StandardMaterial>,
) -> (Entity, Entity, Entity, Entity, Entity) {
    // Torso
    let torso = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.46, 0.52, 0.26))),
            MeshMaterial3d(body.clone()),
            Transform::from_xyz(0.0, 1.08, 0.0),
            Visibility::default(),
        ))
        .id();

    // Head — local to torso
    let head = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.38, 0.38, 0.38))),
            MeshMaterial3d(skin.clone()),
            Transform::from_xyz(0.0, 0.45, 0.0),
            Visibility::default(),
        ))
        .id();
    commands.entity(head).set_parent(torso);

    // Arms
    let left_arm = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.16, 0.46, 0.16))),
            MeshMaterial3d(body.clone()),
            Transform::from_xyz(-0.32, 1.08, 0.0),
            Visibility::default(),
        ))
        .id();
    let right_arm = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.16, 0.46, 0.16))),
            MeshMaterial3d(skin.clone()),
            Transform::from_xyz(0.32, 1.08, 0.0),
            Visibility::default(),
        ))
        .id();

    // Legs
    let left_leg = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.18, 0.50, 0.18))),
            MeshMaterial3d(dark.clone()),
            Transform::from_xyz(-0.13, 0.25, 0.0),
            Visibility::default(),
        ))
        .id();
    let right_leg = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.18, 0.50, 0.18))),
            MeshMaterial3d(dark.clone()),
            Transform::from_xyz(0.13, 0.25, 0.0),
            Visibility::default(),
        ))
        .id();

    (torso, left_arm, right_arm, left_leg, right_leg)
}

fn spawn_hud(commands: &mut Commands) {
    // Full-screen damage flash overlay (always present)
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.85, 0.0, 0.0, 0.0)),
        ZIndex(10),
        DamageFlash { timer: 0.0 },
        GameEntity,
    ));

    // Game-over overlay
    let overlay = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::Flex,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)),
            ZIndex(50),
            Visibility::Hidden,
            HudGameOverlay,
            GameEntity,
        ))
        .id();
    let go_text = commands
        .spawn((
            Text::new(""),
            TextFont {
                font_size: 52.0,
                ..default()
            },
            TextColor(Color::WHITE),
            HudGameOverText,
            GameEntity,
        ))
        .id();
    commands.entity(go_text).set_parent(overlay);

    // HP text (bottom-left)
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(20.0),
            bottom: Val::Px(44.0),
            ..default()
        },
        Text::new("HP: 100"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
        HudHpText,
        GameEntity,
    ));

    // HP bar background
    let hp_bg = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                bottom: Val::Px(20.0),
                width: Val::Px(200.0),
                height: Val::Px(18.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.0, 0.0, 0.8)),
            GameEntity,
        ))
        .id();

    // HP bar fill
    let hp_fill = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.8, 0.1, 0.1)),
            HudHpBarFill,
            GameEntity,
        ))
        .id();
    commands.entity(hp_fill).set_parent(hp_bg);

    // Weapon / armor label (bottom-centre)
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            bottom: Val::Px(20.0),
            ..default()
        },
        Text::new(""),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.8, 0.5)),
        HudWeaponText,
        GameEntity,
    ));

    // Controls panel (top-left)
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                top: Val::Px(20.0),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                ..default()
            },
            GameEntity,
        ))
        .with_children(|parent| {
            for line in [
                "WASD       Move",
                "Mouse      Look",
                "LMB        Attack",
                "Walk green  Extract",
                "ESC        Quit",
            ] {
                parent.spawn((
                    Text::new(line),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.85, 0.85, 0.85, 0.65)),
                ));
            }
        });

    // Kill count (top-right)
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(20.0),
            top: Val::Px(20.0),
            ..default()
        },
        Text::new("Kills: 0"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.9, 0.5)),
        HudKillText,
        GameEntity,
    ));

    // Crosshair (centre)
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            margin: UiRect {
                left: Val::Px(-7.0),
                top: Val::Px(-10.0),
                ..default()
            },
            ..default()
        },
        Text::new("+"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::WHITE),
        GameEntity,
    ));

    // Extraction progress text
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            bottom: Val::Px(50.0),
            ..default()
        },
        Text::new(""),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(0.3, 1.0, 0.4)),
        HudExtractText,
        GameEntity,
    ));

    // Extraction bar background
    let ext_bg = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(35.0),
                bottom: Val::Px(75.0),
                width: Val::Px(300.0),
                height: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            GameEntity,
        ))
        .id();

    // Extraction bar fill
    let ext_fill = commands
        .spawn((
            Node {
                width: Val::Percent(0.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.9, 0.3)),
            HudExtractBarFill,
            GameEntity,
        ))
        .id();
    commands.entity(ext_fill).set_parent(ext_bg);
}

fn reset_game(
    mut should: ResMut<ShouldReset>,
    mut commands: Commands,
    entities: Query<Entity, With<GameEntity>>,
    mut phase: ResMut<GamePhase>,
    mut kills: ResMut<KillCount>,
    mut extract: ResMut<ExtractionState>,
    mut choice: ResMut<LoadoutChoice>,
    mut look: ResMut<PlayerLook>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !should.0 {
        return;
    }
    should.0 = false;

    for e in entities.iter() {
        commands.entity(e).despawn_recursive();
    }

    *kills = KillCount::default();
    *extract = ExtractionState::default();
    *choice = LoadoutChoice::default();
    *look = PlayerLook::default();

    spawn_world(&mut commands, &mut meshes, &mut materials);
    spawn_player(&mut commands);
    spawn_enemies(&mut commands, &mut meshes, &mut materials);
    spawn_loadout_ui(&mut commands);
    spawn_hud(&mut commands);

    *phase = GamePhase::LoadoutSelect;
}

fn loadout_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut choice: ResMut<LoadoutChoice>,
    phase: Res<GamePhase>,
) {
    if *phase != GamePhase::LoadoutSelect {
        return;
    }
    if keys.just_pressed(KeyCode::Digit1) {
        choice.weapon = WeaponType::Sword;
        choice.weapon_idx = 0;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        choice.weapon = WeaponType::Bow;
        choice.weapon_idx = 1;
    }
    if keys.just_pressed(KeyCode::Digit3) {
        choice.weapon = WeaponType::Magic;
        choice.weapon_idx = 2;
    }
    if keys.just_pressed(KeyCode::Digit4) {
        choice.armor = ArmorType::Steel;
        choice.armor_idx = 0;
    }
    if keys.just_pressed(KeyCode::Digit5) {
        choice.armor = ArmorType::Dragonhide;
        choice.armor_idx = 1;
    }
    if keys.just_pressed(KeyCode::Digit6) {
        choice.armor = ArmorType::MageRobes;
        choice.armor_idx = 2;
    }
}

fn update_loadout_ui(
    choice: Res<LoadoutChoice>,
    mut color_q: ParamSet<(
        Query<(&LoadoutWeaponLabel, &mut TextColor)>,
        Query<(&LoadoutArmorLabel, &mut TextColor)>,
    )>,
) {
    let sel = Color::srgb(1.0, 0.88, 0.12);
    let dim = Color::srgba(0.7, 0.7, 0.7, 0.55);
    for (lbl, mut col) in color_q.p0().iter_mut() {
        *col = TextColor(if lbl.0 == choice.weapon_idx { sel } else { dim });
    }
    for (lbl, mut col) in color_q.p1().iter_mut() {
        *col = TextColor(if lbl.0 == choice.armor_idx { sel } else { dim });
    }
}

fn confirm_loadout(
    keys: Res<ButtonInput<KeyCode>>,
    mut phase: ResMut<GamePhase>,
    choice: Res<LoadoutChoice>,
    mut health_q: Query<&mut Health, With<Player>>,
    mut player_q: Query<&mut Transform, (With<Player>, Without<FpsCameraArm>)>,
    mut look: ResMut<PlayerLook>,
    loadout_ui: Query<Entity, With<LoadoutUiRoot>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera_q: Query<Entity, With<Camera3d>>,
) {
    if *phase != GamePhase::LoadoutSelect {
        return;
    }
    if !keys.just_pressed(KeyCode::Enter) {
        return;
    }

    // Apply armor HP
    if let Ok(mut hp) = health_q.get_single_mut() {
        let max = choice.armor.max_hp();
        *hp = Health::new(max);
    }
    // Move player to spawn position, facing south into dungeon
    if let Ok(mut tf) = player_q.get_single_mut() {
        tf.translation = Vec3::new(0.0, 0.0, 18.0);
        tf.rotation = Quat::from_rotation_y(std::f32::consts::PI);
    }
    look.yaw = std::f32::consts::PI;
    look.pitch = 0.0;

    // Despawn loadout UI
    for e in loadout_ui.iter() {
        commands.entity(e).despawn_recursive();
    }

    // Spawn held weapon as child of camera
    if let Ok(cam) = camera_q.get_single() {
        spawn_weapon_visual(
            &mut commands,
            &mut meshes,
            &mut materials,
            choice.weapon,
            cam,
        );
    }

    *phase = GamePhase::Playing;
}

fn spawn_weapon_visual(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    weapon: WeaponType,
    cam_entity: Entity,
) {
    // Offset: lower-right corner of view
    let offset = Transform::from_xyz(0.28, -0.22, -0.50);

    match weapon {
        WeaponType::Sword => {
            let blade = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.06, 0.06, 0.52))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(0.75, 0.75, 0.82),
                        metallic: 0.8,
                        perceptual_roughness: 0.25,
                        ..default()
                    })),
                    offset,
                    WeaponVisual,
                    GameEntity,
                ))
                .id();
            let handle = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.07, 0.07, 0.18))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(0.42, 0.24, 0.08),
                        ..default()
                    })),
                    Transform::from_xyz(0.0, 0.0, 0.30),
                ))
                .id();
            commands.entity(handle).set_parent(blade);
            commands.entity(blade).set_parent(cam_entity);
        }
        WeaponType::Bow => {
            let bow = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.05, 0.55, 0.05))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(0.52, 0.32, 0.10),
                        perceptual_roughness: 0.88,
                        ..default()
                    })),
                    offset,
                    WeaponVisual,
                    GameEntity,
                ))
                .id();
            // Arrow nocked
            let arrow = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.02, 0.02, 0.48))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(0.65, 0.55, 0.30),
                        ..default()
                    })),
                    Transform::from_xyz(0.0, 0.0, -0.02),
                ))
                .id();
            commands.entity(arrow).set_parent(bow);
            commands.entity(bow).set_parent(cam_entity);
        }
        WeaponType::Magic => {
            let staff = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.05, 0.05, 0.55))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(0.42, 0.24, 0.08),
                        perceptual_roughness: 0.88,
                        ..default()
                    })),
                    offset,
                    WeaponVisual,
                    GameEntity,
                ))
                .id();
            // Glowing orb at tip
            let orb = commands
                .spawn((
                    Mesh3d(meshes.add(Sphere::new(0.07).mesh().ico(2).unwrap())),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(0.55, 0.10, 0.95),
                        emissive: LinearRgba::new(0.5, 0.05, 1.5, 1.0),
                        ..default()
                    })),
                    Transform::from_xyz(0.0, 0.0, -0.30),
                ))
                .id();
            commands.entity(orb).set_parent(staff);
            commands.entity(staff).set_parent(cam_entity);
        }
    }
}

/// Called from setup and reset — spawns the loadout selection screen UI.
fn spawn_loadout_ui(commands: &mut Commands) {
    let root = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::Flex,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.86)),
            ZIndex(100),
            LoadoutUiRoot,
            GameEntity,
        ))
        .id();

    let panel = commands
        .spawn((
            Node {
                width: Val::Px(720.0),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                padding: UiRect::all(Val::Px(30.0)),
                row_gap: Val::Px(6.0),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.06, 0.04, 0.97)),
            BorderColor(Color::srgb(0.50, 0.40, 0.18)),
        ))
        .id();
    commands.entity(panel).set_parent(root);

    // ── Helper closures ────────────────────────────────────────────────────────
    let title = |s: &str| -> (Text, TextFont, TextColor) {
        (
            Text::new(s.to_string()),
            TextFont {
                font_size: 26.0,
                ..default()
            },
            TextColor(Color::srgb(0.92, 0.78, 0.18)),
        )
    };
    let section = |s: &str| -> (Text, TextFont, TextColor) {
        (
            Text::new(s.to_string()),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(0.75, 0.65, 0.45)),
        )
    };
    let hint = |s: &str| -> (Text, TextFont, TextColor) {
        (
            Text::new(s.to_string()),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(Color::srgba(0.65, 0.65, 0.65, 0.75)),
        )
    };

    // Title
    let t = commands
        .spawn(title("OSRS FPS  —  SELECT YOUR LOADOUT"))
        .id();
    commands.entity(t).set_parent(panel);
    let sep = commands
        .spawn(section("─────────────────────────────────────────────"))
        .id();
    commands.entity(sep).set_parent(panel);

    // Weapons
    let wh = commands.spawn(section("  WEAPON  (press 1 / 2 / 3)")).id();
    commands.entity(wh).set_parent(panel);

    for (i, w) in [WeaponType::Sword, WeaponType::Bow, WeaponType::Magic]
        .iter()
        .enumerate()
    {
        let label_txt = format!("  [{}] {}", i + 1, w.name());
        let lbl = commands
            .spawn((
                Text::new(label_txt),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(if i == 0 {
                    Color::srgb(1.0, 0.88, 0.12)
                } else {
                    Color::srgba(0.7, 0.7, 0.7, 0.55)
                }),
                LoadoutWeaponLabel(i),
            ))
            .id();
        commands.entity(lbl).set_parent(panel);

        let desc = commands.spawn(hint(&format!("       {}", w.desc()))).id();
        commands.entity(desc).set_parent(panel);
    }

    let sep2 = commands
        .spawn(section("─────────────────────────────────────────────"))
        .id();
    commands.entity(sep2).set_parent(panel);

    // Armors
    let ah = commands.spawn(section("  ARMOUR  (press 4 / 5 / 6)")).id();
    commands.entity(ah).set_parent(panel);

    for (i, a) in [
        ArmorType::Steel,
        ArmorType::Dragonhide,
        ArmorType::MageRobes,
    ]
    .iter()
    .enumerate()
    {
        let label_txt = format!("  [{}] {}", i + 4, a.name());
        let lbl = commands
            .spawn((
                Text::new(label_txt),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(if i == 0 {
                    Color::srgb(1.0, 0.88, 0.12)
                } else {
                    Color::srgba(0.7, 0.7, 0.7, 0.55)
                }),
                LoadoutArmorLabel(i),
            ))
            .id();
        commands.entity(lbl).set_parent(panel);

        let desc = commands.spawn(hint(&format!("       {}", a.desc()))).id();
        commands.entity(desc).set_parent(panel);
    }

    let sep3 = commands
        .spawn(section("─────────────────────────────────────────────"))
        .id();
    commands.entity(sep3).set_parent(panel);

    let enter = commands
        .spawn((
            Text::new("  Press ENTER to begin"),
            TextFont {
                font_size: 20.0,
                ..default()
            },
            TextColor(Color::srgb(0.55, 0.90, 0.35)),
        ))
        .id();
    commands.entity(enter).set_parent(panel);
}

fn cursor_management(phase: Res<GamePhase>, mut windows: Query<&mut Window>) {
    let Ok(mut win) = windows.get_single_mut() else {
        return;
    };
    match *phase {
        GamePhase::Playing => {
            win.cursor_options.grab_mode = CursorGrabMode::Locked;
            win.cursor_options.visible = false;
        }
        _ => {
            win.cursor_options.grab_mode = CursorGrabMode::None;
            win.cursor_options.visible = true;
        }
    }
}

fn fps_look(
    mut mouse_motion: EventReader<MouseMotion>,
    mut look: ResMut<PlayerLook>,
    phase: Res<GamePhase>,
    mut player_q: Query<&mut Transform, (With<Player>, Without<FpsCameraArm>)>,
    mut arm_q: Query<&mut Transform, (With<FpsCameraArm>, Without<Player>)>,
) {
    if *phase != GamePhase::Playing {
        mouse_motion.clear();
        return;
    }
    for ev in mouse_motion.read() {
        look.yaw -= ev.delta.x * MOUSE_SENS;
        look.pitch = (look.pitch - ev.delta.y * MOUSE_SENS).clamp(-1.48, 1.48);
    }
    if let Ok(mut tf) = player_q.get_single_mut() {
        tf.rotation = Quat::from_rotation_y(look.yaw);
    }
    if let Ok(mut tf) = arm_q.get_single_mut() {
        tf.rotation = Quat::from_rotation_x(look.pitch);
    }
}

fn fps_movement(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    phase: Res<GamePhase>,
    mut player_q: Query<&mut Transform, (With<Player>, Without<FpsCameraArm>, Without<Enemy>)>,
    collider_q: Query<(&Transform, &Collider), (Without<Player>, Without<Enemy>)>,
) {
    if *phase != GamePhase::Playing {
        return;
    }
    let dt = time.delta_secs();
    let Ok(mut tf) = player_q.get_single_mut() else {
        return;
    };

    let fwd = {
        let f = *tf.forward();
        Vec3::new(f.x, 0.0, f.z).normalize_or_zero()
    };
    let right = {
        let r = *tf.right();
        Vec3::new(r.x, 0.0, r.z).normalize_or_zero()
    };

    let mut dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        dir += fwd;
    }
    if keys.pressed(KeyCode::KeyS) {
        dir -= fwd;
    }
    if keys.pressed(KeyCode::KeyA) {
        dir -= right;
    }
    if keys.pressed(KeyCode::KeyD) {
        dir += right;
    }

    tf.translation += dir.normalize_or_zero() * PLAYER_SPEED * dt;
    tf.translation.y = 0.0;
    tf.translation.x = tf.translation.x.clamp(-BOUNDS_X, BOUNDS_X);
    tf.translation.z = tf.translation.z.clamp(BOUNDS_Z_MIN, BOUNDS_Z_MAX);

    let pos = tf.translation;
    for (ctf, col) in collider_q.iter() {
        push_out(&mut tf.translation, ctf.translation, col);
    }
    let _ = pos;
}

fn push_out(pos: &mut Vec3, col_pos: Vec3, col: &Collider) {
    match *col {
        Collider::Circle(r) => {
            let diff = Vec2::new(pos.x - col_pos.x, pos.z - col_pos.z);
            let dist = diff.length();
            let min_d = r + PLAYER_RADIUS;
            if dist < min_d && dist > 1e-4 {
                let push = diff.normalize() * (min_d - dist);
                pos.x += push.x;
                pos.z += push.y;
            }
        }
        Collider::Obb { half_x, half_z } => {
            let dx = pos.x - col_pos.x;
            let dz = pos.z - col_pos.z;
            let ox = (half_x + PLAYER_RADIUS) - dx.abs();
            let oz = (half_z + PLAYER_RADIUS) - dz.abs();
            if ox > 0.0 && oz > 0.0 {
                if ox < oz {
                    pos.x += ox * dx.signum();
                } else {
                    pos.z += oz * dz.signum();
                }
            }
        }
    }
}

fn fps_combat(
    mouse: Res<ButtonInput<MouseButton>>,
    phase: Res<GamePhase>,
    choice: Res<LoadoutChoice>,
    time: Res<Time>,
    mut player_q: Query<(&Transform, &mut WeaponCooldown), With<Player>>,
    arm_gtf_q: Query<&GlobalTransform, With<FpsCameraArm>>,
    enemy_q: Query<(Entity, &Transform), (With<Enemy>, Without<EnemyDying>)>,
    mut damage_events: EventWriter<DamageEvent>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if *phase != GamePhase::Playing {
        return;
    }
    let Ok((player_tf, mut cd)) = player_q.get_single_mut() else {
        return;
    };
    cd.0 -= time.delta_secs();
    if cd.0 > 0.0 {
        return;
    }
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    cd.0 = choice.weapon.cooldown();

    let (dmg_min, dmg_max) = choice.weapon.dmg_range();
    let mut dmg = dmg_min + (dmg_max - dmg_min) * rand_f32();
    if choice.weapon == WeaponType::Magic {
        dmg *= choice.armor.magic_bonus();
    }

    if choice.weapon.is_ranged() {
        // Shoot a projectile from camera arm
        let Ok(arm_gtf) = arm_gtf_q.get_single() else {
            return;
        };
        let (_, rot, origin) = arm_gtf.to_scale_rotation_translation();
        let fwd = rot * Vec3::NEG_Z;
        let proj_color = choice.weapon.color();
        let emissive = LinearRgba::new(
            proj_color.to_linear().red * 3.0,
            proj_color.to_linear().green * 3.0,
            proj_color.to_linear().blue * 3.0,
            1.0,
        );
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.09).mesh().ico(1).unwrap())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: proj_color,
                emissive,
                ..default()
            })),
            Transform::from_translation(origin + fwd * 0.6),
            Projectile {
                vel: fwd * choice.weapon.proj_speed(),
                damage: dmg,
                aoe_radius: choice.weapon.aoe(),
                lifetime: 3.0,
            },
            GameEntity,
        ));
    } else {
        // Sword melee — hit any enemy within range and in front
        let player_pos = player_tf.translation;
        let player_fwd = {
            let f = *player_tf.forward();
            Vec3::new(f.x, 0.0, f.z).normalize_or_zero()
        };
        for (enemy_e, enemy_tf) in enemy_q.iter() {
            let to_enemy = enemy_tf.translation - player_pos;
            let dist = Vec2::new(to_enemy.x, to_enemy.z).length();
            if dist > 2.5 {
                continue;
            }
            // Must be roughly in front (dot > 0)
            let flat = Vec2::new(to_enemy.x, to_enemy.z).normalize_or_zero();
            let dot = flat.dot(Vec2::new(player_fwd.x, player_fwd.z));
            if dot > 0.25 {
                damage_events.send(DamageEvent {
                    target: enemy_e,
                    amount: dmg,
                });
            }
        }
    }
}

fn projectile_update(
    time: Res<Time>,
    phase: Res<GamePhase>,
    mut proj_q: Query<(Entity, &mut Transform, &mut Projectile)>,
    enemy_q: Query<(Entity, &GlobalTransform), (With<Enemy>, Without<EnemyDying>)>,
    mut damage_events: EventWriter<DamageEvent>,
    mut commands: Commands,
) {
    if *phase != GamePhase::Playing {
        return;
    }
    let dt = time.delta_secs();

    for (proj_e, mut proj_tf, mut proj) in proj_q.iter_mut() {
        proj.lifetime -= dt;
        if proj.lifetime <= 0.0 {
            commands.entity(proj_e).despawn_recursive();
            continue;
        }
        proj_tf.translation += proj.vel * dt;
        proj_tf.translation.y = proj_tf.translation.y.max(0.08);

        // Check hits against enemies
        let mut hit = false;
        for (enemy_e, enemy_gtf) in enemy_q.iter() {
            let dist = (proj_tf.translation - enemy_gtf.translation()).length();
            if dist < 0.65 + proj.aoe_radius {
                damage_events.send(DamageEvent {
                    target: enemy_e,
                    amount: proj.damage,
                });
                hit = true;
                // AoE: also damage nearby enemies
                if proj.aoe_radius > 0.0 {
                    for (other_e, other_gtf) in enemy_q.iter() {
                        if other_e == enemy_e {
                            continue;
                        }
                        let d2 = (proj_tf.translation - other_gtf.translation()).length();
                        if d2 < proj.aoe_radius {
                            damage_events.send(DamageEvent {
                                target: other_e,
                                amount: proj.damage * 0.6,
                            });
                        }
                    }
                }
                break;
            }
        }
        if hit {
            commands.entity(proj_e).despawn_recursive();
        }
    }
}

fn apply_damage(
    mut events: EventReader<DamageEvent>,
    mut hp_q: Query<&mut Health>,
    player_q: Query<Entity, With<Player>>,
    mut flash_q: Query<&mut DamageFlash>,
) {
    for ev in events.read() {
        if let Ok(mut hp) = hp_q.get_mut(ev.target) {
            hp.cur -= ev.amount;
            // If target is player, trigger red flash
            if let Ok(player_e) = player_q.get_single() {
                if ev.target == player_e {
                    for mut flash in flash_q.iter_mut() {
                        flash.timer = FLASH_DURATION;
                    }
                }
            }
        }
    }
}

fn ai_update(
    time: Res<Time>,
    phase: Res<GamePhase>,
    player_q: Query<(Entity, &Transform, &Health), (With<Player>, Without<Enemy>)>,
    mut enemy_q: Query<
        (Entity, &mut Transform, &mut Enemy, &mut AnimState),
        (Without<Player>, Without<EnemyDying>),
    >,
    mut damage_events: EventWriter<DamageEvent>,
) {
    if *phase != GamePhase::Playing {
        return;
    }
    let dt = time.delta_secs();
    let Ok((player_e, player_tf, player_hp)) = player_q.get_single() else {
        return;
    };
    if player_hp.cur <= 0.0 {
        return;
    }

    for (_enemy_e, mut etf, mut enemy, mut anim) in enemy_q.iter_mut() {
        let to_player = player_tf.translation - etf.translation;
        let dist_xz = Vec2::new(to_player.x, to_player.z).length();

        match enemy.state {
            EnemyState::Patrol => {
                if dist_xz < ENEMY_DETECT {
                    enemy.state = EnemyState::Chase;
                    continue;
                }
                // Wander toward patrol_target
                enemy.patrol_timer -= dt;
                if enemy.patrol_timer <= 0.0 {
                    enemy.patrol_timer = 2.0 + rand_f32() * 2.5;
                    let angle = rand_f32() * std::f32::consts::TAU;
                    let radius = rand_f32() * 4.0;
                    enemy.patrol_target = enemy.patrol_origin
                        + Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
                }
                let to_t = enemy.patrol_target - etf.translation;
                let d = Vec2::new(to_t.x, to_t.z).length();
                if d > 0.4 {
                    let dir = Vec3::new(to_t.x, 0.0, to_t.z).normalize_or_zero();
                    etf.translation += dir * (ENEMY_SPEED * 0.5) * dt;
                    etf.translation.y = 0.0;
                    etf.look_to(dir, Vec3::Y);
                    *anim = AnimState::Walking;
                } else {
                    *anim = AnimState::Idle;
                }
            }
            EnemyState::Chase => {
                if dist_xz > ENEMY_LOSE {
                    enemy.state = EnemyState::Patrol;
                    continue;
                }
                if dist_xz > ENEMY_MELEE_RANGE {
                    let dir = Vec3::new(to_player.x, 0.0, to_player.z).normalize_or_zero();
                    etf.translation += dir * ENEMY_SPEED * dt;
                    etf.translation.y = 0.0;
                    etf.look_to(dir, Vec3::Y);
                    *anim = AnimState::Walking;
                } else {
                    *anim = AnimState::Idle;
                    enemy.attack_cd -= dt;
                    if enemy.attack_cd <= 0.0 {
                        enemy.attack_cd = ENEMY_ATK_CD;
                        damage_events.send(DamageEvent {
                            target: player_e,
                            amount: ENEMY_DMG * (0.7 + rand_f32() * 0.6),
                        });
                    }
                }
            }
        }
    }
}

fn check_deaths(
    mut commands: Commands,
    mut phase: ResMut<GamePhase>,
    player_q: Query<&Health, With<Player>>,
    enemy_q: Query<(Entity, &Health), (With<Enemy>, Without<EnemyDying>)>,
    mut kills: ResMut<KillCount>,
    mut windows: Query<&mut Window>,
) {
    // Player death
    if let Ok(hp) = player_q.get_single() {
        if hp.cur <= 0.0 && *phase == GamePhase::Playing {
            *phase = GamePhase::Dead;
            if let Ok(mut win) = windows.get_single_mut() {
                win.cursor_options.grab_mode = CursorGrabMode::None;
                win.cursor_options.visible = true;
            }
        }
    }
    // Enemy deaths
    for (e, hp) in enemy_q.iter() {
        if hp.cur <= 0.0 {
            kills.0 += 1;
            commands.entity(e).insert(EnemyDying { timer: 0.35 });
        }
    }
}

fn enemy_dying_update(
    mut commands: Commands,
    time: Res<Time>,
    mut dying_q: Query<(Entity, &mut Transform, &mut EnemyDying)>,
) {
    let dt = time.delta_secs();
    for (e, mut tf, mut dying) in dying_q.iter_mut() {
        dying.timer -= dt;
        let s = (dying.timer / 0.35).clamp(0.0, 1.0);
        tf.scale = Vec3::splat(s);
        if dying.timer <= 0.0 {
            commands.entity(e).despawn_recursive();
        }
    }
}

fn damage_flash_update(
    time: Res<Time>,
    mut flash_q: Query<(&mut DamageFlash, &mut BackgroundColor)>,
) {
    let dt = time.delta_secs();
    for (mut flash, mut bg) in flash_q.iter_mut() {
        flash.timer -= dt;
        let alpha = (flash.timer / FLASH_DURATION).clamp(0.0, 1.0) * 0.45;
        *bg = BackgroundColor(Color::srgba(0.85, 0.0, 0.0, alpha));
    }
}

fn extraction_update(
    time: Res<Time>,
    mut phase: ResMut<GamePhase>,
    player_q: Query<&Transform, With<Player>>,
    zone_q: Query<&Transform, With<ExtractionZone>>,
    mut extract: ResMut<ExtractionState>,
) {
    if *phase != GamePhase::Playing {
        return;
    }
    let Ok(player_tf) = player_q.get_single() else {
        return;
    };
    let pos = player_tf.translation;

    let mut on_zone = false;
    for zone_tf in zone_q.iter() {
        let d = Vec2::new(pos.x - zone_tf.translation.x, pos.z - zone_tf.translation.z).length();
        if d < EXTRACT_RADIUS {
            on_zone = true;
            break;
        }
    }

    if on_zone {
        extract.timer += time.delta_secs();
        extract.active = true;
        if extract.timer >= EXTRACT_TIME {
            *phase = GamePhase::Extracted;
        }
    } else {
        extract.active = false;
        extract.timer = (extract.timer - time.delta_secs()).max(0.0);
    }
}

fn update_hud(
    phase: Res<GamePhase>,
    choice: Res<LoadoutChoice>,
    kills: Res<KillCount>,
    extract: Res<ExtractionState>,
    player_q: Query<&Health, With<Player>>,
    mut texts: ParamSet<(
        Query<&mut Text, With<HudHpText>>,
        Query<&mut Text, With<HudWeaponText>>,
        Query<&mut Text, With<HudKillText>>,
        Query<&mut Text, With<HudExtractText>>,
        Query<&mut Text, With<HudGameOverText>>,
    )>,
    mut hp_bar_q: Query<&mut Node, (With<HudHpBarFill>, Without<HudExtractBarFill>)>,
    mut extract_bar_q: Query<&mut Node, With<HudExtractBarFill>>,
    mut overlay_q: Query<&mut Visibility, With<HudGameOverlay>>,
) {
    let (hp_cur, hp_frac) = player_q
        .get_single()
        .map(|h| (h.cur.max(0.0), h.frac()))
        .unwrap_or((0.0, 0.0));

    // HP text
    {
        let mut q = texts.p0();
        if let Ok(mut t) = q.get_single_mut() {
            t.0 = format!("HP: {:.0}", hp_cur);
        }
    }
    // HP bar width
    if let Ok(mut node) = hp_bar_q.get_single_mut() {
        node.width = Val::Percent(hp_frac * 100.0);
    }

    // Weapon / armor label
    {
        let mut q = texts.p1();
        if let Ok(mut t) = q.get_single_mut() {
            t.0 = format!("{} / {}", choice.weapon.name(), choice.armor.name());
        }
    }

    // Kill count
    {
        let mut q = texts.p2();
        if let Ok(mut t) = q.get_single_mut() {
            t.0 = format!("Kills: {}", kills.0);
        }
    }

    // Extraction text + bar
    {
        let mut q = texts.p3();
        if let Ok(mut t) = q.get_single_mut() {
            t.0 = if extract.active {
                format!(
                    "EXTRACTING... {:.1}s",
                    (EXTRACT_TIME - extract.timer).max(0.0)
                )
            } else {
                String::new()
            };
        }
    }
    if let Ok(mut node) = extract_bar_q.get_single_mut() {
        node.width = Val::Percent((extract.timer / EXTRACT_TIME).clamp(0.0, 1.0) * 100.0);
    }

    // Game-over overlay
    let show = matches!(*phase, GamePhase::Dead | GamePhase::Extracted);
    for mut vis in overlay_q.iter_mut() {
        *vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    {
        let mut q = texts.p4();
        if let Ok(mut t) = q.get_single_mut() {
            t.0 = match *phase {
                GamePhase::Dead => "YOU DIED\n\nPress R to try again".into(),
                GamePhase::Extracted => "EXTRACTION SUCCESSFUL!\n\nPress R to play again".into(),
                _ => String::new(),
            };
        }
    }
}

fn handle_keys(
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<GamePhase>,
    mut should_reset: ResMut<ShouldReset>,
    mut exit: EventWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.send(AppExit::Success);
    }
    if keys.just_pressed(KeyCode::KeyR) {
        if matches!(*phase, GamePhase::Dead | GamePhase::Extracted) {
            should_reset.0 = true;
        }
    }
}
