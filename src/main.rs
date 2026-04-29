use bevy::prelude::*;

// ── Constants ─────────────────────────────────────────────────
const WINDOW_W: f32 = 1100.0;
const WINDOW_H: f32 = 760.0;
const PLAYER_SPEED: f32 = 4.5;
const ENEMY_SPEED: f32 = 2.8;
const ENEMY_CHASE: f32 = 3.6;
const ATTACK_RANGE: f32 = 2.3;
const INTERACT_DIST: f32 = 2.0;
const DETECT_RADIUS: f32 = 7.5;
const LOSE_RADIUS: f32 = 22.0;
const PLAYER_HP: f32 = 100.0;
const ENEMY_HP: f32 = 60.0;
const PLAYER_DMG: f32 = 30.0;
const ENEMY_DMG: f32 = 14.0;
const PLAYER_ATK_CD: f32 = 0.70;
const ENEMY_ATK_CD: f32 = 1.50;
const EXTRACT_TIME: f32 = 3.5;
const EXTRACT_RADIUS: f32 = 2.5;
const CAM_OFFSET: Vec3 = Vec3::new(0.0, 20.0, 16.0);
const BOUNDS_X: f32 = 28.0;
const BOUNDS_Z_MIN: f32 = -34.0;
const BOUNDS_Z_MAX: f32 = 25.0;
const TILE_SIZE: f32 = 1.0;
const CLICK_ENEMY_RADIUS: f32 = 1.4;
const CLICK_ROCK_RADIUS: f32 = 1.1;
const MOVER_RADIUS: f32 = 0.30;

// ── Ore type ──────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum OreType {
    Copper,
    Tin,
    Iron,
    Coal,
    Adamantite,
    Rune,
}
impl OreType {
    fn name(self) -> &'static str {
        match self {
            Self::Copper => "Copper",
            Self::Tin => "Tin",
            Self::Iron => "Iron",
            Self::Coal => "Coal",
            Self::Adamantite => "Adamantite",
            Self::Rune => "Runite",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Copper => "Copper ore",
            Self::Tin => "Tin ore",
            Self::Iron => "Iron ore",
            Self::Coal => "Coal",
            Self::Adamantite => "Adamantite ore",
            Self::Rune => "Runite ore",
        }
    }
    fn mine_time(self) -> f32 {
        match self {
            Self::Copper | Self::Tin => 2.2,
            Self::Iron => 3.5,
            Self::Coal => 5.0,
            Self::Adamantite => 7.5,
            Self::Rune => 12.0,
        }
    }
    fn respawn_time(self) -> f32 {
        match self {
            Self::Copper | Self::Tin => 12.0,
            Self::Iron => 20.0,
            Self::Coal => 30.0,
            Self::Adamantite => 60.0,
            Self::Rune => 90.0,
        }
    }
    fn full_color(self) -> Color {
        match self {
            Self::Copper => Color::srgb(0.76, 0.40, 0.16),
            Self::Tin => Color::srgb(0.60, 0.66, 0.68),
            Self::Iron => Color::srgb(0.50, 0.23, 0.14),
            Self::Coal => Color::srgb(0.14, 0.12, 0.11),
            Self::Adamantite => Color::srgb(0.08, 0.26, 0.12),
            Self::Rune => Color::srgb(0.10, 0.22, 0.35),
        }
    }
    fn vein_color(self) -> Color {
        match self {
            Self::Copper => Color::srgb(0.90, 0.55, 0.20),
            Self::Tin => Color::srgb(0.80, 0.84, 0.88),
            Self::Iron => Color::srgb(0.70, 0.38, 0.28),
            Self::Coal => Color::srgb(0.30, 0.28, 0.26),
            Self::Adamantite => Color::srgb(0.22, 0.65, 0.28),
            Self::Rune => Color::srgb(0.30, 0.60, 0.90),
        }
    }
    fn value(self) -> u32 {
        match self {
            Self::Copper | Self::Tin => 10,
            Self::Iron => 25,
            Self::Coal => 50,
            Self::Adamantite => 200,
            Self::Rune => 500,
        }
    }
    fn xp(self) -> u32 {
        match self {
            Self::Copper | Self::Tin => 17,
            Self::Iron => 35,
            Self::Coal => 50,
            Self::Adamantite => 95,
            Self::Rune => 125,
        }
    }
}

// ── Anim ──────────────────────────────────────────────────────
#[derive(Component, PartialEq, Clone, Copy, Default)]
enum AnimState {
    #[default]
    Idle,
    Walking,
    Mining,
}

// ── Enemy AI state ────────────────────────────────────────────
#[derive(Clone)]
enum EnemyAi {
    Patrolling { idx: usize, wait: f32 },
    Chasing { lose_timer: f32 },
    Attacking { cooldown: f32 },
    Dead,
}

// ── Game phase ────────────────────────────────────────────────
#[derive(Resource, PartialEq, Default)]
enum GamePhase {
    #[default]
    Playing,
    Dead,
    Extracted,
}

// ── Components ────────────────────────────────────────────────
#[derive(Component)]
struct Player;
#[derive(Component)]
struct Enemy {
    ai: EnemyAi,
    patrol: Vec<Vec3>,
    hp_fill: Entity,
}
#[derive(Component)]
struct Health {
    cur: f32,
    max: f32,
}
#[derive(Component)]
struct AttackCooldown(f32);
#[derive(Component)]
struct Rock {
    ore: OreType,
    depleted: bool,
    respawn_timer: f32,
    full_mat: Handle<StandardMaterial>,
    depl_mat: Handle<StandardMaterial>,
}
#[derive(Component)]
struct ExtractionZone;
#[derive(Component)]
struct GameEntity; // all entities tagged → despawn on reset

#[derive(Component)]
enum Collider {
    Circle(f32),
    Obb { half_x: f32, half_z: f32 },
}

// Limbs
#[derive(Component)]
struct PlayerLimbs {
    head: Entity,
    torso: Entity,
    left_arm: Entity,
    right_arm: Entity,
    left_leg: Entity,
    right_leg: Entity,
}
#[derive(Component, Default)]
struct AnimTimer(f32);
#[derive(Component, Default)]
struct SwingTimer(f32);

// UI
#[derive(Component)]
struct HpBarFill;
#[derive(Component)]
struct HpBarText;
#[derive(Component)]
struct OreText;
#[derive(Component)]
struct StatusText;
#[derive(Component)]
struct ExtractBar;
#[derive(Component)]
struct ExtractBarFill;
#[derive(Component)]
struct GameOverlay;
#[derive(Component)]
struct GameOverTitle;
#[derive(Component)]
struct DamageFlash;
#[derive(Component)]
struct MiningBarFill;
#[derive(Component)]
struct ActionStatePanel;
#[derive(Component)]
struct ActionStateLabel;
#[derive(Component)]
struct TargetIndicator;

// Floating damage / XP splat (world-projected UI node)
#[derive(Component)]
struct FloatingSplat {
    world_pos: Vec3,
    timer: f32,
    max_time: f32,
    base_color: Color,
}

// Chat box line index (0 = newest / bottom, N-1 = oldest / top)
#[derive(Component)]
struct ChatLine(usize);

// ── Chat log ──────────────────────────────────────────────────
#[derive(Clone, Copy)]
enum ChatColor {
    Game,    // cyan  – standard game messages
    Damage,  // red   – combat damage
    Xp,      // green – XP gain
    LevelUp, // gold  – level up
}
impl ChatColor {
    fn color(self) -> Color {
        match self {
            Self::Game => Color::srgb(0.0, 0.85, 0.85),
            Self::Damage => Color::srgb(1.0, 0.32, 0.32),
            Self::Xp => Color::srgb(0.45, 1.0, 0.45),
            Self::LevelUp => Color::srgb(1.0, 0.85, 0.0),
        }
    }
}

const CHAT_LINES: usize = 7;

#[derive(Resource, Default)]
struct ChatLog {
    messages: Vec<(String, Color)>,
}
impl ChatLog {
    fn push(&mut self, msg: impl Into<String>, kind: ChatColor) {
        self.messages.push((msg.into(), kind.color()));
        if self.messages.len() > CHAT_LINES {
            self.messages.remove(0);
        }
    }
}

// ── Resources ─────────────────────────────────────────────────
#[derive(Resource, Default)]
struct Inventory {
    copper: u32,
    tin: u32,
    iron: u32,
    coal: u32,
    adamantite: u32,
    rune: u32,
}
impl Inventory {
    fn add(&mut self, o: OreType) {
        match o {
            OreType::Copper => self.copper += 1,
            OreType::Tin => self.tin += 1,
            OreType::Iron => self.iron += 1,
            OreType::Coal => self.coal += 1,
            OreType::Adamantite => self.adamantite += 1,
            OreType::Rune => self.rune += 1,
        }
    }
    fn total(&self) -> u32 {
        self.copper + self.tin + self.iron + self.coal + self.adamantite + self.rune
    }
    fn value(&self) -> u32 {
        self.copper * 10
            + self.tin * 10
            + self.iron * 25
            + self.coal * 50
            + self.adamantite * 200
            + self.rune * 500
    }
    fn clear(&mut self) {
        *self = Self::default();
    }
}

#[derive(Resource, Default)]
struct PlayerStats {
    mining_xp: u32,
}
impl PlayerStats {
    fn level(&self) -> u32 {
        1 + (self.mining_xp / 83).min(98)
    }
}

#[derive(Resource)]
enum PlayerAction {
    Free,
    Mining {
        target: Entity,
        progress: f32,
        total: f32,
        ore: OreType,
    },
    Extracting {
        progress: f32,
    },
}

#[derive(Resource, Default)]
struct ShouldReset(bool);

#[derive(Resource, Clone, Default)]
enum PlayerTarget {
    #[default]
    None,
    Move(Vec3),
    Attack(Entity),
    Mine(Entity),
}

// Tracks spawned click/target indicator entities so they can be repositioned or despawned.
#[derive(Resource, Default)]
struct ClickIndicators {
    move_ent: Option<Entity>,
    target_ent: Option<Entity>,
    /// Which entity the target_ent is currently attached to (for change detection).
    tracked: Option<Entity>,
}

// ── Events ────────────────────────────────────────────────────
#[derive(Event)]
struct DamageEvent {
    target: Entity,
    amount: f32,
}

// ─────────────────────────────────────────────────────────────
//  Main
// ─────────────────────────────────────────────────────────────
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Extraction Mining".into(),
                resolution: (WINDOW_W, WINDOW_H).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.06, 0.05, 0.04)))
        .insert_resource(GamePhase::Playing)
        .insert_resource(PlayerAction::Free)
        .insert_resource(Inventory::default())
        .insert_resource(PlayerStats::default())
        .insert_resource(ShouldReset::default())
        .insert_resource(PlayerTarget::default())
        .insert_resource(ClickIndicators::default())
        .insert_resource(ChatLog::default())
        .add_event::<DamageEvent>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                reset_game,
                handle_click,
                player_walk,
                update_indicators,
                player_combat_mine,
                ai_update,
                resolve_collisions,
                apply_damage,
                check_deaths,
                extraction_update,
                animate_characters,
                update_enemy_hp_bars,
                camera_follow,
                rock_respawn,
                damage_flash_update,
                update_hud,
                update_chat_log,
                update_splats,
                handle_game_over_input,
            )
                .chain(),
        )
        .run();
}

// ─────────────────────────────────────────────────────────────
//  Setup
// ─────────────────────────────────────────────────────────────
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.55, 0.48, 0.38),
        brightness: 180.0,
    });

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 20.0, 21.5).looking_at(Vec3::new(0.0, 0.0, 4.0), Vec3::Y),
        GameEntity,
    ));

    // Overhead directional (simulates a dim underground sky)
    commands.spawn((
        DirectionalLight {
            illuminance: 6_000.0,
            shadows_enabled: true,
            color: Color::srgb(0.88, 0.78, 0.62),
            ..default()
        },
        Transform::from_xyz(-8.0, 14.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
        GameEntity,
    ));
    // Warm torch fill lights scattered across the guild
    for &[lx, lz] in &[
        [-15.0f32, 3.0],
        [15.0, 4.0],
        [-9.0, -10.0],
        [9.0, -10.0],
        [0.0, -20.0],
        [-6.0, 10.0],
        [6.0, 11.0],
    ] {
        commands.spawn((
            PointLight {
                intensity: 14_000.0,
                color: Color::srgb(1.0, 0.62, 0.18),
                range: 14.0,
                ..default()
            },
            Transform::from_xyz(lx, 2.5, lz),
            GameEntity,
        ));
    }

    spawn_world(&mut commands, &mut meshes, &mut materials);
    spawn_player(&mut commands, &mut meshes, &mut materials);
    spawn_enemies(&mut commands, &mut meshes, &mut materials);
    spawn_hud(&mut commands);
}

// ─────────────────────────────────────────────────────────────
//  World
// ─────────────────────────────────────────────────────────────
fn spawn_world(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    // Ground — stone cave floor
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(90.0, 90.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.30, 0.28, 0.25),
            perceptual_roughness: 0.97,
            ..default()
        })),
        GameEntity,
    ));

    // Stone tile path (north-south)
    for zi in -34..=25i32 {
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(2.4, 1.1))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.42, 0.39, 0.34),
                perceptual_roughness: 0.85,
                ..default()
            })),
            Transform::from_xyz(0.0, 0.01, zi as f32),
            GameEntity,
        ));
    }

    // Stone wall border — rectangular blocks like a guild building
    let wall_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.34, 0.28),
        perceptual_roughness: 0.92,
        ..default()
    });
    // South wall (outside BOUNDS_Z_MIN=-34)
    for i in -13..=13i32 {
        let x = i as f32 * 2.2;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(2.1, 2.2, 0.7))),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_xyz(x, 1.1, -35.2),
            GameEntity,
        ));
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.9, 0.55, 0.75))),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_xyz(x - 0.55, 2.5, -35.2),
            GameEntity,
        ));
    }
    // Side walls (x=±29, z from -34 to +25)
    for i in -16..=12i32 {
        let z = i as f32 * 2.2;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.7, 2.2, 2.1))),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_xyz(-29.5, 1.1, z),
            GameEntity,
        ));
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.7, 2.2, 2.1))),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_xyz(29.5, 1.1, z),
            GameEntity,
        ));
    }
    // North wall (open entrance gap in centre for the path)
    for i in -13..=13i32 {
        let x = i as f32 * 2.2;
        if x.abs() < 1.5 {
            continue;
        }
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(2.1, 2.2, 0.7))),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_xyz(x, 1.1, 26.2),
            GameEntity,
        ));
    }
    // Side walls (left x=-22.5, right x=22.5)
    for i in -12..=9i32 {
        let z = i as f32 * 2.2;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.7, 2.2, 2.1))),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_xyz(-22.5, 1.1, z),
            GameEntity,
        ));
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.7, 2.2, 2.1))),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_xyz(22.5, 1.1, z),
            GameEntity,
        ));
    }
    // North wall (open entrance gap in centre for the path)
    for i in -9..=9i32 {
        let x = i as f32 * 2.2;
        if x.abs() < 1.5 {
            continue;
        } // leave entrance gap
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(2.1, 2.2, 0.7))),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_xyz(x, 1.1, 20.5),
            GameEntity,
        ));
    }

    // Stone mine support pillars (replaces trees — guild has no trees)
    let pillar_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.40, 0.36, 0.30),
        perceptual_roughness: 0.93,
        ..default()
    });
    let pillar_cap_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.34, 0.30, 0.24),
        perceptual_roughness: 0.90,
        ..default()
    });
    let beam_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.20, 0.08),
        perceptual_roughness: 0.95,
        ..default()
    });
    for &[tx, tz] in &[
        // Original pillars (scaled out slightly)
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
        // Extra pillars for the expanded map area
        [-24.0, -8.0],
        [24.0, -8.0],
        [-23.0, 14.0],
        [23.0, 14.0],
        [-22.0, -26.0],
        [22.0, -26.0],
        [-8.0, -28.0],
        [8.0, -28.0],
        [0.0, 22.0],
        [-20.0, 0.0],
        [20.0, 0.0],
    ] {
        let ph = 2.4_f32;
        // Pillar shaft
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.20, ph).mesh().resolution(24))),
            MeshMaterial3d(pillar_mat.clone()),
            Transform::from_xyz(tx, ph / 2.0, tz),
            Collider::Circle(0.45),
            GameEntity,
        ));
        // Capital (top cap)
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.55, 0.18, 0.55))),
            MeshMaterial3d(pillar_cap_mat.clone()),
            Transform::from_xyz(tx, ph + 0.09, tz),
            GameEntity,
        ));
        // Base
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.50, 0.14, 0.50))),
            MeshMaterial3d(pillar_cap_mat.clone()),
            Transform::from_xyz(tx, 0.07, tz),
            GameEntity,
        ));
        // Wooden horizontal support beam connecting to nearest wall direction
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.10, 0.10, 1.0))),
            MeshMaterial3d(beam_mat.clone()),
            Transform::from_xyz(tx, ph - 0.3, tz),
            GameEntity,
        ));
    }

    // Cover crates/rocks scattered for tactical interest
    let crate_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.28, 0.16),
        perceptual_roughness: 0.85,
        ..default()
    });
    for &[cx, cz, cs] in &[
        [-7.0f32, -3.0, 0.7],
        [6.0, -6.0, 0.6],
        [-8.0, -12.0, 0.65],
        [9.0, 2.0, 0.55],
        [-3.0, 7.0, 0.5],
        [3.0, -17.0, 0.60],
        [-10.0, 5.0, 0.65],
        [10.0, -10.0, 0.55],
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(cs, cs * 0.8, cs))),
            MeshMaterial3d(crate_mat.clone()),
            Transform::from_xyz(cx, cs * 0.4, cz).with_rotation(Quat::from_rotation_y(cx * 0.5)),
            Collider::Circle(cs * 0.70),
            GameEntity,
        ));
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(cs * 0.7, cs * 0.7, cs * 0.7))),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_xyz(cx + cs * 0.7, cs * 0.35, cz - cs * 0.2)
                .with_rotation(Quat::from_rotation_y(cz * 0.3)),
            GameEntity,
        ));
    }

    // Ruined wall sections (cover)
    let ruin_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.38, 0.28),
        perceptual_roughness: 0.80,
        ..default()
    });
    for &[rx, rz, rw, ra] in &[
        [-11.0f32, -7.0, 2.2, 0.3],
        [9.0, -11.0, 1.8, -0.4],
        [-4.0, 5.0, 2.5, 0.1],
        [5.0, 1.0, 1.5, 0.8],
        [-7.0, -18.0, 2.0, 0.5],
        [7.0, -14.0, 1.8, -0.2],
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(rw, 0.9, 0.25))),
            MeshMaterial3d(ruin_mat.clone()),
            Transform::from_xyz(rx, 0.45, rz).with_rotation(Quat::from_rotation_y(ra)),
            Collider::Obb {
                half_x: rw / 2.0 + 0.05,
                half_z: 0.25,
            },
            GameEntity,
        ));
    }

    // ── Rocks ─────────────────────────────────────────────────
    let depl_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.26, 0.24, 0.22),
        perceptual_roughness: 0.96,
        ..default()
    });
    let dirt_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.33, 0.24, 0.12),
        perceptual_roughness: 1.0,
        ..default()
    });

    let rocks: &[(OreType, f32, f32)] = &[
        // West cluster
        (OreType::Copper, -11.0, -1.0),
        (OreType::Copper, -13.0, -4.0),
        (OreType::Iron, -12.0, -8.0),
        // Central path
        (OreType::Copper, -7.0, 7.0),
        (OreType::Tin, 5.0, -3.0),
        (OreType::Tin, -5.0, -6.0),
        // East cluster
        (OreType::Iron, 10.0, -9.0),
        (OreType::Iron, 13.0, -5.0),
        // South coal belt
        (OreType::Coal, 0.5, -15.0),
        (OreType::Coal, 5.0, -20.0),
        (OreType::Coal, -9.0, -18.0),
        // North rocks
        (OreType::Iron, 8.0, 12.0),
        (OreType::Copper, -14.0, 10.0),
        // Deep south rare ores
        (OreType::Adamantite, -21.0, -24.0),
        (OreType::Rune, 20.0, -27.0),
    ];
    for &(ore, rx, rz) in rocks {
        let r = match ore {
            OreType::Rune => 0.90,
            OreType::Adamantite => 0.75,
            OreType::Coal => 0.78,
            OreType::Iron => 0.65,
            _ => 0.55,
        };
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(r + 0.3, 0.02))),
            MeshMaterial3d(dirt_mat.clone()),
            Transform::from_xyz(rx, 0.01, rz),
            GameEntity,
        ));
        let full_mat = materials.add(StandardMaterial {
            base_color: ore.full_color(),
            perceptual_roughness: 0.82,
            metallic: match ore {
                OreType::Adamantite | OreType::Rune => 0.30,
                OreType::Iron => 0.12,
                _ => 0.06,
            },
            ..default()
        });
        // Scale emissive up for rare ores so they glow visibly
        let emissive_scale = match ore {
            OreType::Rune => 0.55,
            OreType::Adamantite => 0.28,
            _ => 0.12,
        };
        let vein_mat = materials.add(StandardMaterial {
            base_color: ore.vein_color(),
            emissive: {
                let c = ore.vein_color().to_linear();
                LinearRgba::new(
                    c.red * emissive_scale,
                    c.green * emissive_scale,
                    c.blue * emissive_scale,
                    1.0,
                )
            },
            perceptual_roughness: 0.62,
            ..default()
        });
        let rock_e = commands
            .spawn((
                Mesh3d(meshes.add(Sphere::new(r).mesh().uv(32, 18))),
                MeshMaterial3d(full_mat.clone()),
                Transform::from_xyz(rx, r, rz),
                Rock {
                    ore,
                    depleted: false,
                    respawn_timer: 0.0,
                    full_mat,
                    depl_mat: depl_mat.clone(),
                },
                Collider::Circle(r + 0.10),
                GameEntity,
            ))
            .id();
        let vein = commands
            .spawn((
                Mesh3d(meshes.add(Sphere::new(r * 0.52).mesh().uv(20, 12))),
                MeshMaterial3d(vein_mat),
                Transform::from_xyz(0.0, r * 0.1, r * 0.78),
            ))
            .id();
        commands.entity(vein).set_parent(rock_e);
        // Accent stone
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(r * 0.40).mesh().uv(8, 5))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.34, 0.31, 0.28),
                perceptual_roughness: 0.92,
                ..default()
            })),
            Transform::from_xyz(rx + r * 0.95, r * 0.40, rz + r * 0.45),
            GameEntity,
        ));
    }

    // ── Extraction zones ──────────────────────────────────────
    let zone_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.1, 0.9, 0.2, 0.5),
        emissive: LinearRgba::new(0.0, 0.6, 0.1, 1.0),
        alpha_mode: AlphaMode::Blend,
        unlit: false,
        ..default()
    });
    let pillar_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.85, 0.25),
        emissive: LinearRgba::new(0.0, 0.8, 0.15, 1.0),
        ..default()
    });

    for ex in [-18.0f32, 18.0] {
        let ez = -30.0_f32;
        // Glowing platform
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(EXTRACT_RADIUS, 0.08).mesh().resolution(48))),
            MeshMaterial3d(zone_mat.clone()),
            Transform::from_xyz(ex, 0.04, ez),
            ExtractionZone,
            GameEntity,
        ));
        // Corner pillars
        for &[px, pz] in &[[-1.8f32, -1.8], [1.8, -1.8], [-1.8, 1.8], [1.8, 1.8f32]] {
            commands.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.12, 2.2))),
                MeshMaterial3d(pillar_mat.clone()),
                Transform::from_xyz(ex + px, 1.1, ez + pz),
                GameEntity,
            ));
        }
        // "EXTRACT" sign bar
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(2.8, 0.38, 0.10))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.12, 0.80, 0.22),
                emissive: LinearRgba::new(0.0, 0.5, 0.1, 1.0),
                ..default()
            })),
            Transform::from_xyz(ex, 2.4, ez - 1.85),
            GameEntity,
        ));
    }
}

// ─────────────────────────────────────────────────────────────
//  Spawn player
// ─────────────────────────────────────────────────────────────
fn spawn_player(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    // OSRS default skin: bright yellow, brown hair, blue shirt, dark pants
    let skin = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.87, 0.42),
        perceptual_roughness: 0.78,
        ..default()
    });
    let shirt = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.30, 0.70),
        perceptual_roughness: 0.85,
        ..default()
    });
    let pants = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.18, 0.14),
        perceptual_roughness: 0.90,
        ..default()
    });
    let boot = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.12, 0.06),
        perceptual_roughness: 0.90,
        ..default()
    });
    let hair = materials.add(StandardMaterial {
        base_color: Color::srgb(0.26, 0.14, 0.04),
        perceptual_roughness: 0.90,
        ..default()
    });
    let axe_h = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.26, 0.10),
        perceptual_roughness: 0.90,
        ..default()
    });
    let axe_m = materials.add(StandardMaterial {
        base_color: Color::srgb(0.58, 0.60, 0.62),
        metallic: 0.6,
        perceptual_roughness: 0.40,
        ..default()
    });

    let (head, torso, left_arm, right_arm, left_leg, right_leg) = build_humanoid(
        commands,
        meshes,
        &skin,
        &shirt,
        &pants,
        &boot,
        &hair,
        Some((&axe_h, &axe_m)),
    );

    let player = commands
        .spawn((
            Transform::from_xyz(0.0, 0.0, 7.0),
            Visibility::default(),
            Player,
            Health {
                cur: PLAYER_HP,
                max: PLAYER_HP,
            },
            AttackCooldown(0.0),
            AnimState::default(),
            AnimTimer::default(),
            SwingTimer::default(),
            PlayerLimbs {
                head,
                torso,
                left_arm,
                right_arm,
                left_leg,
                right_leg,
            },
            GameEntity,
        ))
        .id();
    for c in [head, torso, left_arm, right_arm, left_leg, right_leg] {
        commands.entity(c).set_parent(player);
    }
}

// ─────────────────────────────────────────────────────────────
//  Spawn enemies
// ─────────────────────────────────────────────────────────────
fn spawn_enemies(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    // OSRS-style goblin: green skin, brown leather armor
    let skin = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.48, 0.16),
        perceptual_roughness: 0.82,
        ..default()
    });
    let shirt = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.24, 0.10),
        perceptual_roughness: 0.88,
        ..default()
    });
    let pants = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.20, 0.10),
        perceptual_roughness: 0.90,
        ..default()
    });
    let boot = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.12, 0.06),
        perceptual_roughness: 0.90,
        ..default()
    });
    let hair = materials.add(StandardMaterial {
        base_color: Color::srgb(0.10, 0.08, 0.04),
        perceptual_roughness: 0.90,
        ..default()
    });
    let axe_h = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.24, 0.10),
        perceptual_roughness: 0.90,
        ..default()
    });
    let axe_m = materials.add(StandardMaterial {
        base_color: Color::srgb(0.50, 0.52, 0.54),
        metallic: 0.55,
        perceptual_roughness: 0.45,
        ..default()
    });
    let hp_bg = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.05, 0.05),
        unlit: true,
        ..default()
    });
    let hp_fg = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.85, 0.20),
        emissive: LinearRgba::new(0.0, 0.3, 0.05, 1.0),
        unlit: true,
        ..default()
    });

    let enemy_defs: &[(&[(f32, f32)], f32, f32)] = &[
        // patrol waypoints (x,z), spawn_x, spawn_z
        (
            &[(-10.0, -3.0), (-10.0, 5.0), (-4.0, 5.0), (-4.0, -3.0)],
            -9.0,
            2.0,
        ),
        (
            &[(9.0, -2.0), (9.0, -11.0), (5.0, -11.0), (5.0, -2.0)],
            8.0,
            -4.0,
        ),
        (
            &[(-3.0, -14.0), (3.0, -14.0), (3.0, -10.0), (-3.0, -10.0)],
            0.0,
            -12.0,
        ),
        (
            &[(7.0, 11.0), (-7.0, 11.0), (-7.0, 6.0), (7.0, 6.0)],
            4.0,
            9.0,
        ),
    ];

    for &(waypoints, sx, sz) in enemy_defs {
        let patrol: Vec<Vec3> = waypoints
            .iter()
            .map(|&(x, z)| Vec3::new(x, 0.0, z))
            .collect();

        let (head, torso, left_arm, right_arm, left_leg, right_leg) = build_humanoid(
            commands,
            meshes,
            &skin,
            &shirt,
            &pants,
            &boot,
            &hair,
            Some((&axe_h, &axe_m)),
        );

        // HP bar (parented to enemy root)
        let bg = commands
            .spawn((
                Mesh3d(meshes.add(Cuboid::new(0.8, 0.08, 0.04))),
                MeshMaterial3d(hp_bg.clone()),
                Transform::from_xyz(0.0, 1.55, 0.0),
            ))
            .id();
        let fill = commands
            .spawn((
                Mesh3d(meshes.add(Cuboid::new(0.8, 0.08, 0.04))),
                MeshMaterial3d(hp_fg.clone()),
                Transform::from_xyz(0.0, 1.55, 0.001),
                HpBarFill,
            ))
            .id();

        let enemy = commands
            .spawn((
                Transform::from_xyz(sx, 0.0, sz),
                Visibility::default(),
                Enemy {
                    ai: EnemyAi::Patrolling { idx: 0, wait: 0.0 },
                    patrol,
                    hp_fill: fill,
                },
                Health {
                    cur: ENEMY_HP,
                    max: ENEMY_HP,
                },
                AttackCooldown(0.0),
                AnimState::default(),
                AnimTimer::default(),
                PlayerLimbs {
                    head,
                    torso,
                    left_arm,
                    right_arm,
                    left_leg,
                    right_leg,
                },
                GameEntity,
            ))
            .id();

        for c in [
            head, torso, left_arm, right_arm, left_leg, right_leg, bg, fill,
        ] {
            commands.entity(c).set_parent(enemy);
        }
    }
}

// ─────────────────────────────────────────────────────────────
//  Build humanoid (shared between player and enemy)
// ─────────────────────────────────────────────────────────────
fn build_humanoid(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    skin: &Handle<StandardMaterial>,
    shirt: &Handle<StandardMaterial>,
    pants: &Handle<StandardMaterial>,
    boot: &Handle<StandardMaterial>,
    hair: &Handle<StandardMaterial>,
    pickaxe: Option<(&Handle<StandardMaterial>, &Handle<StandardMaterial>)>,
) -> (Entity, Entity, Entity, Entity, Entity, Entity) {
    // Head
    let hd = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.27, 0.27, 0.26))),
            MeshMaterial3d(skin.clone()),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();
    let hr = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.29, 0.10, 0.28))),
            MeshMaterial3d(hair.clone()),
            Transform::from_xyz(0.0, 0.18, 0.0),
        ))
        .id();
    let head = commands
        .spawn((Transform::from_xyz(0.0, 1.05, 0.0), Visibility::default()))
        .id();
    commands.entity(hd).set_parent(head);
    commands.entity(hr).set_parent(head);

    // Face features on the +Z face (camera-facing at rest).
    // Reuse `hair` handle for eye/mouth color — dark brown, close to OSRS look.
    let eye_l = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.065, 0.065, 0.025))),
            MeshMaterial3d(hair.clone()),
            Transform::from_xyz(0.07, 0.03, 0.13),
        ))
        .id();
    let eye_r = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.065, 0.065, 0.025))),
            MeshMaterial3d(hair.clone()),
            Transform::from_xyz(-0.07, 0.03, 0.13),
        ))
        .id();
    let mouth = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.10, 0.030, 0.025))),
            MeshMaterial3d(hair.clone()),
            Transform::from_xyz(0.0, -0.07, 0.13),
        ))
        .id();
    commands.entity(eye_l).set_parent(head);
    commands.entity(eye_r).set_parent(head);
    commands.entity(mouth).set_parent(head);

    // Torso
    let torso = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.33, 0.38, 0.22))),
            MeshMaterial3d(shirt.clone()),
            Transform::from_xyz(0.0, 0.70, 0.0),
        ))
        .id();

    // Left arm
    let la_m = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.12, 0.34, 0.12))),
            MeshMaterial3d(shirt.clone()),
            Transform::from_xyz(0.0, -0.17, 0.0),
        ))
        .id();
    let left_arm = commands
        .spawn((Transform::from_xyz(-0.24, 0.82, 0.0), Visibility::default()))
        .id();
    commands.entity(la_m).set_parent(left_arm);

    // Right arm (with optional pickaxe)
    let ra_m = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.12, 0.34, 0.12))),
            MeshMaterial3d(shirt.clone()),
            Transform::from_xyz(0.0, -0.17, 0.0),
        ))
        .id();
    let right_arm = commands
        .spawn((Transform::from_xyz(0.24, 0.82, 0.0), Visibility::default()))
        .id();
    commands.entity(ra_m).set_parent(right_arm);
    if let Some((axe_h, axe_m)) = pickaxe {
        let shaft = commands
            .spawn((
                Mesh3d(meshes.add(Cuboid::new(0.055, 0.40, 0.055))),
                MeshMaterial3d(axe_h.clone()),
                Transform::from_xyz(0.06, -0.50, -0.05).with_rotation(Quat::from_rotation_z(0.18)),
            ))
            .id();
        let head_ = commands
            .spawn((
                Mesh3d(meshes.add(Cuboid::new(0.22, 0.09, 0.07))),
                MeshMaterial3d(axe_m.clone()),
                Transform::from_xyz(0.10, -0.68, -0.05),
            ))
            .id();
        commands.entity(shaft).set_parent(right_arm);
        commands.entity(head_).set_parent(right_arm);
    }

    // Left leg
    let ll_t = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.13, 0.28, 0.13))),
            MeshMaterial3d(pants.clone()),
            Transform::from_xyz(0.0, -0.14, 0.0),
        ))
        .id();
    let ll_b = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.14, 0.13, 0.17))),
            MeshMaterial3d(boot.clone()),
            Transform::from_xyz(0.0, -0.33, 0.02),
        ))
        .id();
    let left_leg = commands
        .spawn((Transform::from_xyz(-0.11, 0.35, 0.0), Visibility::default()))
        .id();
    commands.entity(ll_t).set_parent(left_leg);
    commands.entity(ll_b).set_parent(left_leg);

    // Right leg
    let rl_t = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.13, 0.28, 0.13))),
            MeshMaterial3d(pants.clone()),
            Transform::from_xyz(0.0, -0.14, 0.0),
        ))
        .id();
    let rl_b = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.14, 0.13, 0.17))),
            MeshMaterial3d(boot.clone()),
            Transform::from_xyz(0.0, -0.33, 0.02),
        ))
        .id();
    let right_leg = commands
        .spawn((Transform::from_xyz(0.11, 0.35, 0.0), Visibility::default()))
        .id();
    commands.entity(rl_t).set_parent(right_leg);
    commands.entity(rl_b).set_parent(right_leg);

    (head, torso, left_arm, right_arm, left_leg, right_leg)
}

// ─────────────────────────────────────────────────────────────
//  HUD
// ─────────────────────────────────────────────────────────────
fn spawn_hud(commands: &mut Commands) {
    // OSRS colour palette
    let panel_bg = Color::srgba(0.18, 0.13, 0.05, 0.95);
    let panel_gold = Color::srgb(1.00, 0.82, 0.22);
    let panel_text = Color::srgb(1.00, 1.00, 1.00);
    let bar_dark = Color::srgb(0.28, 0.06, 0.06);
    let bar_green = Color::srgb(0.10, 0.80, 0.15);

    // Damage flash overlay
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.8, 0.0, 0.0, 0.0)),
        ZIndex(5),
        DamageFlash,
        GameEntity,
    ));

    // ── Top-left stats panel ──────────────────────────────────
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(10.0)),
                min_width: Val::Px(230.0),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(panel_bg),
            BorderColor(Color::srgb(0.45, 0.34, 0.12)),
            GameEntity,
        ))
        .with_children(|p| {
            // Title
            p.spawn((
                Text::new("── Extraction Mining ──"),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(panel_gold),
            ));
            // HP
            p.spawn((
                Text::new("Hitpoints: 100 / 100"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(panel_text),
                HpBarText,
            ));
            p.spawn((
                Node {
                    width: Val::Px(210.0),
                    height: Val::Px(12.0),
                    ..default()
                },
                BackgroundColor(bar_dark),
            ))
            .with_children(|bar| {
                bar.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(bar_green),
                    HpBarFill,
                ));
            });
            // Inventory
            p.spawn((
                Text::new("Ore: 0  (0 gp)"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(panel_gold),
                OreText,
            ));
            // Skill level
            p.spawn((
                Text::new("Mining Lv: 1"),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.65, 0.90, 0.45)),
                StatusText,
            ));
        });

    // ── OSRS-style chat box (bottom-left) ─────────────────────
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                left: Val::Px(10.0),
                width: Val::Px(510.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect {
                    left: Val::Px(8.0),
                    right: Val::Px(8.0),
                    top: Val::Px(6.0),
                    bottom: Val::Px(6.0),
                },
                row_gap: Val::Px(1.0),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.10, 0.07, 0.02, 0.92)),
            BorderColor(Color::srgb(0.42, 0.32, 0.10)),
            GameEntity,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("Chat"),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::srgb(0.75, 0.60, 0.18)),
            ));
            // Lines: oldest (index CHAT_LINES-1) at top, newest (0) at bottom
            for i in (0..CHAT_LINES).rev() {
                p.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.0, 0.85, 0.85)),
                    ChatLine(i),
                    GameEntity,
                ));
            }
        });

    // ── Mining / extraction progress bar (above chat box) ─────
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(178.0),
                left: Val::Px(10.0),
                width: Val::Px(510.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            GameEntity,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new(""),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(panel_gold),
                MiningBarFill,
            ));
            p.spawn((
                Node {
                    width: Val::Px(510.0),
                    height: Val::Px(14.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.12, 0.09, 0.04, 0.88)),
            ))
            .with_children(|bar| {
                bar.spawn((
                    Node {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.85, 0.65, 0.08)),
                    ExtractBarFill,
                ));
            });
        });

    // ── Controls hint (bottom-right) ─────────────────────────
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                right: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(3.0),
                padding: UiRect::all(Val::Px(10.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(panel_bg),
            BorderColor(Color::srgb(0.45, 0.34, 0.12)),
            GameEntity,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("── Controls ──"),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(panel_gold),
            ));
            for (key, desc) in &[
                ("LMB ground", "Move"),
                ("LMB enemy", "Attack"),
                ("LMB rock", "Mine"),
                ("Walk to zone", "Extract"),
                ("R", "Restart"),
                ("ESC", "Quit"),
            ] {
                p.spawn((
                    Text::new(format!("{:<14}{}", key, desc)),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.82, 0.82, 0.82)),
                ));
            }
        });

    // ── Extraction distance (top-right) ──────────────────────
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                right: Val::Px(10.0),
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(panel_bg),
            BorderColor(Color::srgb(0.45, 0.34, 0.12)),
            GameEntity,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("EXTRACT: --"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.25, 0.92, 0.38)),
                ExtractBar,
            ));
        });

    // ── Game over / win overlay ───────────────────────────────
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(18.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.82)),
            Visibility::Hidden,
            ZIndex(10),
            GameOverlay,
            GameEntity,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new(""),
                TextFont {
                    font_size: 46.0,
                    ..default()
                },
                TextColor(panel_gold),
                GameOverTitle,
            ));
            p.spawn((
                Text::new("Press R to restart  |  ESC to quit"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.70)),
            ));
        });

    // ── Action state banner (top-center) ─────────────────────
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Percent(50.0),
                padding: UiRect::axes(Val::Px(22.0), Val::Px(8.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            BorderColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            Visibility::Hidden,
            ZIndex(4),
            ActionStatePanel,
            GameEntity,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new(""),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                ActionStateLabel,
            ));
        });
}

// ─────────────────────────────────────────────────────────────
//  handle_click  (LMB → set PlayerTarget)
// ─────────────────────────────────────────────────────────────
fn handle_click(
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mouse: Res<ButtonInput<MouseButton>>,
    phase: Res<GamePhase>,
    enemy_q: Query<(Entity, &Transform), With<Enemy>>,
    rock_q: Query<(Entity, &Transform, &Rock)>,
    mut click_target: ResMut<PlayerTarget>,
    mut action: ResMut<PlayerAction>,
) {
    if *phase != GamePhase::Playing {
        return;
    }
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.get_single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok((camera, cam_gtf)) = camera_q.get_single() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(cam_gtf, cursor_pos) else {
        return;
    };
    let dir = Vec3::from(ray.direction);
    if dir.y.abs() < 1e-5 {
        return;
    }
    let t = -ray.origin.y / dir.y;
    if t <= 0.0 {
        return;
    }
    let world_pos = ray.origin + dir * t;

    // Check enemies first
    let mut nearest_enemy: Option<(Entity, f32)> = None;
    for (entity, etf) in &enemy_q {
        let d = flat_diff(world_pos, etf.translation).length();
        if d < CLICK_ENEMY_RADIUS {
            if nearest_enemy.map_or(true, |(_, bd)| d < bd) {
                nearest_enemy = Some((entity, d));
            }
        }
    }
    if let Some((entity, _)) = nearest_enemy {
        *click_target = PlayerTarget::Attack(entity);
        if matches!(*action, PlayerAction::Mining { .. }) {
            *action = PlayerAction::Free;
        }
        return;
    }

    // Check rocks
    let mut nearest_rock: Option<(Entity, f32)> = None;
    for (entity, rtf, rock) in &rock_q {
        if rock.depleted {
            continue;
        }
        let d = flat_diff(world_pos, rtf.translation).length();
        if d < CLICK_ROCK_RADIUS {
            if nearest_rock.map_or(true, |(_, bd)| d < bd) {
                nearest_rock = Some((entity, d));
            }
        }
    }
    if let Some((entity, _)) = nearest_rock {
        *click_target = PlayerTarget::Mine(entity);
        if matches!(*action, PlayerAction::Mining { .. }) {
            *action = PlayerAction::Free;
        }
        return;
    }

    // Move to clicked ground position
    let snapped = Vec3::new(
        (world_pos.x / TILE_SIZE).round() * TILE_SIZE,
        0.0,
        (world_pos.z / TILE_SIZE).round() * TILE_SIZE,
    );
    let snapped = Vec3::new(
        snapped.x.clamp(-BOUNDS_X, BOUNDS_X),
        0.0,
        snapped.z.clamp(BOUNDS_Z_MIN, BOUNDS_Z_MAX),
    );
    *click_target = PlayerTarget::Move(snapped);
    if matches!(*action, PlayerAction::Mining { .. }) {
        *action = PlayerAction::Free;
    }
}

// ─────────────────────────────────────────────────────────────
//  player_walk  (move toward PlayerTarget)
// ─────────────────────────────────────────────────────────────
fn player_walk(
    time: Res<Time>,
    phase: Res<GamePhase>,
    mut player_q: Query<(&mut Transform, &mut AnimState, &mut SwingTimer), With<Player>>,
    mut click_target: ResMut<PlayerTarget>,
    mut action: ResMut<PlayerAction>,
    enemy_q: Query<&Transform, (With<Enemy>, Without<Player>)>,
    rock_q: Query<(Entity, &Transform, &Rock), Without<Player>>,
) {
    if *phase != GamePhase::Playing {
        return;
    }
    let Ok((mut tf, mut anim, mut swing)) = player_q.get_single_mut() else {
        return;
    };
    let dt = time.delta_secs();

    // Tick swing timer
    swing.0 = (swing.0 - dt).max(0.0);

    // Don't walk while mining
    if matches!(*action, PlayerAction::Mining { .. }) {
        *anim = AnimState::Mining;
        return;
    }

    let current = (*click_target).clone();

    let result = match &current {
        PlayerTarget::None => {
            if swing.0 > 0.0 {
                *anim = AnimState::Mining;
            } else if matches!(*anim, AnimState::Walking) {
                *anim = AnimState::Idle;
            }
            return;
        }
        PlayerTarget::Move(pos) => Some((*pos, 0.40_f32)),
        PlayerTarget::Attack(entity) => match enemy_q.get(*entity) {
            Ok(etf) => Some((etf.translation, ATTACK_RANGE * 0.80)),
            Err(_) => {
                *click_target = PlayerTarget::None;
                if swing.0 <= 0.0 {
                    *anim = AnimState::Idle;
                }
                return;
            }
        },
        PlayerTarget::Mine(entity) => match rock_q.get(*entity) {
            Ok((_, rtf, rock)) => {
                if rock.depleted {
                    *click_target = PlayerTarget::None;
                    *anim = AnimState::Idle;
                    return;
                }
                Some((rtf.translation, INTERACT_DIST * 0.9))
            }
            Err(_) => {
                *click_target = PlayerTarget::None;
                *anim = AnimState::Idle;
                return;
            }
        },
    };

    let Some((dest, stop_dist)) = result else {
        return;
    };

    let diff = flat_diff(tf.translation, dest);
    let dist = diff.length();

    if dist > stop_dist {
        let dir = diff.normalize();
        tf.translation += dir * PLAYER_SPEED * dt;
        tf.translation.x = tf.translation.x.clamp(-BOUNDS_X, BOUNDS_X);
        tf.translation.z = tf.translation.z.clamp(BOUNDS_Z_MIN, BOUNDS_Z_MAX);
        face(&mut tf, dir);
        if swing.0 <= 0.0 {
            *anim = AnimState::Walking;
        }
    } else {
        // Arrived at destination
        match &current {
            PlayerTarget::Move(_) => {
                *click_target = PlayerTarget::None;
                if swing.0 <= 0.0 {
                    *anim = AnimState::Idle;
                }
            }
            PlayerTarget::Attack(_) => {
                // Stay in range — auto-attack fires from player_combat_mine
                if swing.0 > 0.0 {
                    *anim = AnimState::Mining;
                } else {
                    *anim = AnimState::Idle;
                }
            }
            PlayerTarget::Mine(entity) => {
                if let Ok((_, _, rock)) = rock_q.get(*entity) {
                    if !rock.depleted {
                        *action = PlayerAction::Mining {
                            target: *entity,
                            progress: 0.0,
                            total: rock.ore.mine_time(),
                            ore: rock.ore,
                        };
                        *anim = AnimState::Mining;
                    }
                }
                *click_target = PlayerTarget::None;
            }
            PlayerTarget::None => {}
        }
    }
}

// ─────────────────────────────────────────────────────────────
//  player_combat_mine  (auto-attack + mining tick)
// ─────────────────────────────────────────────────────────────
fn player_combat_mine(
    time: Res<Time>,
    phase: Res<GamePhase>,
    mut player_q: Query<
        (
            &Transform,
            &mut AnimState,
            &mut AttackCooldown,
            &mut SwingTimer,
        ),
        With<Player>,
    >,
    mut rock_q: Query<(
        Entity,
        &Transform,
        &mut Rock,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
    enemy_q: Query<(Entity, &Transform), (With<Enemy>, Without<Player>)>,
    mut action: ResMut<PlayerAction>,
    mut inventory: ResMut<Inventory>,
    mut stats: ResMut<PlayerStats>,
    mut damage_events: EventWriter<DamageEvent>,
    mut chat_log: ResMut<ChatLog>,
) {
    if *phase != GamePhase::Playing {
        return;
    }
    let Ok((player_tf, mut anim, mut cd, mut swing)) = player_q.get_single_mut() else {
        return;
    };
    let dt = time.delta_secs();

    // Tick attack cooldown
    cd.0 = (cd.0 - dt).max(0.0);

    // ── Auto-attack: swing whenever an enemy is in range and cd is ready ──
    if cd.0 <= 0.0 {
        let mut nearest: Option<(Entity, f32)> = None;
        for (e, etf) in &enemy_q {
            let d = flat_diff(player_tf.translation, etf.translation).length();
            if d < ATTACK_RANGE {
                if nearest.map_or(true, |(_, bd)| d < bd) {
                    nearest = Some((e, d));
                }
            }
        }
        if let Some((target, _)) = nearest {
            damage_events.send(DamageEvent {
                target,
                amount: PLAYER_DMG,
            });
            cd.0 = PLAYER_ATK_CD;
            swing.0 = PLAYER_ATK_CD;
            *anim = AnimState::Mining;
        }
    }

    // ── Tick active mining ────────────────────────────────────
    let current = std::mem::replace(&mut *action, PlayerAction::Free);
    *action = match current {
        PlayerAction::Mining {
            target,
            progress,
            total,
            ore,
        } => {
            let new_p = progress + dt;
            if new_p >= total {
                if let Ok((_, _, mut rock, mut mat)) = rock_q.get_mut(target) {
                    rock.depleted = true;
                    rock.respawn_timer = ore.respawn_time();
                    mat.0 = rock.depl_mat.clone();
                }
                let old_level = stats.level();
                inventory.add(ore);
                stats.mining_xp += ore.xp();
                let new_level = stats.level();
                chat_log.push(
                    format!("You mine some {} ore.", ore.name()),
                    ChatColor::Game,
                );
                chat_log.push(format!("You gain {} Mining XP.", ore.xp()), ChatColor::Xp);
                if new_level > old_level {
                    chat_log.push(
                        format!("Congratulations! Your Mining level is now {}!", new_level),
                        ChatColor::LevelUp,
                    );
                }
                *anim = AnimState::Idle;
                PlayerAction::Free
            } else {
                *anim = AnimState::Mining;
                PlayerAction::Mining {
                    target,
                    progress: new_p,
                    total,
                    ore,
                }
            }
        }
        other => other,
    };
}

// ─────────────────────────────────────────────────────────────
//  update_indicators  (click / target visual rings)
// ─────────────────────────────────────────────────────────────
fn update_indicators(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    click_target: Res<PlayerTarget>,
    tf_q: Query<&Transform, Without<TargetIndicator>>,
    mut ind: ResMut<ClickIndicators>,
    mut ind_tf_q: Query<&mut Transform, With<TargetIndicator>>,
) {
    let pulse = 1.0 + (time.elapsed_secs() * 5.0).sin() * 0.10;

    match &*click_target {
        PlayerTarget::None => {
            drop_indicator(&mut commands, &mut ind.move_ent);
            drop_indicator(&mut commands, &mut ind.target_ent);
            ind.tracked = None;
        }

        PlayerTarget::Move(pos) => {
            drop_indicator(&mut commands, &mut ind.target_ent);
            ind.tracked = None;

            let world = Vec3::new(pos.x, 0.07, pos.z);
            if let Some(e) = ind.move_ent {
                if let Ok(mut tf) = ind_tf_q.get_mut(e) {
                    tf.translation = world;
                    tf.scale = Vec3::splat(pulse);
                }
            } else {
                let mat = materials.add(StandardMaterial {
                    base_color: Color::srgba(0.25, 0.95, 1.0, 0.72),
                    emissive: LinearRgba::new(0.05, 0.55, 0.85, 1.0),
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    ..default()
                });
                let mesh = meshes.add(Torus {
                    minor_radius: 0.055,
                    major_radius: 0.32,
                });
                ind.move_ent = Some(
                    commands
                        .spawn((
                            Mesh3d(mesh),
                            MeshMaterial3d(mat),
                            Transform::from_translation(world),
                            TargetIndicator,
                            GameEntity,
                        ))
                        .id(),
                );
            }
        }

        PlayerTarget::Attack(target_e) => {
            drop_indicator(&mut commands, &mut ind.move_ent);

            // Respawn ring if target changed
            if ind.tracked != Some(*target_e) {
                drop_indicator(&mut commands, &mut ind.target_ent);
                ind.tracked = Some(*target_e);
            }

            if let Ok(etf) = tf_q.get(*target_e) {
                let world = Vec3::new(etf.translation.x, 0.07, etf.translation.z);
                if let Some(e) = ind.target_ent {
                    if let Ok(mut tf) = ind_tf_q.get_mut(e) {
                        tf.translation = world;
                        tf.scale = Vec3::splat(pulse);
                    }
                } else {
                    let mat = materials.add(StandardMaterial {
                        base_color: Color::srgba(1.0, 0.18, 0.18, 0.80),
                        emissive: LinearRgba::new(0.90, 0.08, 0.08, 1.0),
                        alpha_mode: AlphaMode::Blend,
                        unlit: true,
                        ..default()
                    });
                    let mesh = meshes.add(Torus {
                        minor_radius: 0.065,
                        major_radius: 0.48,
                    });
                    ind.target_ent = Some(
                        commands
                            .spawn((
                                Mesh3d(mesh),
                                MeshMaterial3d(mat),
                                Transform::from_translation(world),
                                TargetIndicator,
                                GameEntity,
                            ))
                            .id(),
                    );
                }
            }
        }

        PlayerTarget::Mine(target_e) => {
            drop_indicator(&mut commands, &mut ind.move_ent);

            if ind.tracked != Some(*target_e) {
                drop_indicator(&mut commands, &mut ind.target_ent);
                ind.tracked = Some(*target_e);
            }

            if let Ok(rtf) = tf_q.get(*target_e) {
                let world = Vec3::new(rtf.translation.x, 0.07, rtf.translation.z);
                if let Some(e) = ind.target_ent {
                    if let Ok(mut tf) = ind_tf_q.get_mut(e) {
                        tf.translation = world;
                        tf.scale = Vec3::splat(pulse);
                    }
                } else {
                    let mat = materials.add(StandardMaterial {
                        base_color: Color::srgba(1.0, 0.82, 0.12, 0.80),
                        emissive: LinearRgba::new(0.55, 0.40, 0.04, 1.0),
                        alpha_mode: AlphaMode::Blend,
                        unlit: true,
                        ..default()
                    });
                    let mesh = meshes.add(Torus {
                        minor_radius: 0.065,
                        major_radius: 0.60,
                    });
                    ind.target_ent = Some(
                        commands
                            .spawn((
                                Mesh3d(mesh),
                                MeshMaterial3d(mat),
                                Transform::from_translation(world),
                                TargetIndicator,
                                GameEntity,
                            ))
                            .id(),
                    );
                }
            }
        }
    }
}

fn drop_indicator(commands: &mut Commands, slot: &mut Option<Entity>) {
    if let Some(e) = slot.take() {
        commands.entity(e).despawn_recursive();
    }
}

// ─────────────────────────────────────────────────────────────
//  ai_update
// ─────────────────────────────────────────────────────────────
fn ai_update(
    time: Res<Time>,
    phase: Res<GamePhase>,
    player_q: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemy_q: Query<
        (
            &mut Transform,
            &mut Enemy,
            &mut AnimState,
            &mut AttackCooldown,
        ),
        Without<Player>,
    >,
    mut damage_events: EventWriter<DamageEvent>,
    player_entity_q: Query<Entity, With<Player>>,
) {
    if *phase != GamePhase::Playing {
        return;
    }
    let Ok(player_tf) = player_q.get_single() else {
        return;
    };
    let Ok(player_entity) = player_entity_q.get_single() else {
        return;
    };
    let dt = time.delta_secs();

    for (mut tf, mut enemy, mut anim, mut cd) in &mut enemy_q {
        if matches!(enemy.ai, EnemyAi::Dead) {
            continue;
        }

        cd.0 = (cd.0 - dt).max(0.0);
        let to_player = flat_diff(tf.translation, player_tf.translation);
        let dist = to_player.length();

        let new_ai = match enemy.ai.clone() {
            EnemyAi::Patrolling { idx, wait } => {
                if dist < DETECT_RADIUS {
                    *anim = AnimState::Walking;
                    EnemyAi::Chasing { lose_timer: 6.0 }
                } else if wait > 0.0 {
                    *anim = AnimState::Idle;
                    EnemyAi::Patrolling {
                        idx,
                        wait: wait - dt,
                    }
                } else {
                    let wp = enemy.patrol[idx];
                    let diff = flat_diff(tf.translation, wp);
                    if diff.length() < 0.4 {
                        let next = (idx + 1) % enemy.patrol.len();
                        *anim = AnimState::Idle;
                        EnemyAi::Patrolling {
                            idx: next,
                            wait: 1.8,
                        }
                    } else {
                        let dir = diff.normalize();
                        tf.translation += dir * ENEMY_SPEED * dt;
                        clamp_pos(&mut tf);
                        face(&mut tf, dir);
                        *anim = AnimState::Walking;
                        EnemyAi::Patrolling { idx, wait: 0.0 }
                    }
                }
            }

            EnemyAi::Chasing { lose_timer } => {
                if dist < ATTACK_RANGE * 0.9 {
                    *anim = AnimState::Mining;
                    EnemyAi::Attacking { cooldown: 0.0 }
                } else if dist > LOSE_RADIUS {
                    let new_t = lose_timer - dt;
                    if new_t <= 0.0 {
                        *anim = AnimState::Idle;
                        EnemyAi::Patrolling { idx: 0, wait: 0.0 }
                    } else {
                        // Keep chasing but timer ticking
                        let dir = to_player.normalize();
                        tf.translation += dir * ENEMY_CHASE * dt;
                        clamp_pos(&mut tf);
                        face(&mut tf, dir);
                        *anim = AnimState::Walking;
                        EnemyAi::Chasing { lose_timer: new_t }
                    }
                } else {
                    let dir = to_player.normalize();
                    tf.translation += dir * ENEMY_CHASE * dt;
                    clamp_pos(&mut tf);
                    face(&mut tf, dir);
                    *anim = AnimState::Walking;
                    EnemyAi::Chasing { lose_timer: 6.0 }
                }
            }

            EnemyAi::Attacking { cooldown } => {
                if dist > ATTACK_RANGE * 1.4 {
                    *anim = AnimState::Walking;
                    EnemyAi::Chasing { lose_timer: 6.0 }
                } else {
                    // Face player
                    if to_player.length() > 0.01 {
                        face(&mut tf, to_player.normalize());
                    }
                    *anim = AnimState::Mining;
                    if cooldown <= 0.0 {
                        damage_events.send(DamageEvent {
                            target: player_entity,
                            amount: ENEMY_DMG,
                        });
                        EnemyAi::Attacking {
                            cooldown: ENEMY_ATK_CD,
                        }
                    } else {
                        EnemyAi::Attacking {
                            cooldown: cooldown - dt,
                        }
                    }
                }
            }

            EnemyAi::Dead => EnemyAi::Dead,
        };
        enemy.ai = new_ai;
    }
}

// ─────────────────────────────────────────────────────────────
//  apply_damage
// ─────────────────────────────────────────────────────────────
fn apply_damage(
    mut commands: Commands,
    mut events: EventReader<DamageEvent>,
    mut health_q: Query<&mut Health>,
    transform_q: Query<&Transform>,
    mut flash_q: Query<&mut BackgroundColor, With<DamageFlash>>,
    player_q: Query<Entity, With<Player>>,
    mut action: ResMut<PlayerAction>,
    mut chat_log: ResMut<ChatLog>,
) {
    for ev in events.read() {
        if let Ok(mut hp) = health_q.get_mut(ev.target) {
            hp.cur = (hp.cur - ev.amount).max(0.0);
        }
        let is_player_target = player_q.get(ev.target).is_ok();

        // Spawn floating damage splat
        let world_pos = transform_q
            .get(ev.target)
            .map(|tf| tf.translation + Vec3::new(0.0, 1.8, 0.0))
            .unwrap_or(Vec3::new(0.0, 1.8, 0.0));
        let splat_color = Color::srgb(1.0, 0.15, 0.15);
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..default()
            },
            Text::new(format!("{}", ev.amount as i32)),
            TextFont {
                font_size: 20.0,
                ..default()
            },
            TextColor(splat_color),
            ZIndex(20),
            FloatingSplat {
                world_pos,
                timer: 0.0,
                max_time: 1.2,
                base_color: splat_color,
            },
            GameEntity,
        ));

        // Chat messages
        if is_player_target {
            for mut bg in &mut flash_q {
                bg.0 = Color::srgba(0.8, 0.0, 0.0, 0.45);
            }
            if matches!(*action, PlayerAction::Extracting { .. }) {
                *action = PlayerAction::Free;
            }
            chat_log.push(
                format!("You have been hit for {} damage.", ev.amount as i32),
                ChatColor::Damage,
            );
        } else {
            chat_log.push(
                format!("You hit for {} damage.", ev.amount as i32),
                ChatColor::Game,
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────
//  check_deaths
// ─────────────────────────────────────────────────────────────
fn check_deaths(
    mut commands: Commands,
    mut phase: ResMut<GamePhase>,
    player_q: Query<(Entity, &Health), With<Player>>,
    mut enemy_q: Query<(Entity, &Health, &mut Enemy, &mut AnimState)>,
) {
    // Player death
    if let Ok((_, hp)) = player_q.get_single() {
        if hp.cur <= 0.0 && *phase == GamePhase::Playing {
            *phase = GamePhase::Dead;
        }
    }
    // Enemy deaths
    for (entity, hp, mut enemy, mut anim) in &mut enemy_q {
        if hp.cur <= 0.0 && !matches!(enemy.ai, EnemyAi::Dead) {
            enemy.ai = EnemyAi::Dead;
            *anim = AnimState::Idle;
            commands.entity(entity).despawn_recursive();
        }
    }
}

// ─────────────────────────────────────────────────────────────
//  extraction_update
// ─────────────────────────────────────────────────────────────
fn extraction_update(
    time: Res<Time>,
    player_q: Query<&Transform, With<Player>>,
    zone_q: Query<&Transform, With<ExtractionZone>>,
    mut action: ResMut<PlayerAction>,
    mut game_phase: ResMut<GamePhase>,
) {
    if *game_phase != GamePhase::Playing {
        return;
    }
    let Ok(player_tf) = player_q.get_single() else {
        return;
    };

    let on_zone = zone_q
        .iter()
        .any(|ztf| flat_diff(player_tf.translation, ztf.translation).length() < EXTRACT_RADIUS);

    let current = std::mem::replace(&mut *action, PlayerAction::Free);
    *action = match current {
        PlayerAction::Extracting { progress } => {
            if !on_zone {
                PlayerAction::Free // left the zone
            } else {
                let new_p = progress + time.delta_secs();
                if new_p >= EXTRACT_TIME {
                    *game_phase = GamePhase::Extracted;
                    PlayerAction::Free
                } else {
                    PlayerAction::Extracting { progress: new_p }
                }
            }
        }
        PlayerAction::Free => {
            if on_zone {
                PlayerAction::Extracting { progress: 0.0 }
            } else {
                PlayerAction::Free
            }
        }
        other => {
            if on_zone {
                PlayerAction::Extracting { progress: 0.0 }
            } else {
                other
            }
        }
    };
}

// ─────────────────────────────────────────────────────────────
//  animate_characters  (player + enemies)
// ─────────────────────────────────────────────────────────────
fn animate_characters(
    time: Res<Time>,
    mut chars: Query<(
        &PlayerLimbs,
        &AnimState,
        &mut AnimTimer,
        Option<&SwingTimer>,
    )>,
    mut transforms: Query<&mut Transform, Without<PlayerLimbs>>,
) {
    for (limbs, state, mut timer, swing) in &mut chars {
        timer.0 += time.delta_secs();
        let t = timer.0;
        match state {
            AnimState::Idle => {
                let b = (t * 1.1).sin() * 0.016;
                sr(&mut transforms, limbs.left_arm, Quat::from_rotation_z(0.08));
                sr(
                    &mut transforms,
                    limbs.right_arm,
                    Quat::from_rotation_z(-0.08),
                );
                sr(&mut transforms, limbs.torso, Quat::from_rotation_z(b * 0.4));
                sr(
                    &mut transforms,
                    limbs.head,
                    Quat::from_rotation_x(-0.04 + b),
                );
                sr(&mut transforms, limbs.left_leg, Quat::IDENTITY);
                sr(&mut transforms, limbs.right_leg, Quat::IDENTITY);
            }
            AnimState::Walking => {
                let s = (t * 3.8).sin();
                let bob = (t * 7.6).sin().abs() * 0.024;
                sr(
                    &mut transforms,
                    limbs.left_leg,
                    Quat::from_rotation_x(s * 0.52),
                );
                sr(
                    &mut transforms,
                    limbs.right_leg,
                    Quat::from_rotation_x(-s * 0.52),
                );
                sr(
                    &mut transforms,
                    limbs.left_arm,
                    Quat::from_rotation_x(-s * 0.36),
                );
                sr(
                    &mut transforms,
                    limbs.right_arm,
                    Quat::from_rotation_x(s * 0.36),
                );
                sr(&mut transforms, limbs.head, Quat::IDENTITY);
                sr(&mut transforms, limbs.torso, Quat::IDENTITY);
                sty(&mut transforms, limbs.torso, 0.70 + bob);
                sty(&mut transforms, limbs.head, 1.05 + bob);
            }
            AnimState::Mining => {
                // If this entity has an active SwingTimer, use phase-driven attack swing.
                // Otherwise (enemies, actual mining) fall back to the looping timer anim.
                let use_swing = swing.map_or(false, |sw| sw.0 > 0.0);
                if use_swing {
                    let sw = swing.unwrap();
                    // phase 0 = swing just started, 1 = swing finished
                    let phase = (1.0 - sw.0 / PLAYER_ATK_CD).clamp(0.0, 1.0);
                    // windup (0..0.3) → impact (0.3) → follow-through/return (0.3..1.0)
                    // Positive rotation_x = arm tip toward camera (wind-up back)
                    // Negative rotation_x = arm tip away from camera (strike toward enemy)
                    let arm_x = if phase < 0.30 {
                        lerp(0.1, 2.4, phase / 0.30) // raise arm up/back
                    } else {
                        lerp(2.4, -1.4, (phase - 0.30) / 0.70) // slam forward through target
                    };
                    let support_x = if phase < 0.30 {
                        lerp(0.0, 1.2, phase / 0.30)
                    } else {
                        lerp(1.2, -0.6, (phase - 0.30) / 0.70)
                    };
                    let lean = if phase < 0.30 {
                        lerp(0.0, -0.22, phase / 0.30) // lean back on wind-up
                    } else {
                        lerp(-0.22, 0.18, (phase - 0.30) / 0.70) // lean forward on strike
                    };
                    sr(
                        &mut transforms,
                        limbs.right_arm,
                        Quat::from_rotation_x(arm_x),
                    );
                    sr(
                        &mut transforms,
                        limbs.left_arm,
                        Quat::from_rotation_x(support_x),
                    );
                    sr(&mut transforms, limbs.torso, Quat::from_rotation_x(lean));
                    sr(
                        &mut transforms,
                        limbs.head,
                        Quat::from_rotation_x(lean * 0.4 - 0.05),
                    );
                    sr(&mut transforms, limbs.left_leg, Quat::from_rotation_x(0.12));
                    sr(
                        &mut transforms,
                        limbs.right_leg,
                        Quat::from_rotation_x(-0.12),
                    );
                } else {
                    // Looping mining / enemy attack animation
                    let s = (t * 3.5).sin();
                    let imp = s.max(0.0);
                    sr(
                        &mut transforms,
                        limbs.right_arm,
                        Quat::from_rotation_x(s * 1.4 - 0.25),
                    );
                    sr(
                        &mut transforms,
                        limbs.left_arm,
                        Quat::from_rotation_x(-s * 0.45 + 0.10),
                    );
                    sr(
                        &mut transforms,
                        limbs.torso,
                        Quat::from_rotation_x(imp * 0.16),
                    );
                    sr(
                        &mut transforms,
                        limbs.head,
                        Quat::from_rotation_x(imp * 0.10 - 0.05),
                    );
                    sr(&mut transforms, limbs.left_leg, Quat::from_rotation_x(0.08));
                    sr(
                        &mut transforms,
                        limbs.right_leg,
                        Quat::from_rotation_x(-0.08),
                    );
                }
            }
        }
    }
}

fn sr(q: &mut Query<&mut Transform, Without<PlayerLimbs>>, e: Entity, r: Quat) {
    if let Ok(mut tf) = q.get_mut(e) {
        tf.rotation = r;
    }
}
fn sty(q: &mut Query<&mut Transform, Without<PlayerLimbs>>, e: Entity, y: f32) {
    if let Ok(mut tf) = q.get_mut(e) {
        tf.translation.y = y;
    }
}

// ─────────────────────────────────────────────────────────────
//  update_enemy_hp_bars
// ─────────────────────────────────────────────────────────────
fn update_enemy_hp_bars(
    enemies: Query<(&Health, &Enemy)>,
    mut fills: Query<&mut Transform, (With<HpBarFill>, Without<Health>)>,
) {
    for (hp, enemy) in &enemies {
        let ratio = (hp.cur / hp.max).clamp(0.0, 1.0);
        if let Ok(mut tf) = fills.get_mut(enemy.hp_fill) {
            tf.scale.x = ratio.max(0.001);
            tf.translation.x = 0.4 * (ratio - 1.0);
        }
    }
}

// ─────────────────────────────────────────────────────────────
//  rock_respawn
// ─────────────────────────────────────────────────────────────
fn rock_respawn(
    mut rocks: Query<(&mut Rock, &mut MeshMaterial3d<StandardMaterial>)>,
    time: Res<Time>,
) {
    for (mut rock, mut mat) in &mut rocks {
        if rock.depleted && rock.respawn_timer > 0.0 {
            rock.respawn_timer -= time.delta_secs();
            if rock.respawn_timer <= 0.0 {
                rock.depleted = false;
                mat.0 = rock.full_mat.clone();
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────
//  damage_flash_update
// ─────────────────────────────────────────────────────────────
fn damage_flash_update(
    mut flash_q: Query<&mut BackgroundColor, With<DamageFlash>>,
    time: Res<Time>,
) {
    for mut bg in &mut flash_q {
        let a = bg.0.to_srgba().alpha;
        if a > 0.0 {
            let new_a = (a - time.delta_secs() * 2.5).max(0.0);
            bg.0 = Color::srgba(0.8, 0.0, 0.0, new_a);
        }
    }
}

// ─────────────────────────────────────────────────────────────
//  update_hud
// ─────────────────────────────────────────────────────────────
fn update_hud(
    phase: Res<GamePhase>,
    action: Res<PlayerAction>,
    inventory: Res<Inventory>,
    stats: Res<PlayerStats>,
    player_hp: Query<&Health, With<Player>>,
    player_tf: Query<&Transform, (With<Player>, Without<ExtractionZone>)>,
    zone_q: Query<&Transform, With<ExtractionZone>>,
    enemy_q: Query<&Transform, (With<Enemy>, Without<Player>)>,
    mut texts: ParamSet<(
        Query<&mut Text, With<HpBarText>>,
        Query<&mut Text, With<OreText>>,
        Query<&mut Text, With<StatusText>>,
        Query<&mut Text, With<MiningBarFill>>,
        Query<&mut Text, With<ExtractBar>>,
        Query<&mut Text, With<GameOverTitle>>,
        Query<&mut Text, With<ActionStateLabel>>,
    )>,
    mut hp_bar_fill: Query<&mut Node, (With<HpBarFill>, Without<ExtractBarFill>)>,
    mut extract_fill: Query<&mut Node, With<ExtractBarFill>>,
    mut overlay: Query<&mut Visibility, With<GameOverlay>>,
    mut state_panel: Query<
        (&mut Visibility, &mut BackgroundColor),
        (With<ActionStatePanel>, Without<GameOverlay>),
    >,
) {
    // HP
    if let Ok(hp) = player_hp.get_single() {
        let ratio = (hp.cur / hp.max).clamp(0.0, 1.0);
        for mut t in texts.p0().iter_mut() {
            **t = format!("HP  {:.0} / {:.0}", hp.cur, hp.max);
        }
        for mut node in &mut hp_bar_fill {
            node.width = Val::Percent(ratio * 100.0);
        }
    }

    // Ore
    for mut t in texts.p1().iter_mut() {
        **t = format!("Ore: {}  ({} gp)", inventory.total(), inventory.value());
    }

    // Mining level / status
    for mut t in texts.p2().iter_mut() {
        **t = format!("Mining Lv: {}  |  XP: {}", stats.level(), stats.mining_xp);
    }

    // Action label + bar
    for mut t in texts.p3().iter_mut() {
        **t = match &*action {
            PlayerAction::Free => String::new(),
            PlayerAction::Mining {
                ore,
                progress,
                total,
                ..
            } => {
                format!("Mining {}...  {:.0}%", ore.name(), progress / total * 100.0)
            }
            PlayerAction::Extracting { progress } => {
                format!("EXTRACTING...  {:.0}%", progress / EXTRACT_TIME * 100.0)
            }
        };
    }
    for mut node in &mut extract_fill {
        node.width = Val::Percent(match &*action {
            PlayerAction::Mining {
                progress, total, ..
            } => (progress / total * 100.0).min(100.0),
            PlayerAction::Extracting { progress } => (progress / EXTRACT_TIME * 100.0).min(100.0),
            _ => 0.0,
        });
    }

    // Extraction distance indicator
    if let Ok(ptf) = player_tf.get_single() {
        let nearest_dist = zone_q
            .iter()
            .map(|ztf| flat_diff(ptf.translation, ztf.translation).length())
            .fold(f32::MAX, f32::min);
        for mut t in texts.p4().iter_mut() {
            if nearest_dist < EXTRACT_RADIUS {
                **t = "STAND STILL TO EXTRACT".into();
            } else {
                **t = format!("EXTRACT  {:.0}m", nearest_dist);
            }
        }
    }

    // ── Action state banner ───────────────────────────────────
    // Determine state: Extracting > Combat > Mining > Idle
    let in_combat = if let Ok(ptf) = player_tf.get_single() {
        enemy_q
            .iter()
            .any(|etf| flat_diff(ptf.translation, etf.translation).length() < ATTACK_RANGE * 2.0)
    } else {
        false
    };

    enum StateKind {
        Idle,
        Mining,
        Combat,
        Extracting,
    }
    let state = match &*action {
        PlayerAction::Extracting { .. } => StateKind::Extracting,
        PlayerAction::Mining { .. } => StateKind::Mining,
        _ if in_combat => StateKind::Combat,
        _ => StateKind::Idle,
    };

    let (label, bg_color, text_color) = match state {
        StateKind::Idle => ("", Color::srgba(0.0, 0.0, 0.0, 0.0), Color::WHITE),
        StateKind::Mining => (
            "  MINING  ",
            Color::srgba(0.55, 0.38, 0.05, 0.88),
            Color::srgb(1.0, 0.88, 0.3),
        ),
        StateKind::Combat => (
            "  IN COMBAT  ",
            Color::srgba(0.55, 0.05, 0.05, 0.88),
            Color::srgb(1.0, 0.35, 0.35),
        ),
        StateKind::Extracting => (
            "  EXTRACTING  ",
            Color::srgba(0.05, 0.42, 0.12, 0.88),
            Color::srgb(0.4, 1.0, 0.55),
        ),
    };

    for (mut vis, mut bg) in &mut state_panel {
        *vis = if label.is_empty() {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
        *bg = BackgroundColor(bg_color);
    }
    for mut t in texts.p6().iter_mut() {
        **t = label.into();
    }

    // Game over overlay
    let show_overlay = *phase != GamePhase::Playing;
    for mut vis in &mut overlay {
        *vis = if show_overlay {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut t in texts.p5().iter_mut() {
        **t = match &*phase {
            GamePhase::Extracted => format!(
                "EXTRACTED!\nEscaped with {} ore  ({} gp)",
                inventory.total(),
                inventory.value()
            ),
            GamePhase::Dead => "YOU DIED\nAll ore lost.".into(),
            GamePhase::Playing => String::new(),
        };
    }
    // suppress unused variable warning for text_color (used in future styling)
    let _ = text_color;
}

// ─────────────────────────────────────────────────────────────
//  handle_game_over_input
// ─────────────────────────────────────────────────────────────
fn handle_game_over_input(
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<GamePhase>,
    mut should_reset: ResMut<ShouldReset>,
    mut app_exit: EventWriter<AppExit>,
) {
    if *phase == GamePhase::Playing {
        return;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        should_reset.0 = true;
    }
    if keys.just_pressed(KeyCode::Escape) {
        app_exit.send(AppExit::Success);
    }
}

// ─────────────────────────────────────────────────────────────
//  reset_game
// ─────────────────────────────────────────────────────────────
fn reset_game(
    mut should_reset: ResMut<ShouldReset>,
    mut commands: Commands,
    game_entities: Query<Entity, With<GameEntity>>,
    mut phase: ResMut<GamePhase>,
    mut inventory: ResMut<Inventory>,
    mut stats: ResMut<PlayerStats>,
    mut action: ResMut<PlayerAction>,
    mut click_target: ResMut<PlayerTarget>,
    mut click_indicators: ResMut<ClickIndicators>,
    mut chat_log: ResMut<ChatLog>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !should_reset.0 {
        return;
    }
    should_reset.0 = false;
    for e in &game_entities {
        commands.entity(e).despawn_recursive();
    }
    *phase = GamePhase::Playing;
    *inventory = Inventory::default();
    *stats = PlayerStats::default();
    *action = PlayerAction::Free;
    *click_target = PlayerTarget::None;
    *click_indicators = ClickIndicators::default();
    *chat_log = ChatLog::default();

    // Re-run setup inline (same as setup but without Startup)
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.55, 0.48, 0.38),
        brightness: 180.0,
    });
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 20.0, 21.5).looking_at(Vec3::new(0.0, 0.0, 4.0), Vec3::Y),
        GameEntity,
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 6_000.0,
            shadows_enabled: true,
            color: Color::srgb(0.88, 0.78, 0.62),
            ..default()
        },
        Transform::from_xyz(-8.0, 14.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
        GameEntity,
    ));
    for &[lx, lz] in &[
        [-15.0f32, 3.0],
        [15.0, 4.0],
        [-9.0, -10.0],
        [9.0, -10.0],
        [0.0, -20.0],
        [-6.0, 10.0],
        [6.0, 11.0],
    ] {
        commands.spawn((
            PointLight {
                intensity: 14_000.0,
                color: Color::srgb(1.0, 0.62, 0.18),
                range: 14.0,
                ..default()
            },
            Transform::from_xyz(lx, 2.5, lz),
            GameEntity,
        ));
    }

    spawn_world(&mut commands, &mut meshes, &mut materials);
    spawn_player(&mut commands, &mut meshes, &mut materials);
    spawn_enemies(&mut commands, &mut meshes, &mut materials);
    spawn_hud(&mut commands);
}

// ─────────────────────────────────────────────────────────────
//  Helpers
// ─────────────────────────────────────────────────────────────
fn flat_diff(from: Vec3, to: Vec3) -> Vec3 {
    Vec3::new(to.x - from.x, 0.0, to.z - from.z)
}
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}
fn face(tf: &mut Transform, dir: Vec3) {
    if dir.length() > 0.001 {
        tf.rotation = Quat::from_rotation_y(dir.x.atan2(dir.z));
    }
}
fn clamp_pos(tf: &mut Transform) {
    tf.translation.x = tf.translation.x.clamp(-BOUNDS_X, BOUNDS_X);
    tf.translation.z = tf.translation.z.clamp(BOUNDS_Z_MIN, BOUNDS_Z_MAX);
}

fn push_from_circle(mover_pos: Vec3, obs_pos: Vec3, obs_r: f32, mover_r: f32) -> Vec3 {
    let diff = Vec3::new(mover_pos.x - obs_pos.x, 0.0, mover_pos.z - obs_pos.z);
    let dist = diff.length();
    let min_dist = obs_r + mover_r;
    if dist < min_dist {
        if dist < 1e-4 {
            Vec3::new(min_dist, 0.0, 0.0)
        } else {
            diff.normalize() * (min_dist - dist)
        }
    } else {
        Vec3::ZERO
    }
}

fn push_from_obb(
    mover_pos: Vec3,
    box_tf: &Transform,
    half_x: f32,
    half_z: f32,
    mover_r: f32,
) -> Vec3 {
    let rel = Vec3::new(
        mover_pos.x - box_tf.translation.x,
        0.0,
        mover_pos.z - box_tf.translation.z,
    );
    let inv_rot = box_tf.rotation.inverse();
    let local = inv_rot * rel;
    let lx = local.x;
    let lz = local.z;

    let cx = lx.clamp(-half_x, half_x);
    let cz = lz.clamp(-half_z, half_z);
    let diff = Vec3::new(lx - cx, 0.0, lz - cz);
    let dist = diff.length();
    let min_dist = mover_r;

    if dist < min_dist {
        let push_local = if dist < 1e-4 {
            // Inside the box — push out along shortest axis
            let dx = half_x - lx.abs();
            let dz = half_z - lz.abs();
            if dx < dz {
                Vec3::new(lx.signum() * (dx + min_dist), 0.0, 0.0)
            } else {
                Vec3::new(0.0, 0.0, lz.signum() * (dz + min_dist))
            }
        } else {
            diff.normalize() * (min_dist - dist)
        };
        box_tf.rotation * push_local
    } else {
        Vec3::ZERO
    }
}

// ─────────────────────────────────────────────────────────────
//  resolve_collisions  (push movers out of static colliders + each other)
// ─────────────────────────────────────────────────────────────
fn resolve_collisions(
    static_q: Query<(&Transform, &Collider), (Without<Player>, Without<Enemy>)>,
    mut mover_q: Query<(Entity, &mut Transform), Or<(With<Player>, With<Enemy>)>>,
) {
    // Step 1: Push movers out of static colliders
    let entity_ids: Vec<Entity> = mover_q.iter().map(|(e, _)| e).collect();
    for entity in &entity_ids {
        let Ok((_, mut tf)) = mover_q.get_mut(*entity) else {
            continue;
        };
        let pos = tf.translation;
        let mut push = Vec3::ZERO;
        for (box_tf, collider) in &static_q {
            push += match collider {
                Collider::Circle(r) => push_from_circle(pos, box_tf.translation, *r, MOVER_RADIUS),
                Collider::Obb { half_x, half_z } => {
                    push_from_obb(pos, box_tf, *half_x, *half_z, MOVER_RADIUS)
                }
            };
        }
        if push.length_squared() > 1e-8 {
            tf.translation += push;
            clamp_pos(&mut tf);
        }
    }

    // Step 2: Push movers away from each other
    let movers: Vec<(Entity, Vec3)> = mover_q.iter().map(|(e, tf)| (e, tf.translation)).collect();
    let n = movers.len();
    let mut corrections = vec![Vec3::ZERO; n];
    let min_dist = MOVER_RADIUS * 2.0;
    for i in 0..n {
        for j in (i + 1)..n {
            let diff = Vec3::new(
                movers[i].1.x - movers[j].1.x,
                0.0,
                movers[i].1.z - movers[j].1.z,
            );
            let dist = diff.length();
            if dist < min_dist {
                let push = if dist < 1e-4 {
                    Vec3::new(min_dist * 0.5, 0.0, 0.0)
                } else {
                    diff.normalize() * (min_dist - dist) * 0.5
                };
                corrections[i] += push;
                corrections[j] -= push;
            }
        }
    }
    for (i, (entity, _)) in movers.iter().enumerate() {
        if corrections[i].length_squared() > 1e-8 {
            if let Ok((_, mut tf)) = mover_q.get_mut(*entity) {
                tf.translation += corrections[i];
                clamp_pos(&mut tf);
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────
//  update_splats  (floating damage / XP numbers)
// ─────────────────────────────────────────────────────────────
fn update_splats(
    mut commands: Commands,
    time: Res<Time>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut splat_q: Query<(Entity, &mut FloatingSplat, &mut Node, &mut TextColor)>,
) {
    let Ok((camera, cam_gtf)) = camera_q.get_single() else {
        return;
    };
    let dt = time.delta_secs();

    for (entity, mut splat, mut node, mut tc) in &mut splat_q {
        splat.timer += dt;
        splat.world_pos.y += 1.6 * dt;

        if splat.timer >= splat.max_time {
            commands.entity(entity).despawn();
            continue;
        }

        // Project to screen
        if let Ok(screen_pos) = camera.world_to_viewport(cam_gtf, splat.world_pos) {
            node.left = Val::Px(screen_pos.x - 12.0);
            node.top = Val::Px(screen_pos.y - 12.0);
        }

        // Fade out in the last half of lifetime
        let t = splat.timer / splat.max_time;
        let alpha = if t < 0.5 { 1.0 } else { 1.0 - (t - 0.5) * 2.0 };
        let c = splat.base_color.to_srgba();
        tc.0 = Color::srgba(c.red, c.green, c.blue, alpha);
    }
}

// ─────────────────────────────────────────────────────────────
//  update_chat_log  (refresh chat box text from ChatLog resource)
// ─────────────────────────────────────────────────────────────
fn update_chat_log(log: Res<ChatLog>, mut chat_q: Query<(&ChatLine, &mut Text, &mut TextColor)>) {
    for (line, mut text, mut color) in &mut chat_q {
        let n = log.messages.len();
        // line.0 == 0 → newest (last in vec), line.0 == N-1 → oldest (first in vec)
        if line.0 < n {
            let idx = n - 1 - line.0;
            **text = log.messages[idx].0.clone();
            color.0 = log.messages[idx].1;
        } else {
            **text = String::new();
        }
    }
}

// ─────────────────────────────────────────────────────────────
//  camera_follow
// ─────────────────────────────────────────────────────────────
fn camera_follow(
    player_q: Query<&Transform, (With<Player>, Without<Camera3d>)>,
    mut cam_q: Query<&mut Transform, With<Camera3d>>,
    time: Res<Time>,
) {
    let Ok(ptf) = player_q.get_single() else {
        return;
    };
    let Ok(mut ctf) = cam_q.get_single_mut() else {
        return;
    };
    let target = ptf.translation + CAM_OFFSET;
    let pos = ctf
        .translation
        .lerp(target, (time.delta_secs() * 7.0).min(1.0));
    let look = Vec3::new(ptf.translation.x, 0.0, ptf.translation.z - 1.0);
    *ctf = Transform::from_translation(pos).looking_at(look, Vec3::Y);
}
