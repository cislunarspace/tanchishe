use std::collections::VecDeque;

use bevy::prelude::{ButtonInput, KeyCode, Resource};

use crate::grid::Direction;
use crate::snake::{self, Snake};

/// 方向输入队列。
///
/// 每帧将所有 newly pressed 的方向键依次入队；在步进前由 `apply_input_queue`
/// 只取出队列中第一个不与当前方向相反的方向应用到蛇上。
#[derive(Resource, Default)]
pub struct InputQueue {
    pub directions: VecDeque<Direction>,
}

/// 从按键映射到方向；无匹配则返回 None
pub fn key_to_direction(key: KeyCode) -> Option<Direction> {
    match key {
        KeyCode::ArrowUp | KeyCode::KeyW => Some(Direction::Up),
        KeyCode::ArrowDown | KeyCode::KeyS => Some(Direction::Down),
        KeyCode::ArrowLeft | KeyCode::KeyA => Some(Direction::Left),
        KeyCode::ArrowRight | KeyCode::KeyD => Some(Direction::Right),
        _ => None,
    }
}

/// 读取本帧新按下的方向键并入队。
pub fn handle_direction_input(keys: &ButtonInput<KeyCode>, queue: &mut InputQueue) {
    for key in keys.get_just_pressed() {
        if let Some(dir) = key_to_direction(*key) {
            queue.directions.push_back(dir);
        }
    }
}

/// 从队列中取出一个不与当前方向相反的方向应用到蛇。
///
/// 每帧最多应用一个有效方向；若取出的第一个方向与当前方向相反，则丢弃并继续
/// 检查下一个，直到找到有效方向或队列为空。
pub fn apply_input_queue(queue: &mut InputQueue, snake: &mut Snake) {
    while let Some(dir) = queue.directions.pop_front() {
        if !dir.is_opposite(&snake.direction) {
            snake::change_direction(snake, dir);
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_to_direction_arrows() {
        assert_eq!(key_to_direction(KeyCode::ArrowUp), Some(Direction::Up));
        assert_eq!(key_to_direction(KeyCode::ArrowDown), Some(Direction::Down));
        assert_eq!(key_to_direction(KeyCode::ArrowLeft), Some(Direction::Left));
        assert_eq!(
            key_to_direction(KeyCode::ArrowRight),
            Some(Direction::Right)
        );
    }

    #[test]
    fn test_key_to_direction_wasd() {
        assert_eq!(key_to_direction(KeyCode::KeyW), Some(Direction::Up));
        assert_eq!(key_to_direction(KeyCode::KeyS), Some(Direction::Down));
        assert_eq!(key_to_direction(KeyCode::KeyA), Some(Direction::Left));
        assert_eq!(key_to_direction(KeyCode::KeyD), Some(Direction::Right));
    }

    #[test]
    fn test_key_to_direction_none() {
        assert_eq!(key_to_direction(KeyCode::Space), None);
        assert_eq!(key_to_direction(KeyCode::Enter), None);
        assert_eq!(key_to_direction(KeyCode::KeyR), None);
    }

    #[test]
    fn test_opposite_rejected() {
        let mut snake = snake::spawn_snake(); // 向右
        snake::change_direction(&mut snake, Direction::Left);
        assert_eq!(snake.next_direction, Direction::Right);
    }

    #[test]
    fn test_non_opposite_accepted() {
        let mut snake = snake::spawn_snake(); // 向右
        snake::change_direction(&mut snake, Direction::Up);
        assert_eq!(snake.next_direction, Direction::Up);
    }

    #[test]
    fn test_apply_input_queue_takes_first_valid() {
        let mut snake = snake::spawn_snake(); // 向右
        let mut queue = InputQueue::default();
        queue.directions.push_back(Direction::Left); // 与当前相反，应被丢弃
        queue.directions.push_back(Direction::Up); // 有效
        queue.directions.push_back(Direction::Down); // 不应被处理

        apply_input_queue(&mut queue, &mut snake);

        assert_eq!(snake.next_direction, Direction::Up);
        assert_eq!(queue.directions.len(), 1);
        assert_eq!(queue.directions.front(), Some(&Direction::Down));
    }

    #[test]
    fn test_apply_input_queue_empty_does_nothing() {
        let mut snake = snake::spawn_snake();
        let mut queue = InputQueue::default();
        apply_input_queue(&mut queue, &mut snake);
        assert_eq!(snake.next_direction, snake.direction);
    }

    #[test]
    fn test_apply_input_queue_all_opposite_gets_discarded() {
        let mut snake = snake::spawn_snake(); // 向右
        let mut queue = InputQueue::default();
        queue.directions.push_back(Direction::Left);
        queue.directions.push_back(Direction::Left);

        apply_input_queue(&mut queue, &mut snake);

        assert_eq!(snake.next_direction, Direction::Right);
        assert!(queue.directions.is_empty());
    }
}
