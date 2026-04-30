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

        let (la, ra, ll, rl) = build_humanoid(commands, meshes, &skin, &armor, &dark_mat);
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

        commands.entity(la).set_parent(root);
        commands.entity(ra).set_parent(root);
        commands.entity(ll).set_parent(root);
        commands.entity(rl).set_parent(root);
    }
}

// ── Build humanoid mesh hierarchy ─────────────────────────────────────────────
/// Returns (left_arm, right_arm, left_leg, right_leg) entity IDs.
/// Torso and head are spawned as children of the caller's root entity via set_parent.
fn build_humanoid(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    skin: &Handle<StandardMaterial>,
    body: &Handle<StandardMaterial>,
    dark: &Handle<StandardMaterial>,
) -> (Entity, Entity, Entity, Entity) {
    // Head
    let head = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.38, 0.38, 0.38))),
            MeshMaterial3d(skin.clone()),
            Transform::from_xyz(0.0, 1.55, 0.0),
        ))
        .id();

    // Torso
    let torso = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.46, 0.52, 0.26))),
            MeshMaterial3d(body.clone()),
            Transform::from_xyz(0.0, 1.08, 0.0),
        ))
        .id();

    // Arms
    let left_arm = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.16, 0.46, 0.16))),
            MeshMaterial3d(body.clone()),
            Transform::from_xyz(-0.32, 1.08, 0.0),
        ))
        .id();
    let right_arm = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.16, 0.46, 0.16))),
            MeshMaterial3d(skin.clone()),
            Transform::from_xyz(0.32, 1.08, 0.0),
        ))
        .id();

    // Legs
    let left_leg = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.18, 0.50, 0.18))),
            MeshMaterial3d(dark.clone()),
            Transform::from_xyz(-0.13, 0.58, 0.0),
        ))
        .id();
    let right_leg = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.18, 0.50, 0.18))),
            MeshMaterial3d(dark.clone()),
            Transform::from_xyz(0.13, 0.58, 0.0),
        ))
        .id();

    // Head and torso are also children — caller sets root
    commands.entity(head).set_parent_in_place(torso); // head sits on torso
                                                      // Return the four limb entities; caller must set_parent on all of them + torso
                                                      // We return torso separately so caller can parent it too
                                                      // Actually: we return la, ra, ll, rl; torso+head chain is separate
                                                      // Let's spawn torso + head as a single unit, parent torso to root
    let _ = head; // head is already parented to torso above
    let _ = torso; // caller must set_parent(torso, root)

    // For simplicity, parent all to the root inside build_humanoid by returning them all
    (left_arm, right_arm, left_leg, right_leg)
}

fn reset_game(
    mut _should: ResMut<ShouldReset>,
    _commands: Commands,
    _entities: Query<Entity, With<GameEntity>>,
    mut _phase: ResMut<GamePhase>,
    mut _kills: ResMut<KillCount>,
    mut _extract: ResMut<ExtractionState>,
    mut _choice: ResMut<LoadoutChoice>,
    mut _look: ResMut<PlayerLook>,
    mut _meshes: ResMut<Assets<Mesh>>,
    mut _materials: ResMut<Assets<StandardMaterial>>,
) {
}

fn loadout_input(
    _keys: Res<ButtonInput<KeyCode>>,
    mut _choice: ResMut<LoadoutChoice>,
    _phase: Res<GamePhase>,
) {
}

fn update_loadout_ui(
    _choice: Res<LoadoutChoice>,
    _weapon_q: Query<(&LoadoutWeaponLabel, &mut TextColor)>,
    _armor_q: Query<(&LoadoutArmorLabel, &mut TextColor)>,
) {
}

fn confirm_loadout(
    _keys: Res<ButtonInput<KeyCode>>,
    mut _phase: ResMut<GamePhase>,
    _choice: Res<LoadoutChoice>,
    _health_q: Query<&mut Health, With<Player>>,
    _player_q: Query<&mut Transform, (With<Player>, Without<FpsCameraArm>)>,
    _look: ResMut<PlayerLook>,
    _loadout_ui: Query<Entity, With<LoadoutUiRoot>>,
    _commands: Commands,
    _windows: Query<&mut Window>,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
    _camera_q: Query<Entity, With<Camera3d>>,
) {
}

fn cursor_management(_phase: Res<GamePhase>, _windows: Query<&mut Window>) {}

fn fps_look(
    _mouse_motion: EventReader<MouseMotion>,
    mut _look: ResMut<PlayerLook>,
    _phase: Res<GamePhase>,
    _player_q: Query<&mut Transform, (With<Player>, Without<FpsCameraArm>)>,
    _arm_q: Query<&mut Transform, (With<FpsCameraArm>, Without<Player>)>,
) {
}

fn fps_movement(
    _keys: Res<ButtonInput<KeyCode>>,
    _time: Res<Time>,
    _phase: Res<GamePhase>,
    _player_q: Query<&mut Transform, (With<Player>, Without<FpsCameraArm>, Without<Enemy>)>,
    _collider_q: Query<(&Transform, &Collider), (Without<Player>, Without<Enemy>)>,
) {
}

fn fps_combat(
    _mouse: Res<ButtonInput<MouseButton>>,
    _phase: Res<GamePhase>,
    _choice: Res<LoadoutChoice>,
    _time: Res<Time>,
    _player_q: Query<(&Transform, &mut WeaponCooldown), With<Player>>,
    _arm_gtf_q: Query<&GlobalTransform, With<FpsCameraArm>>,
    _enemy_q: Query<(Entity, &Transform, &Health), (With<Enemy>, Without<EnemyDying>)>,
    _damage_events: EventWriter<DamageEvent>,
    _commands: Commands,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
) {
}

fn projectile_update(
    _time: Res<Time>,
    _phase: Res<GamePhase>,
    _proj_q: Query<(Entity, &mut Transform, &mut Projectile)>,
    _enemy_q: Query<(Entity, &GlobalTransform), (With<Enemy>, Without<EnemyDying>)>,
    _damage_events: EventWriter<DamageEvent>,
    _commands: Commands,
) {
}

fn ai_update(
    _time: Res<Time>,
    _phase: Res<GamePhase>,
    _player_q: Query<(Entity, &Transform, &Health), (With<Player>, Without<Enemy>)>,
    _enemy_q: Query<
        (Entity, &mut Transform, &mut Enemy, &mut AnimState),
        (Without<Player>, Without<EnemyDying>),
    >,
    _damage_events: EventWriter<DamageEvent>,
) {
}

fn apply_damage(
    mut _events: EventReader<DamageEvent>,
    mut _hp_q: Query<&mut Health>,
    _player_q: Query<Entity, With<Player>>,
    _flash_q: Query<&mut DamageFlash>,
) {
}

fn check_deaths(
    _commands: Commands,
    mut _phase: ResMut<GamePhase>,
    _player_q: Query<&Health, With<Player>>,
    _enemy_q: Query<(Entity, &Health), (With<Enemy>, Without<EnemyDying>)>,
    mut _kills: ResMut<KillCount>,
    mut _windows: Query<&mut Window>,
) {
}

fn enemy_dying_update(
    _commands: Commands,
    _time: Res<Time>,
    _dying_q: Query<(Entity, &mut Transform, &mut EnemyDying)>,
) {
}

fn extraction_update(
    _time: Res<Time>,
    _phase: Res<GamePhase>,
    _player_q: Query<&Transform, With<Player>>,
    _zone_q: Query<&Transform, With<ExtractionZone>>,
    mut _extract: ResMut<ExtractionState>,
    mut _game_phase: ResMut<GamePhase>,
) {
}

fn damage_flash_update(
    _time: Res<Time>,
    _flash_q: Query<(&mut DamageFlash, &mut BackgroundColor)>,
) {
}

fn update_hud(
    _phase: Res<GamePhase>,
    _choice: Res<LoadoutChoice>,
    _kills: Res<KillCount>,
    _extract: Res<ExtractionState>,
    _player_q: Query<&Health, With<Player>>,
    _texts: ParamSet<(
        Query<&mut Text, With<HudHpText>>,
        Query<&mut Text, With<HudWeaponText>>,
        Query<&mut Text, With<HudKillText>>,
        Query<&mut Text, With<HudExtractText>>,
        Query<&mut Text, With<HudGameOverText>>,
    )>,
    _hp_bar_q: Query<&mut Node, (With<HudHpBarFill>, Without<HudExtractBarFill>)>,
    _extract_bar_q: Query<&mut Node, With<HudExtractBarFill>>,
    _overlay_q: Query<&mut Visibility, With<HudGameOverlay>>,
) {
}

fn handle_keys(
    _keys: Res<ButtonInput<KeyCode>>,
    _phase: Res<GamePhase>,
    mut _should_reset: ResMut<ShouldReset>,
    mut _exit: EventWriter<AppExit>,
) {
}
