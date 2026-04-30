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
    mut _commands: Commands,
    mut _meshes: ResMut<Assets<Mesh>>,
    mut _materials: ResMut<Assets<StandardMaterial>>,
) {
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
