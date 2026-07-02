use std::collections::VecDeque;

use bevy::prelude::Resource;

use crate::grid::{self, Direction, GridPosition};

/// 蛇资源，持有方向和各段网格坐标
#[derive(Resource)]
pub struct Snake {
    pub direction: Direction,
    pub next_direction: Direction,
    pub body: VecDeque<GridPosition>,
}

/// 初始化一条长度为 3 的蛇，位于棋盘中部，方向向右
pub fn spawn_snake() -> Snake {
    let head = GridPosition::new(GRID_WIDTH / 2, GRID_HEIGHT / 2);
    let body = VecDeque::from([
        head,
        GridPosition::new(head.x - 1, head.y),
        GridPosition::new(head.x - 2, head.y),
    ]);
    Snake {
        direction: Direction::Right,
        next_direction: Direction::Right,
        body,
    }
}

const GRID_WIDTH: i32 = grid::GRID_WIDTH;
const GRID_HEIGHT: i32 = grid::GRID_HEIGHT;

/// 改变方向；如果输入方向与当前方向相反则忽略
pub fn change_direction(snake: &mut Snake, new_dir: Direction) {
    if !new_dir.is_opposite(&snake.direction) {
        snake.next_direction = new_dir;
    }
}

/// 执行一步移动：先应用 pending 方向，再计算新蛇头位置。
/// 如果新蛇头越界，返回 true 表示撞墙。
pub fn advance_snake(snake: &mut Snake) -> bool {
    advance_snake_with_growth(snake, false)
}

/// 执行一步移动，可选择是否增长（吃到食物时不移除尾部）。
/// 如果新蛇头越界，返回 true 表示撞墙。
pub fn advance_snake_with_growth(snake: &mut Snake, grow: bool) -> bool {
    snake.direction = snake.next_direction;

    let new_head = snake.direction.apply(snake.body.front().unwrap());

    if grid::is_out_of_bounds(&new_head) {
        return true;
    }

    snake.body.push_front(new_head);
    if !grow {
        snake.body.pop_back();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试初始化蛇的属性
    #[test]
    fn test_spawn_snake() {
        let snake = spawn_snake();
        assert_eq!(snake.body.len(), 3);
        assert_eq!(snake.direction, Direction::Right);
        let head = *snake.body.front().unwrap();
        assert_eq!(head.x, GRID_WIDTH / 2);
        assert_eq!(head.y, GRID_HEIGHT / 2);
    }

    /// 正常移动：蛇头向右前进一格，尾部收缩
    #[test]
    fn test_advance_normal() {
        let mut snake = spawn_snake();
        let old_head = *snake.body.front().unwrap();
        let hit_wall = advance_snake(&mut snake);
        assert!(!hit_wall);
        assert_eq!(snake.body.front().unwrap().x, old_head.x + 1);
        assert_eq!(snake.body.front().unwrap().y, old_head.y);
        assert_eq!(snake.body.len(), 3);
    }

    /// 越界检测：蛇头在左边界向左移动应撞墙
    #[test]
    fn test_advance_wall_left() {
        let mut snake = spawn_snake();
        snake.body = VecDeque::from([GridPosition::new(0, 5)]);
        snake.direction = Direction::Left;
        snake.next_direction = Direction::Left;
        assert!(advance_snake(&mut snake));
    }

    /// 越界检测：蛇头在右边界向右移动应撞墙
    #[test]
    fn test_advance_wall_right() {
        let mut snake = spawn_snake();
        snake.body = VecDeque::from([GridPosition::new(GRID_WIDTH - 1, 5)]);
        snake.direction = Direction::Right;
        snake.next_direction = Direction::Right;
        assert!(advance_snake(&mut snake));
    }

    /// 越界检测：蛇头在上边界向上移动应撞墙
    #[test]
    fn test_advance_wall_up() {
        let mut snake = spawn_snake();
        snake.body = VecDeque::from([GridPosition::new(5, 0)]);
        snake.direction = Direction::Up;
        snake.next_direction = Direction::Up;
        assert!(advance_snake(&mut snake));
    }

    /// 越界检测：蛇头在下边界向下移动应撞墙
    #[test]
    fn test_advance_wall_down() {
        let mut snake = spawn_snake();
        snake.body = VecDeque::from([GridPosition::new(5, GRID_HEIGHT - 1)]);
        snake.direction = Direction::Down;
        snake.next_direction = Direction::Down;
        assert!(advance_snake(&mut snake));
    }

    /// 普通移动不应撞墙
    #[test]
    fn test_no_wall_hit() {
        let mut snake = spawn_snake();
        let hit = advance_snake(&mut snake);
        assert!(!hit);
    }

    /// 反向输入过滤：当前向右，输入左应被忽略
    #[test]
    fn test_reverse_input_rejected() {
        let mut snake = spawn_snake();
        change_direction(&mut snake, Direction::Left);
        assert_eq!(snake.next_direction, Direction::Right);
    }
}
