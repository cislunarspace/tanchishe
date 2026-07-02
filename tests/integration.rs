use std::collections::VecDeque;
use std::time::Duration;

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::{App, Entity, Events, KeyCode, NextState, State};
use rand::rngs::StdRng;
use rand::SeedableRng;

use tanchishe::ai;
use tanchishe::app::{build_app, AutoPlayMode, ResetOnEnter, StepTimer};
use tanchishe::audio::{DeathEvent, FoodEaten};
use tanchishe::food::{spawn_food, Food};
use tanchishe::game::{step, Score};
use tanchishe::grid::{Direction, GridPosition, GRID_HEIGHT, GRID_WIDTH};
use tanchishe::high_score::{HighScore, HIGH_SCORE_FILE};
use tanchishe::input::key_to_direction;
use tanchishe::snake::{self, Snake};
use tanchishe::state::AppState;

/// 辅助：创建指定位置和方向的单段蛇
fn make_snake(x: i32, y: i32, dir: Direction) -> Snake {
    Snake {
        direction: dir,
        next_direction: dir,
        body: VecDeque::from([GridPosition::new(x, y)]),
    }
}

/// 辅助：执行 n 步移动，返回是否撞墙
fn run_steps(snake: &mut Snake, n: usize) -> bool {
    for _ in 0..n {
        if snake::advance_snake(snake) {
            return true;
        }
    }
    false
}

/// 输入能驱动蛇改变方向
#[test]
fn test_input_drives_direction_change() {
    let mut snake = snake::spawn_snake(); // 向右
    assert_eq!(snake.direction, Direction::Right);

    // 按下：不能直接改（与右不相反），应接受
    snake::change_direction(&mut snake, Direction::Down);
    assert_eq!(snake.next_direction, Direction::Down);

    // 执行一步后方向生效
    let _ = snake::advance_snake(&mut snake);
    assert_eq!(snake.direction, Direction::Down);

    // 按上：与下相反，应被拒绝
    snake::change_direction(&mut snake, Direction::Up);
    assert_eq!(snake.next_direction, Direction::Down);
}

/// 正常向右移动测试
#[test]
fn test_move_right() {
    let mut snake = make_snake(5, 5, Direction::Right);
    let hit = snake::advance_snake(&mut snake);
    assert!(!hit);
    assert_eq!(*snake.body.front().unwrap(), GridPosition::new(6, 5));
}

/// 反向输入过滤测试
#[test]
fn test_reverse_input_rejected() {
    let mut snake = snake::spawn_snake(); // 向右
    snake::change_direction(&mut snake, Direction::Left);
    // next_direction 应保持 Right
    assert_eq!(snake.next_direction, Direction::Right);
}

/// 碰撞检测：向上撞墙触发游戏结束
#[test]
fn test_wall_collision_triggers_game_over() {
    let mut snake = make_snake(5, 0, Direction::Up);
    let hit = snake::advance_snake(&mut snake);
    assert!(hit, "向上撞墙应返回 true");
}

/// 碰撞检测：向左撞墙
#[test]
fn test_wall_collision_left() {
    let mut snake = make_snake(0, 5, Direction::Left);
    let hit = snake::advance_snake(&mut snake);
    assert!(hit, "向左撞墙应返回 true");
}

/// 碰撞检测：向右撞墙
#[test]
fn test_wall_collision_right() {
    let mut snake = make_snake(GRID_WIDTH - 1, 5, Direction::Right);
    let hit = snake::advance_snake(&mut snake);
    assert!(hit, "向右撞墙应返回 true");
}

/// 碰撞检测：向下撞墙
#[test]
fn test_wall_collision_down() {
    let mut snake = make_snake(5, GRID_HEIGHT - 1, Direction::Down);
    let hit = snake::advance_snake(&mut snake);
    assert!(hit, "向下撞墙应返回 true");
}

/// 普通移动不触发游戏结束
#[test]
fn test_no_wall_in_center() {
    let mut snake = make_snake(15, 10, Direction::Right);
    let hit = run_steps(&mut snake, 5);
    assert!(!hit, "棋盘中部移动不应撞墙");
}

/// 方向变更后蛇按新方向移动
#[test]
fn test_direction_change_affects_movement() {
    let mut snake = make_snake(10, 10, Direction::Right);
    snake::change_direction(&mut snake, Direction::Down);
    let _ = snake::advance_snake(&mut snake);
    assert_eq!(*snake.body.front().unwrap(), GridPosition::new(10, 11));
}

/// 按键映射测试
#[test]
fn test_key_mapping_arrows() {
    assert_eq!(
        key_to_direction(bevy::prelude::KeyCode::ArrowUp),
        Some(Direction::Up)
    );
    assert_eq!(
        key_to_direction(bevy::prelude::KeyCode::ArrowLeft),
        Some(Direction::Left)
    );
}

#[test]
fn test_key_mapping_wasd() {
    assert_eq!(
        key_to_direction(bevy::prelude::KeyCode::KeyW),
        Some(Direction::Up)
    );
    assert_eq!(
        key_to_direction(bevy::prelude::KeyCode::KeyA),
        Some(Direction::Left)
    );
}

/// 吃到食物后蛇增长、加分
#[test]
fn test_eating_grows_snake_and_increases_score() {
    let mut snake = Snake {
        direction: Direction::Right,
        next_direction: Direction::Right,
        body: VecDeque::from([
            GridPosition::new(5, 5),
            GridPosition::new(4, 5),
            GridPosition::new(3, 5),
        ]),
    };
    let food = Food {
        position: GridPosition::new(6, 5),
    };
    let mut score = Score::default();

    let result = step(&mut snake, &food, &mut score);

    assert!(result.ate_food);
    assert_eq!(snake.body.len(), 4);
    assert_eq!(score.value, 10);
}

/// 食物不会生成在蛇身体上
#[test]
fn test_food_spawns_on_empty_cell() {
    let snake = Snake {
        direction: Direction::Right,
        next_direction: Direction::Right,
        body: VecDeque::from([
            GridPosition::new(5, 5),
            GridPosition::new(4, 5),
            GridPosition::new(3, 5),
        ]),
    };
    let mut rng = StdRng::seed_from_u64(123);
    let food = spawn_food(&snake, &mut rng).unwrap();
    assert!(!snake.body.contains(&food.position));
}

/// 棋盘满时无法生成食物，触发胜利条件
#[test]
fn test_victory_when_board_is_full() {
    let mut body = VecDeque::new();
    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            body.push_back(GridPosition::new(x, y));
        }
    }
    let snake = Snake {
        direction: Direction::Right,
        next_direction: Direction::Right,
        body,
    };
    let mut rng = StdRng::seed_from_u64(42);
    assert!(spawn_food(&snake, &mut rng).is_none());
}

/// 辅助：推进应用到目标状态
fn update_until_stable(app: &mut App, target: AppState, max_frames: usize) {
    for _ in 0..max_frames {
        app.update();
        if *app.world().resource::<State<AppState>>().get() == target {
            return;
        }
    }
    panic!("未能在 {max_frames} 帧内到达状态 {target:?}");
}

/// 完整状态切换链路：Menu → Playing → Paused → Playing → GameOver → Menu
#[test]
fn test_state_transition_chain() {
    let mut app = build_app(true);
    app.update();
    assert_eq!(
        app.world().resource::<State<AppState>>().get(),
        &AppState::Menu
    );

    // Menu → Playing
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Playing);
    update_until_stable(&mut app, AppState::Playing, 5);

    // Playing → Paused
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Paused);
    update_until_stable(&mut app, AppState::Paused, 5);

    // Paused → Playing（继续，不重开）
    app.world_mut().resource_mut::<ResetOnEnter>().0 = false;
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Playing);
    update_until_stable(&mut app, AppState::Playing, 5);

    // Playing → GameOver
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::GameOver);
    update_until_stable(&mut app, AppState::GameOver, 5);

    // GameOver → Menu
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Menu);
    update_until_stable(&mut app, AppState::Menu, 5);
}

/// 最高分持久化端到端：保存后能加载，且更新只在得分更高时发生
#[test]
fn test_high_score_persistence() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(HIGH_SCORE_FILE);

    let mut high = HighScore::load(&path);
    assert_eq!(high.value, 0);

    high.update(50);
    high.save(&path);

    let mut loaded = HighScore::load(&path);
    assert_eq!(loaded.value, 50);

    // 低分不刷新
    loaded.update(30);
    loaded.save(&path);
    let loaded2 = HighScore::load(&path);
    assert_eq!(loaded2.value, 50);
}

/// 音频触发逻辑：吃到食物时发送 FoodEaten 事件。
#[test]
fn test_food_eaten_event_is_sent() {
    let mut app = build_app(true);
    app.world_mut().resource_mut::<ResetOnEnter>().0 = true;
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Playing);
    update_until_stable(&mut app, AppState::Playing, 5);

    // 食物放在蛇头正前方，触发 FoodEaten。
    {
        let snake = app.world().resource::<Snake>();
        let head = *snake.body.front().unwrap();
        app.world_mut().resource_mut::<Food>().position = snake.next_direction.apply(&head);
    }
    app.world_mut()
        .resource_mut::<StepTimer>()
        .0
        .set_elapsed(Duration::from_secs_f32(0.099));
    app.update();

    let food_events = app.world().resource::<Events<FoodEaten>>();
    let mut reader = food_events.get_cursor();
    assert_eq!(reader.read(food_events).count(), 1);
}

/// 音频触发逻辑：死亡时发送 DeathEvent 事件。
#[test]
fn test_death_event_is_sent() {
    let mut app = build_app(true);
    app.world_mut().resource_mut::<ResetOnEnter>().0 = true;
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Playing);
    update_until_stable(&mut app, AppState::Playing, 5);

    // 把蛇头放到左边界朝左，触发 DeathEvent。
    {
        let mut snake = app.world_mut().resource_mut::<Snake>();
        snake.body.clear();
        snake.body.push_back(GridPosition::new(0, 5));
        snake.direction = Direction::Left;
        snake.next_direction = Direction::Left;
    }
    app.world_mut()
        .resource_mut::<StepTimer>()
        .0
        .set_elapsed(Duration::from_secs_f32(0.099));
    app.update();

    let death_events = app.world().resource::<Events<DeathEvent>>();
    let mut reader = death_events.get_cursor();
    assert_eq!(reader.read(death_events).count(), 1);
}

/// 自动玩家端到端：主菜单按 A 启动自动通关，推进足够帧数后进入 Victory，
/// 且蛇身占满棋盘、分数为吃满棋盘所得。
#[test]
fn autoplay_reaches_victory() {
    let mut app = build_app(true);
    app.update();
    assert_eq!(
        app.world().resource::<State<AppState>>().get(),
        &AppState::Menu
    );

    // 主菜单按 A 键启动自动通关（Fast 难度）。
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
    update_until_stable(&mut app, AppState::Playing, 10);
    assert!(app.world().resource::<AutoPlayMode>().0);

    // 缩短步进间隔，使测试在可接受时间内推进约 600 步。
    // 注意：吃到食物后 move_snake 会按当前速度重设间隔，因此每帧按当前
    // duration 设置 elapsed，确保计时器在本帧必然触发。
    app.world_mut()
        .resource_mut::<StepTimer>()
        .0
        .set_duration(Duration::from_secs_f32(0.001));

    let max_frames = 1000;
    for _ in 0..max_frames {
        // 为了让自动玩家在有限帧数内必然胜利，每帧把食物放到 AI 下一步要去的格子。
        // 这样自动玩家每步都吃并持续增长，仍然走哈密顿回路，最终占满棋盘触发胜利。
        {
            let snake = app.world().resource::<Snake>();
            if let Some(&head) = snake.body.front() {
                let next_pos = ai::next_direction(head).apply(&head);
                app.world_mut().resource_mut::<Food>().position = next_pos;
            }
        }
        {
            let duration = app.world().resource::<StepTimer>().0.duration();
            app.world_mut()
                .resource_mut::<StepTimer>()
                .0
                .set_elapsed(duration - Duration::from_nanos(1));
        }
        app.update();
        if *app.world().resource::<State<AppState>>().get() == AppState::Victory {
            break;
        }
    }

    assert_eq!(
        app.world().resource::<State<AppState>>().get(),
        &AppState::Victory,
        "自动玩家应在 {max_frames} 帧内进入 Victory"
    );

    let snake = app.world().resource::<Snake>();
    let score = app.world().resource::<Score>();
    let expected_cells = (GRID_WIDTH * GRID_HEIGHT) as usize;
    assert_eq!(
        snake.body.len(),
        expected_cells,
        "胜利时蛇身应占满 {GRID_WIDTH}×{GRID_HEIGHT} 棋盘"
    );
    assert_eq!(
        score.value,
        (expected_cells - 3) as u32 * 10,
        "胜利时分数应为吃满棋盘所得"
    );
}
