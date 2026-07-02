use std::path::Path;
use std::time::Duration;

use bevy::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::ai;
use crate::audio::{mute_button_text, DeathEvent, FoodEaten, GameAudioPlugin, MuteButtonText};
use crate::food::{spawn_food, Food};
use crate::game::{step, step_interval, Difficulty, Score};
use crate::grid::{CELL_SIZE, GRID_HEIGHT, GRID_WIDTH};
use crate::high_score::{HighScore, HIGH_SCORE_FILE};
use crate::input;
use crate::snake;
use crate::state::AppState;

/// 字体资源路径
const FONT_PATH: &str = "fonts/NotoSansSC-Regular.ttf";

/// UI 按钮标识
#[derive(Component, Clone, Copy, Debug)]
pub enum UiButton {
    Start,
    AutoPlay,
    DifficultySlow,
    DifficultyMedium,
    DifficultyFast,
    ToggleMute,
    Resume,
    Restart,
    MainMenu,
}

/// 蛇段精灵标记
#[derive(Component)]
pub struct SnakeSegment;

/// 食物精灵标记
#[derive(Component)]
pub struct FoodSprite;

/// HUD 分数文字
#[derive(Component)]
pub struct ScoreText;

/// HUD 最高分文字
#[derive(Component)]
pub struct HighScoreText;

/// 菜单根节点
#[derive(Component)]
pub struct MenuRoot;

/// 游戏世界实体标记，进入主菜单或重开时需要清理
#[derive(Component)]
pub struct GameEntity;

/// 暂停遮罩根节点
#[derive(Component)]
pub struct PauseRoot;

/// 游戏结束界面根节点
#[derive(Component)]
pub struct GameOverRoot;

/// 胜利界面根节点
#[derive(Component)]
pub struct VictoryRoot;

/// 步进计时器
#[derive(Resource)]
pub struct StepTimer(pub Timer);

/// 蛇实体列表，与网格坐标一一对应
#[derive(Resource)]
pub struct SnakeEntities {
    pub segments: Vec<Entity>,
}

/// 食物实体
#[derive(Resource)]
pub struct FoodEntity(pub Entity);

/// 游戏用随机数生成器
#[derive(Resource)]
pub struct GameRng(pub StdRng);

/// 进入 Playing 状态时是否需要重置游戏（重开为 true，继续为 false）
#[derive(Resource, Default)]
pub struct ResetOnEnter(pub bool);

/// 是否由自动玩家（哈密顿回路 AI）控制蛇
#[derive(Resource, Default)]
pub struct AutoPlayMode(pub bool);

/// 死亡动画计时器
#[derive(Resource)]
pub struct DeathAnimationTimer(pub Timer);

/// 游戏结束原因
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GameOverReason {
    #[default]
    Wall,
    SelfCollision,
}

/// 测试模式标记，用于关闭窗口与渲染相关系统
#[derive(Resource, Default)]
pub struct IsTesting(pub bool);

const NORMAL_BUTTON: Color = Color::srgb(0.2, 0.2, 0.2);
const HOVERED_BUTTON: Color = Color::srgb(0.35, 0.35, 0.35);

/// 条件：当前处于测试模式
fn is_testing(testing: Res<IsTesting>) -> bool {
    testing.0
}

/// 条件：自动玩家已开启
fn autoplay_enabled(autoplay: Res<AutoPlayMode>) -> bool {
    autoplay.0
}
const SELECTED_BUTTON: Color = Color::srgb(0.25, 0.45, 0.25);
const BORDER_COLOR: Color = Color::srgb(0.5, 0.5, 0.5);

/// 构建并运行生产环境应用
pub fn run() {
    build_app(false).run();
}

/// 构建应用。`testing` 为 true 时使用最小插件集，不创建窗口与渲染。
pub fn build_app(testing: bool) -> App {
    let mut app = App::new();

    if testing {
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::state::app::StatesPlugin)
            .add_plugins(bevy::input::InputPlugin);
    } else {
        app.add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "贪吃蛇".into(),
                    resolution: (
                        GRID_WIDTH as f32 * CELL_SIZE,
                        GRID_HEIGHT as f32 * CELL_SIZE,
                    )
                        .into(),
                    resizable: false,
                    ..default()
                }),
                ..default()
            }),
        );
    }

    app.add_plugins(GameAudioPlugin { enabled: !testing })
        .init_state::<AppState>()
        .insert_resource(HighScore::load(Path::new(HIGH_SCORE_FILE)))
        .insert_resource(Difficulty::Medium)
        .insert_resource(Score::default())
        .insert_resource(snake::spawn_snake())
        .insert_resource(SnakeEntities { segments: vec![] })
        .insert_resource(Food {
            position: crate::grid::GridPosition::new(0, 0),
        })
        .insert_resource(FoodEntity(Entity::from_raw(0)))
        .insert_resource(StepTimer(Timer::new(
            step_interval(Difficulty::Medium.base_speed()),
            TimerMode::Repeating,
        )))
        .insert_resource(GameRng(StdRng::from_entropy()))
        .insert_resource(input::InputQueue::default())
        .insert_resource(ResetOnEnter::default())
        .insert_resource(AutoPlayMode::default())
        .insert_resource(GameOverReason::default())
        .insert_resource(DeathAnimationTimer(Timer::new(
            Duration::from_secs_f32(0.3),
            TimerMode::Once,
        )))
        .insert_resource(IsTesting(testing))
        .add_systems(Startup, setup_camera.run_if(not(is_testing)))
        .add_systems(OnEnter(AppState::Menu), setup_menu.run_if(not(is_testing)))
        .add_systems(
            OnExit(AppState::Menu),
            cleanup::<MenuRoot>.run_if(not(is_testing)),
        )
        .add_systems(OnEnter(AppState::Playing), setup_game)
        .add_systems(
            OnExit(AppState::Playing),
            cleanup::<PauseRoot>.run_if(not(is_testing)),
        )
        .add_systems(OnEnter(AppState::Dying), start_death_animation)
        .add_systems(Update, death_animation.run_if(in_state(AppState::Dying)))
        .add_systems(
            OnEnter(AppState::Paused),
            setup_pause.run_if(not(is_testing)),
        )
        .add_systems(
            OnExit(AppState::Paused),
            cleanup::<PauseRoot>.run_if(not(is_testing)),
        )
        .add_systems(
            OnEnter(AppState::GameOver),
            setup_game_over.run_if(not(is_testing)),
        )
        .add_systems(
            OnExit(AppState::GameOver),
            cleanup::<GameOverRoot>.run_if(not(is_testing)),
        )
        .add_systems(
            OnEnter(AppState::Victory),
            setup_victory.run_if(not(is_testing)),
        )
        .add_systems(
            OnExit(AppState::Victory),
            cleanup::<VictoryRoot>.run_if(not(is_testing)),
        )
        .add_systems(
            Update,
            (
                menu_input.run_if(in_state(AppState::Menu)),
                menu_button_interaction.run_if(in_state(AppState::Menu)),
                update_difficulty_buttons.run_if(in_state(AppState::Menu)),
                button_hover.run_if(in_state(AppState::Menu)),
                playing_input.run_if(in_state(AppState::Playing)),
                handle_input
                    .run_if(in_state(AppState::Playing).and(not(autoplay_enabled)))
                    .before(apply_input_queue),
                apply_input_queue
                    .run_if(in_state(AppState::Playing).and(not(autoplay_enabled)))
                    .before(move_snake),
                ai_control_system
                    .run_if(in_state(AppState::Playing).and(autoplay_enabled))
                    .before(move_snake),
                move_snake.run_if(in_state(AppState::Playing)),
                sync_snake_entities.run_if(in_state(AppState::Playing).and(not(is_testing))),
                sync_food_entity.run_if(in_state(AppState::Playing).and(not(is_testing))),
                update_hud.run_if(
                    in_state(AppState::Playing)
                        .or(in_state(AppState::Paused))
                        .or(in_state(AppState::Dying))
                        .or(in_state(AppState::GameOver))
                        .or(in_state(AppState::Victory)),
                ),
                pause_button_interaction.run_if(in_state(AppState::Paused)),
                paused_input.run_if(in_state(AppState::Paused)),
                game_over_button_interaction
                    .run_if(in_state(AppState::GameOver).or(in_state(AppState::Victory))),
                game_over_input
                    .run_if(in_state(AppState::GameOver).or(in_state(AppState::Victory))),
            ),
        );

    app
}

/// 初始化相机
fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Transform::from_xyz(
            GRID_WIDTH as f32 * CELL_SIZE / 2.0,
            GRID_HEIGHT as f32 * CELL_SIZE / 2.0,
            0.0,
        ),
    ));
}

/// 清理带指定标记的实体及其子实体
fn cleanup<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

/// 生成一个按钮（含文字子节点）
fn spawn_button(parent: &mut ChildBuilder, font: &Handle<Font>, text: &str, button: UiButton) {
    parent
        .spawn((
            Button,
            button,
            Node {
                width: Val::Px(220.0),
                height: Val::Px(50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(NORMAL_BUTTON),
            BorderColor(BORDER_COLOR),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(text),
                TextFont {
                    font: font.clone(),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                TextLayout::new_with_justify(JustifyText::Center),
            ));
        });
}

/// 生成静音切换按钮，文字会随静音状态更新。
fn spawn_mute_button(parent: &mut ChildBuilder, font: &Handle<Font>, muted: bool) {
    parent
        .spawn((
            Button,
            UiButton::ToggleMute,
            Node {
                width: Val::Px(220.0),
                height: Val::Px(50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(NORMAL_BUTTON),
            BorderColor(BORDER_COLOR),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(mute_button_text(muted)),
                TextFont {
                    font: font.clone(),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                TextLayout::new_with_justify(JustifyText::Center),
                MuteButtonText,
            ));
        });
}

/// 生成一段居中的文字标签
fn spawn_label(parent: &mut ChildBuilder, font: &Handle<Font>, text: &str, font_size: f32) {
    parent.spawn((
        Text::new(text),
        TextFont {
            font: font.clone(),
            font_size,
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 1.0)),
        TextLayout::new_with_justify(JustifyText::Center),
    ));
}

/// 主菜单
fn setup_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    muted: Res<crate::audio::AudioMuted>,
    game_entities: Query<Entity, With<GameEntity>>,
    pause_root: Query<Entity, With<PauseRoot>>,
    game_over_root: Query<Entity, With<GameOverRoot>>,
    victory_root: Query<Entity, With<VictoryRoot>>,
) {
    // 从任何状态返回主菜单时，清理残留的游戏实体与界面
    for entity in game_entities
        .iter()
        .chain(pause_root.iter())
        .chain(game_over_root.iter())
        .chain(victory_root.iter())
    {
        commands.entity(entity).despawn_recursive();
    }

    let font = asset_server.load(FONT_PATH);
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(20.0),
                ..default()
            },
            MenuRoot,
        ))
        .with_children(|parent| {
            spawn_label(parent, &font, "贪吃蛇", 64.0);
            spawn_button(parent, &font, "开始游戏", UiButton::Start);
            spawn_button(parent, &font, "自动通关 (A)", UiButton::AutoPlay);
            spawn_label(parent, &font, "选择难度（1/2/3）", 24.0);
            parent
                .spawn((Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(10.0),
                    ..default()
                },))
                .with_children(|parent| {
                    spawn_button(parent, &font, "慢", UiButton::DifficultySlow);
                    spawn_button(parent, &font, "中", UiButton::DifficultyMedium);
                    spawn_button(parent, &font, "快", UiButton::DifficultyFast);
                });
            spawn_mute_button(parent, &font, muted.muted);
            spawn_label(parent, &font, "静音快捷键：Ctrl+M", 18.0);
        });
}

/// 游戏场景：棋盘、蛇、食物、HUD
#[allow(clippy::too_many_arguments)]
fn setup_game(
    mut commands: Commands,
    reset: Res<ResetOnEnter>,
    mut snake: ResMut<snake::Snake>,
    mut entities: ResMut<SnakeEntities>,
    mut score: ResMut<Score>,
    mut step_timer: ResMut<StepTimer>,
    difficulty: Res<Difficulty>,
    mut food: ResMut<Food>,
    mut food_entity: ResMut<FoodEntity>,
    mut rng: ResMut<GameRng>,
    mut queue: ResMut<input::InputQueue>,
    game_entities: Query<Entity, With<GameEntity>>,
    asset_server: Option<Res<AssetServer>>,
    testing: Res<IsTesting>,
) {
    // 进入游戏时清空输入队列，避免暂停/菜单/死亡前的残留输入影响本局
    queue.directions.clear();

    if !reset.0 {
        // 从暂停继续，游戏实体已经存在，无需重置
        return;
    }

    // 清理旧的游戏实体
    for entity in &game_entities {
        commands.entity(entity).despawn_recursive();
    }
    entities.segments.clear();

    // 重置游戏状态
    let new_snake = snake::spawn_snake();
    snake.direction = new_snake.direction;
    snake.next_direction = new_snake.next_direction;
    snake.body = new_snake.body;
    *score = Score::default();
    step_timer.0.reset();
    step_timer
        .0
        .set_duration(step_interval(difficulty.base_speed()));

    if testing.0 {
        // 测试模式下不生成精灵与 HUD
        if let Some(new_food) = spawn_food(&snake, &mut rng.0) {
            *food = new_food;
        }
        return;
    }

    // 棋盘背景
    commands.spawn((
        Sprite {
            color: Color::srgb(0.1, 0.1, 0.1),
            custom_size: Some(Vec2::new(
                GRID_WIDTH as f32 * CELL_SIZE,
                GRID_HEIGHT as f32 * CELL_SIZE,
            )),
            ..default()
        },
        Transform::from_xyz(
            GRID_WIDTH as f32 * CELL_SIZE / 2.0,
            GRID_HEIGHT as f32 * CELL_SIZE / 2.0,
            -1.0,
        ),
        GameEntity,
    ));

    // 蛇段精灵
    for (i, &pos) in snake.body.iter().enumerate() {
        let color = if i == 0 {
            Color::srgb(0.2, 0.8, 0.2)
        } else {
            Color::srgb(0.1, 0.6, 0.1)
        };
        let world = pos.to_world();
        let entity = commands
            .spawn((
                Sprite {
                    color,
                    custom_size: Some(Vec2::splat(CELL_SIZE - 1.0)),
                    ..default()
                },
                Transform::from_xyz(world.x, world.y, 0.0),
                Visibility::default(),
                SnakeSegment,
                pos,
                GameEntity,
            ))
            .id();
        entities.segments.push(entity);
    }

    // 食物
    if let Some(new_food) = spawn_food(&snake, &mut rng.0) {
        *food = new_food;
    }
    food_entity.0 = spawn_food_sprite(&mut commands, &food);

    // HUD
    let font = asset_server
        .expect("AssetServer 应在非测试模式下可用")
        .load(FONT_PATH);
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(40.0),
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(40.0),
                ..default()
            },
            GameEntity,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("分数：0"),
                TextFont {
                    font: font.clone(),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                TextLayout::new_with_justify(JustifyText::Center),
                ScoreText,
            ));
            parent.spawn((
                Text::new("最高分：0"),
                TextFont {
                    font: font.clone(),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                TextLayout::new_with_justify(JustifyText::Center),
                HighScoreText,
            ));
        });
}

/// 生成食物精灵
fn spawn_food_sprite(commands: &mut Commands, food: &Food) -> Entity {
    let world = food.position.to_world();
    commands
        .spawn((
            Sprite {
                color: Color::srgb(0.9, 0.2, 0.2),
                custom_size: Some(Vec2::splat(CELL_SIZE - 1.0)),
                ..default()
            },
            Transform::from_xyz(world.x, world.y, 0.0),
            Visibility::default(),
            FoodSprite,
            food.position,
            GameEntity,
        ))
        .id()
}

/// 暂停遮罩
fn setup_pause(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(FONT_PATH);
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            PauseRoot,
        ))
        .with_children(|parent| {
            spawn_label(parent, &font, "暂停", 48.0);
            spawn_button(parent, &font, "继续 (Esc/P)", UiButton::Resume);
            spawn_button(parent, &font, "重开 (R)", UiButton::Restart);
            spawn_button(parent, &font, "返回菜单 (M)", UiButton::MainMenu);
        });
}

/// 游戏结束界面
#[allow(clippy::too_many_arguments)]
fn setup_game_over(
    mut commands: Commands,
    score: Res<Score>,
    mut high_score: ResMut<HighScore>,
    reason: Res<GameOverReason>,
    asset_server: Res<AssetServer>,
) {
    let new_record = high_score.update(score.value);
    high_score.save(Path::new(HIGH_SCORE_FILE));

    let title = match *reason {
        GameOverReason::Wall => "撞墙了！",
        GameOverReason::SelfCollision => "撞到自己了！",
    };

    let font = asset_server.load(FONT_PATH);
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(15.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            GameOverRoot,
        ))
        .with_children(|parent| {
            spawn_label(parent, &font, title, 48.0);
            spawn_label(parent, &font, &format!("本局得分：{}", score.value), 32.0);
            if new_record {
                spawn_label(parent, &font, "新纪录！", 28.0);
            }
            spawn_label(
                parent,
                &font,
                &format!("最高分：{}", high_score.value),
                28.0,
            );
            spawn_button(parent, &font, "重开 (R)", UiButton::Restart);
            spawn_button(parent, &font, "返回菜单 (M)", UiButton::MainMenu);
        });
}

/// 胜利界面
fn setup_victory(
    mut commands: Commands,
    score: Res<Score>,
    mut high_score: ResMut<HighScore>,
    asset_server: Res<AssetServer>,
) {
    let new_record = high_score.update(score.value);
    high_score.save(Path::new(HIGH_SCORE_FILE));

    let font = asset_server.load(FONT_PATH);
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(15.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            VictoryRoot,
        ))
        .with_children(|parent| {
            spawn_label(parent, &font, "胜利！", 48.0);
            spawn_label(parent, &font, &format!("本局得分：{}", score.value), 32.0);
            if new_record {
                spawn_label(parent, &font, "新纪录！", 28.0);
            }
            spawn_label(
                parent,
                &font,
                &format!("最高分：{}", high_score.value),
                28.0,
            );
            spawn_button(parent, &font, "重开 (R)", UiButton::Restart);
            spawn_button(parent, &font, "返回菜单 (M)", UiButton::MainMenu);
        });
}

/// 菜单键盘输入
fn menu_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut difficulty: ResMut<Difficulty>,
    mut reset: ResMut<ResetOnEnter>,
    mut autoplay: ResMut<AutoPlayMode>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keys.just_pressed(KeyCode::Digit1) {
        *difficulty = Difficulty::Slow;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        *difficulty = Difficulty::Medium;
    }
    if keys.just_pressed(KeyCode::Digit3) {
        *difficulty = Difficulty::Fast;
    }
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        autoplay.0 = false;
        reset.0 = true;
        next_state.set(AppState::Playing);
    }
    if keys.just_pressed(KeyCode::KeyA) {
        autoplay.0 = true;
        *difficulty = Difficulty::Fast;
        reset.0 = true;
        next_state.set(AppState::Playing);
    }
}

/// 菜单按钮交互
fn menu_button_interaction(
    mut interactions: Query<(&Interaction, &UiButton), Changed<Interaction>>,
    mut difficulty: ResMut<Difficulty>,
    mut reset: ResMut<ResetOnEnter>,
    mut autoplay: ResMut<AutoPlayMode>,
    mut next_state: ResMut<NextState<AppState>>,
    mut muted: ResMut<crate::audio::AudioMuted>,
    mut mute_text: Query<&mut Text, With<MuteButtonText>>,
) {
    for (interaction, button) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button {
            UiButton::Start => {
                autoplay.0 = false;
                reset.0 = true;
                next_state.set(AppState::Playing);
            }
            UiButton::AutoPlay => {
                autoplay.0 = true;
                *difficulty = Difficulty::Fast;
                reset.0 = true;
                next_state.set(AppState::Playing);
            }
            UiButton::DifficultySlow => *difficulty = Difficulty::Slow,
            UiButton::DifficultyMedium => *difficulty = Difficulty::Medium,
            UiButton::DifficultyFast => *difficulty = Difficulty::Fast,
            UiButton::ToggleMute => {
                muted.muted = !muted.muted;
                for mut text in &mut mute_text {
                    text.0 = mute_button_text(muted.muted);
                }
            }
            _ => {}
        }
    }
}

/// 根据当前难度更新难度按钮颜色
fn update_difficulty_buttons(
    difficulty: Res<Difficulty>,
    mut buttons: Query<(&UiButton, &mut BackgroundColor)>,
) {
    for (button, mut bg) in &mut buttons {
        let selected = matches!(
            (button, *difficulty),
            (UiButton::DifficultySlow, Difficulty::Slow)
                | (UiButton::DifficultyMedium, Difficulty::Medium)
                | (UiButton::DifficultyFast, Difficulty::Fast)
        );
        *bg = if selected {
            SELECTED_BUTTON.into()
        } else {
            NORMAL_BUTTON.into()
        };
    }
}

/// 非难度按钮的悬停效果
fn button_hover(
    mut interactions: Query<(&Interaction, &UiButton, &mut BackgroundColor), Changed<Interaction>>,
) {
    for (interaction, button, mut bg) in &mut interactions {
        if matches!(
            button,
            UiButton::DifficultySlow | UiButton::DifficultyMedium | UiButton::DifficultyFast
        ) {
            continue;
        }
        match interaction {
            Interaction::Pressed | Interaction::Hovered => *bg = HOVERED_BUTTON.into(),
            Interaction::None => *bg = NORMAL_BUTTON.into(),
        }
    }
}

/// 游戏进行中键盘输入
fn playing_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut reset: ResMut<ResetOnEnter>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::KeyP) {
        reset.0 = false;
        next_state.set(AppState::Paused);
    }
    if keys.just_pressed(KeyCode::KeyR) {
        reset.0 = true;
        next_state.set(AppState::Playing);
    }
    if keys.just_pressed(KeyCode::KeyM) {
        next_state.set(AppState::Menu);
    }
}

/// 暂停界面键盘输入
fn paused_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut reset: ResMut<ResetOnEnter>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::KeyP) {
        reset.0 = false;
        next_state.set(AppState::Playing);
    }
    if keys.just_pressed(KeyCode::KeyR) {
        reset.0 = true;
        next_state.set(AppState::Playing);
    }
    if keys.just_pressed(KeyCode::KeyM) {
        next_state.set(AppState::Menu);
    }
}

/// 暂停按钮交互
fn pause_button_interaction(
    mut interactions: Query<(&Interaction, &UiButton), Changed<Interaction>>,
    mut reset: ResMut<ResetOnEnter>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, button) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button {
            UiButton::Resume => {
                reset.0 = false;
                next_state.set(AppState::Playing);
            }
            UiButton::Restart => {
                reset.0 = true;
                next_state.set(AppState::Playing);
            }
            UiButton::MainMenu => {
                next_state.set(AppState::Menu);
            }
            _ => {}
        }
    }
}

/// 游戏结束/胜利界面键盘输入
fn game_over_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut reset: ResMut<ResetOnEnter>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        reset.0 = true;
        next_state.set(AppState::Playing);
    }
    if keys.just_pressed(KeyCode::KeyM) {
        next_state.set(AppState::Menu);
    }
}

/// 游戏结束/胜利按钮交互
fn game_over_button_interaction(
    mut interactions: Query<(&Interaction, &UiButton), Changed<Interaction>>,
    mut reset: ResMut<ResetOnEnter>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, button) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button {
            UiButton::Restart => {
                reset.0 = true;
                next_state.set(AppState::Playing);
            }
            UiButton::MainMenu => {
                next_state.set(AppState::Menu);
            }
            _ => {}
        }
    }
}

/// 读取方向输入并入队
fn handle_input(keys: Res<ButtonInput<KeyCode>>, mut queue: ResMut<input::InputQueue>) {
    input::handle_direction_input(&keys, &mut queue);
}

/// 从输入队列中取出一个有效方向应用到蛇
fn apply_input_queue(mut queue: ResMut<input::InputQueue>, mut snake: ResMut<snake::Snake>) {
    input::apply_input_queue(&mut queue, &mut snake);
}

/// 自动玩家：根据哈密顿回路决定蛇的下一步方向
fn ai_control_system(mut snake: ResMut<snake::Snake>) {
    if let Some(&head) = snake.body.front() {
        snake.next_direction = ai::next_direction(head);
    }
}

/// 定时移动蛇
#[allow(clippy::too_many_arguments)]
fn move_snake(
    time: Res<Time>,
    mut snake: ResMut<snake::Snake>,
    mut food: ResMut<Food>,
    mut food_entity: ResMut<FoodEntity>,
    mut next_state: ResMut<NextState<AppState>>,
    mut step_timer: ResMut<StepTimer>,
    mut score: ResMut<Score>,
    difficulty: Res<Difficulty>,
    mut rng: ResMut<GameRng>,
    mut commands: Commands,
    mut reason: ResMut<GameOverReason>,
    mut food_eaten_events: EventWriter<FoodEaten>,
    mut death_events: EventWriter<DeathEvent>,
    testing: Res<IsTesting>,
) {
    if !step_timer.0.tick(time.delta()).just_finished() {
        return;
    }

    let result = step(&mut snake, &food, &mut score);

    if result.hit_wall {
        *reason = GameOverReason::Wall;
        death_events.send(DeathEvent);
        next_state.set(AppState::Dying);
        return;
    }

    if result.hit_self {
        *reason = GameOverReason::SelfCollision;
        death_events.send(DeathEvent);
        next_state.set(AppState::Dying);
        return;
    }

    if result.ate_food {
        food_eaten_events.send(FoodEaten);
        if let Some(new_food) = spawn_food(&snake, &mut rng.0) {
            *food = new_food;
            if !testing.0 {
                commands.entity(food_entity.0).despawn();
                food_entity.0 = spawn_food_sprite(&mut commands, &food);
            }

            let speed = crate::game::calculate_speed(&difficulty, snake.body.len());
            step_timer.0.set_duration(step_interval(speed));
        } else {
            next_state.set(AppState::Victory);
            if !testing.0 {
                commands.entity(food_entity.0).despawn();
            }
        }
    }
}

/// 同步蛇的网格坐标到 ECS 实体
fn sync_snake_entities(
    mut commands: Commands,
    snake: Res<snake::Snake>,
    mut entities: ResMut<SnakeEntities>,
    mut transforms: Query<&mut Transform>,
) {
    if !snake.is_changed() {
        return;
    }

    let body: Vec<_> = snake.body.iter().copied().collect();

    for (i, &pos) in body.iter().enumerate() {
        if let Some(&entity) = entities.segments.get(i) {
            if let Ok(mut t) = transforms.get_mut(entity) {
                let world = pos.to_world();
                t.translation.x = world.x;
                t.translation.y = world.y;
            }
        }
    }

    for &pos in body.iter().skip(entities.segments.len()) {
        let world = pos.to_world();
        let entity = commands
            .spawn((
                Sprite {
                    color: Color::srgb(0.1, 0.6, 0.1),
                    custom_size: Some(Vec2::splat(CELL_SIZE - 1.0)),
                    ..default()
                },
                Transform::from_xyz(world.x, world.y, 0.0),
                Visibility::default(),
                SnakeSegment,
                pos,
                GameEntity,
            ))
            .id();
        entities.segments.push(entity);
    }

    while entities.segments.len() > body.len() {
        if let Some(entity) = entities.segments.pop() {
            commands.entity(entity).despawn();
        }
    }
}

/// 同步食物位置到食物精灵
fn sync_food_entity(
    food: Res<Food>,
    food_entity: Res<FoodEntity>,
    mut transforms: Query<&mut Transform>,
) {
    if food.is_changed() {
        if let Ok(mut t) = transforms.get_mut(food_entity.0) {
            let world = food.position.to_world();
            t.translation.x = world.x;
            t.translation.y = world.y;
        }
    }
}

/// 死亡动画开始：将蛇头变红并重置计时器。
fn start_death_animation(
    mut commands: Commands,
    entities: Res<SnakeEntities>,
    mut sprites: Query<&mut Sprite>,
) {
    if let Some(&head) = entities.segments.first() {
        if let Ok(mut sprite) = sprites.get_mut(head) {
            sprite.color = Color::srgb(1.0, 0.0, 0.0);
        }
    }
    commands.insert_resource(DeathAnimationTimer(Timer::new(
        Duration::from_secs_f32(0.3),
        TimerMode::Once,
    )));
}

/// 死亡动画更新：蛇头红/透明交替闪烁，计时结束后进入 GameOver。
fn death_animation(
    time: Res<Time>,
    mut timer: ResMut<DeathAnimationTimer>,
    entities: Res<SnakeEntities>,
    mut sprites: Query<&mut Sprite>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    timer.0.tick(time.delta());

    let flash = (timer.0.elapsed().as_secs_f32() * 20.0).sin() > 0.0;
    if let Some(&head) = entities.segments.first() {
        if let Ok(mut sprite) = sprites.get_mut(head) {
            sprite.color = if flash {
                Color::srgb(1.0, 0.0, 0.0)
            } else {
                Color::srgba(1.0, 0.0, 0.0, 0.3)
            };
        }
    }

    if timer.0.just_finished() {
        next_state.set(AppState::GameOver);
    }
}

/// 更新 HUD 分数与最高分显示
fn update_hud(
    score: Res<Score>,
    high_score: Res<HighScore>,
    mut score_query: Query<&mut Text, (With<ScoreText>, Without<HighScoreText>)>,
    mut high_score_query: Query<&mut Text, (With<HighScoreText>, Without<ScoreText>)>,
) {
    if score.is_changed() || high_score.is_changed() {
        for mut text in &mut score_query {
            text.0 = format!("分数：{}", score.value);
        }
        for mut text in &mut high_score_query {
            text.0 = format!("最高分：{}", high_score.value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 辅助：将应用推进到目标状态
    fn update_until_stable(app: &mut App, target: AppState, max_frames: usize) {
        for _ in 0..max_frames {
            app.update();
            if *app.world().resource::<State<AppState>>().get() == target {
                return;
            }
        }
        panic!("未能在 {max_frames} 帧内到达状态 {target:?}");
    }

    /// 应用启动后默认处于 Menu 状态
    #[test]
    fn test_app_starts_in_menu() {
        let mut app = build_app(true);
        app.update();
        assert_eq!(
            app.world().resource::<State<AppState>>().get(),
            &AppState::Menu
        );
    }

    /// Menu → Playing 状态切换
    #[test]
    fn test_menu_to_playing() {
        let mut app = build_app(true);
        app.update();

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Playing);
        update_until_stable(&mut app, AppState::Playing, 5);
    }

    /// Playing → Paused → Playing 状态切换
    #[test]
    fn test_playing_pause_resume() {
        let mut app = build_app(true);
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Playing);
        update_until_stable(&mut app, AppState::Playing, 5);

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Paused);
        update_until_stable(&mut app, AppState::Paused, 5);

        // 继续时不清除游戏状态
        app.world_mut().resource_mut::<ResetOnEnter>().0 = false;
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Playing);
        update_until_stable(&mut app, AppState::Playing, 5);
    }

    /// Playing → GameOver 状态切换
    #[test]
    fn test_playing_to_game_over() {
        let mut app = build_app(true);
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Playing);
        update_until_stable(&mut app, AppState::Playing, 5);

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::GameOver);
        update_until_stable(&mut app, AppState::GameOver, 5);
    }

    /// GameOver → Menu 状态切换
    #[test]
    fn test_game_over_to_menu() {
        let mut app = build_app(true);
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::GameOver);
        update_until_stable(&mut app, AppState::GameOver, 5);

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Menu);
        update_until_stable(&mut app, AppState::Menu, 5);
    }

    /// 重开时 ResetOnEnter 应为 true
    #[test]
    fn test_restart_sets_reset_flag() {
        let mut app = build_app(true);
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::GameOver);
        update_until_stable(&mut app, AppState::GameOver, 5);

        app.world_mut().resource_mut::<ResetOnEnter>().0 = true;
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Playing);
        update_until_stable(&mut app, AppState::Playing, 5);
    }

    /// 吃到食物时触发 FoodEaten 事件。
    #[test]
    fn test_food_eaten_event_sent() {
        let mut app = build_app(true);
        app.world_mut().resource_mut::<ResetOnEnter>().0 = true;
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Playing);
        update_until_stable(&mut app, AppState::Playing, 5);

        // 把食物放在蛇头正前方，确保下一步吃到。
        {
            let snake = app.world().resource::<snake::Snake>();
            let head = *snake.body.front().unwrap();
            let next = snake.next_direction.apply(&head);
            app.world_mut().resource_mut::<Food>().position = next;
        }

        // 让步进计时器即将触发，并推进时间。
        app.world_mut()
            .resource_mut::<StepTimer>()
            .0
            .set_elapsed(Duration::from_secs_f32(0.099));
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .advance_by(Duration::from_secs_f32(0.2));
        app.update();

        let events = app.world().resource::<Events<FoodEaten>>();
        let mut reader = events.get_cursor();
        assert_eq!(reader.read(events).count(), 1);
    }

    /// 撞墙时触发 DeathEvent 并进入 Dying 状态。
    #[test]
    fn test_death_event_and_dying_state() {
        let mut app = build_app(true);
        app.world_mut().resource_mut::<ResetOnEnter>().0 = true;
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Playing);
        update_until_stable(&mut app, AppState::Playing, 5);

        // 把蛇头放在左边界并朝左，确保下一步撞墙。
        {
            let mut snake = app.world_mut().resource_mut::<snake::Snake>();
            snake.body.clear();
            snake.body.push_back(crate::grid::GridPosition::new(0, 5));
            snake.direction = crate::grid::Direction::Left;
            snake.next_direction = crate::grid::Direction::Left;
        }

        app.world_mut()
            .resource_mut::<StepTimer>()
            .0
            .set_elapsed(Duration::from_secs_f32(0.099));
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .advance_by(Duration::from_secs_f32(0.2));
        app.update();

        let events = app.world().resource::<Events<DeathEvent>>();
        let mut reader = events.get_cursor();
        assert_eq!(reader.read(events).count(), 1);

        // 在 Update 中设置 NextState，需要再推进一帧才会真正切换。
        app.update();
        assert_eq!(
            app.world().resource::<State<AppState>>().get(),
            &AppState::Dying
        );
    }

    /// 死亡动画计时结束后自动进入 GameOver。
    #[test]
    fn test_dying_animation_transitions_to_game_over() {
        let mut app = build_app(true);
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Dying);
        update_until_stable(&mut app, AppState::Dying, 5);

        // 将死亡动画计时器推到即将结束，然后等待它自动切换到 GameOver。
        app.world_mut()
            .resource_mut::<DeathAnimationTimer>()
            .0
            .set_elapsed(Duration::from_secs_f32(0.299));
        update_until_stable(&mut app, AppState::GameOver, 10);
    }

    /// 自动通关模式默认关闭。
    #[test]
    fn test_autoplay_mode_defaults_to_false() {
        let mut app = build_app(true);
        app.update();
        assert!(!app.world().resource::<AutoPlayMode>().0);
    }

    /// 主菜单按 A 键启动自动通关，难度为 Fast 并进入 Playing。
    #[test]
    fn test_autoplay_key_a_starts_game() {
        use bevy::input::keyboard::{Key, KeyboardInput};
        use bevy::input::ButtonState;

        let mut app = build_app(true);
        app.update();

        let window = Entity::from_raw(1);
        {
            let mut events = app.world_mut().resource_mut::<Events<KeyboardInput>>();
            events.send(KeyboardInput {
                key_code: KeyCode::KeyA,
                logical_key: Key::Character("a".into()),
                state: ButtonState::Pressed,
                window,
                repeat: false,
            });
        }
        update_until_stable(&mut app, AppState::Playing, 5);

        assert!(app.world().resource::<AutoPlayMode>().0);
        assert_eq!(*app.world().resource::<Difficulty>(), Difficulty::Fast);
        assert!(app.world().resource::<ResetOnEnter>().0);
    }

    /// 自动玩家开启时，AI 控制系统会按哈密顿回路设置蛇的 next_direction。
    #[test]
    fn test_ai_control_system_sets_direction() {
        let mut app = build_app(true);
        app.world_mut().resource_mut::<ResetOnEnter>().0 = true;
        app.world_mut().resource_mut::<AutoPlayMode>().0 = true;
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Playing);
        update_until_stable(&mut app, AppState::Playing, 5);

        let snake = app.world().resource::<snake::Snake>();
        let head = *snake.body.front().unwrap();
        assert_eq!(snake.next_direction, ai::next_direction(head));
    }

    /// Ctrl+M 切换全局静音状态。
    #[test]
    fn test_mute_toggle() {
        use bevy::input::keyboard::{Key, KeyboardInput};
        use bevy::input::ButtonState;

        let mut app = build_app(true);
        app.update();

        assert!(!app.world().resource::<crate::audio::AudioMuted>().muted);

        let window = Entity::from_raw(1);
        {
            let mut events = app.world_mut().resource_mut::<Events<KeyboardInput>>();
            events.send(KeyboardInput {
                key_code: KeyCode::ControlLeft,
                logical_key: Key::Control,
                state: ButtonState::Pressed,
                window,
                repeat: false,
            });
            events.send(KeyboardInput {
                key_code: KeyCode::KeyM,
                logical_key: Key::Character("m".into()),
                state: ButtonState::Pressed,
                window,
                repeat: false,
            });
        }
        app.update();

        assert!(app.world().resource::<crate::audio::AudioMuted>().muted);

        {
            let mut events = app.world_mut().resource_mut::<Events<KeyboardInput>>();
            events.send(KeyboardInput {
                key_code: KeyCode::KeyM,
                logical_key: Key::Character("m".into()),
                state: ButtonState::Released,
                window,
                repeat: false,
            });
            events.send(KeyboardInput {
                key_code: KeyCode::ControlLeft,
                logical_key: Key::Control,
                state: ButtonState::Released,
                window,
                repeat: false,
            });
            events.send(KeyboardInput {
                key_code: KeyCode::ControlLeft,
                logical_key: Key::Control,
                state: ButtonState::Pressed,
                window,
                repeat: false,
            });
            events.send(KeyboardInput {
                key_code: KeyCode::KeyM,
                logical_key: Key::Character("m".into()),
                state: ButtonState::Pressed,
                window,
                repeat: false,
            });
        }
        app.update();

        assert!(!app.world().resource::<crate::audio::AudioMuted>().muted);
    }
}
