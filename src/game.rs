use std::time::Duration;

use bevy::prelude::Resource;

use crate::food::Food;
use crate::grid::{is_out_of_bounds, GridPosition};
use crate::snake::{advance_snake_with_growth, Snake};

/// 难度，决定基础速度
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Difficulty {
    Slow,
    #[default]
    Medium,
    Fast,
}

impl Difficulty {
    /// 基础速度（步/秒）
    pub fn base_speed(&self) -> f32 {
        match self {
            Difficulty::Slow => 6.0,
            Difficulty::Medium => 10.0,
            Difficulty::Fast => 14.0,
        }
    }
}

/// 当前一局分数
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Score {
    pub value: u32,
}

/// 速度上限倍数（相对于基础速度）
const MAX_SPEED_MULTIPLIER: f32 = 2.0;

/// 蛇初始长度
const SNAKE_INITIAL_LENGTH: usize = 3;

/// 根据难度和蛇身长度计算当前速度。
/// 长度每比初始长度增加 5 段，速度提升 10%，但不超过基础速度的 2 倍。
pub fn calculate_speed(difficulty: &Difficulty, snake_length: usize) -> f32 {
    let base = difficulty.base_speed();
    let speed_ups = (snake_length.saturating_sub(SNAKE_INITIAL_LENGTH)) / 5;
    let multiplier = 1.1_f32.powi(speed_ups as i32);
    (base * multiplier).min(base * MAX_SPEED_MULTIPLIER)
}

/// 由速度计算步进间隔
pub fn step_interval(speed: f32) -> Duration {
    Duration::from_secs_f32(1.0 / speed)
}

/// 单步结果
#[derive(Debug, PartialEq, Eq)]
pub struct StepResult {
    pub hit_wall: bool,
    pub hit_self: bool,
    pub ate_food: bool,
}

/// 判断新蛇头是否会撞到自己的身体。
/// 不增长时尾部会离开，因此移动到原尾部位置不算碰撞；
/// 增长时尾部保留，所以移动到原尾部位置仍算碰撞。
fn is_self_collision(snake: &Snake, new_head: &GridPosition, grow: bool) -> bool {
    snake
        .body
        .iter()
        .enumerate()
        .any(|(i, pos)| pos == new_head && (grow || i != snake.body.len() - 1))
}

/// 执行一次步进：应用方向、检测撞墙/撞自己、判断是否吃到食物并增长、加分。
pub fn step(snake: &mut Snake, food: &Food, score: &mut Score) -> StepResult {
    let new_head = snake.next_direction.apply(snake.body.front().unwrap());

    if is_out_of_bounds(&new_head) {
        return StepResult {
            hit_wall: true,
            hit_self: false,
            ate_food: false,
        };
    }

    let grow = new_head == food.position;

    if is_self_collision(snake, &new_head, grow) {
        return StepResult {
            hit_wall: false,
            hit_self: true,
            ate_food: false,
        };
    }

    advance_snake_with_growth(snake, grow);

    if grow {
        score.value += 10;
    }

    StepResult {
        hit_wall: false,
        hit_self: false,
        ate_food: grow,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use crate::grid::{Direction, GridPosition};

    use super::*;

    /// 难度基础速度
    #[test]
    fn test_difficulty_base_speed() {
        assert_eq!(Difficulty::Slow.base_speed(), 6.0);
        assert_eq!(Difficulty::Medium.base_speed(), 10.0);
        assert_eq!(Difficulty::Fast.base_speed(), 14.0);
    }

    /// 速度计算：初始长度无加速
    #[test]
    fn test_speed_initial_length() {
        assert_eq!(calculate_speed(&Difficulty::Medium, 3), 10.0);
    }

    /// 速度计算：每增加 5 段提速 10%
    #[test]
    fn test_speed_increases_every_five_segments() {
        assert!(
            (calculate_speed(&Difficulty::Medium, 8) - 11.0).abs() < f32::EPSILON,
            "8 段应比基础快 10%"
        );
        assert!(
            (calculate_speed(&Difficulty::Medium, 13) - 12.1).abs() < 0.001,
            "13 段应比基础快 21%"
        );
    }

    /// 速度计算：未达 5 段不提速
    #[test]
    fn test_speed_no_boost_between_thresholds() {
        assert_eq!(calculate_speed(&Difficulty::Medium, 7), 10.0);
    }

    /// 速度计算：上限
    #[test]
    fn test_speed_cap() {
        let speed = calculate_speed(&Difficulty::Medium, 100);
        assert!(
            (speed - 20.0).abs() < f32::EPSILON,
            "速度应被限制在 2 倍基础速度"
        );
    }

    /// 步进间隔与速度互为倒数
    #[test]
    fn test_step_interval() {
        let interval = step_interval(10.0);
        assert!((interval.as_secs_f32() - 0.1).abs() < f32::EPSILON);
    }

    /// 正常步进不撞墙、不吃食物
    #[test]
    fn test_step_normal() {
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
            position: GridPosition::new(10, 10),
        };
        let mut score = Score::default();
        let result = step(&mut snake, &food, &mut score);
        assert_eq!(
            result,
            StepResult {
                hit_wall: false,
                hit_self: false,
                ate_food: false
            }
        );
        assert_eq!(snake.body.len(), 3);
        assert_eq!(score.value, 0);
    }

    /// 吃到食物时增长并加分
    #[test]
    fn test_step_eat_food() {
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
        assert_eq!(
            result,
            StepResult {
                hit_wall: false,
                hit_self: false,
                ate_food: true
            }
        );
        assert_eq!(snake.body.len(), 4);
        assert_eq!(score.value, 10);
        assert_eq!(*snake.body.front().unwrap(), GridPosition::new(6, 5));
    }

    /// 撞墙时返回游戏结束
    #[test]
    fn test_step_hit_wall() {
        let mut snake = Snake {
            direction: Direction::Left,
            next_direction: Direction::Left,
            body: VecDeque::from([GridPosition::new(0, 5)]),
        };
        let food = Food {
            position: GridPosition::new(10, 10),
        };
        let mut score = Score::default();
        let result = step(&mut snake, &food, &mut score);
        assert_eq!(
            result,
            StepResult {
                hit_wall: true,
                hit_self: false,
                ate_food: false
            }
        );
        assert_eq!(score.value, 0);
    }

    /// 撞到自己时返回游戏结束
    #[test]
    fn test_step_hit_self() {
        let mut snake = Snake {
            direction: Direction::Right,
            next_direction: Direction::Up,
            body: VecDeque::from([
                GridPosition::new(5, 5),
                GridPosition::new(5, 4),
                GridPosition::new(5, 6),
            ]),
        };
        let food = Food {
            position: GridPosition::new(10, 10),
        };
        let mut score = Score::default();
        let result = step(&mut snake, &food, &mut score);
        assert_eq!(
            result,
            StepResult {
                hit_wall: false,
                hit_self: true,
                ate_food: false
            }
        );
        assert_eq!(score.value, 0);
    }

    /// 不增长时移动到原尾部位置不算撞自己
    #[test]
    fn test_step_into_tail_not_collision() {
        let mut snake = Snake {
            direction: Direction::Right,
            next_direction: Direction::Up,
            body: VecDeque::from([
                GridPosition::new(5, 5),
                GridPosition::new(5, 6),
                GridPosition::new(6, 6),
            ]),
        };
        let food = Food {
            position: GridPosition::new(10, 10),
        };
        let mut score = Score::default();
        let result = step(&mut snake, &food, &mut score);
        assert!(!result.hit_wall);
        assert!(!result.hit_self);
        assert!(!result.ate_food);
    }
}
