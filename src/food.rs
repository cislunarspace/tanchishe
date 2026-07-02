use std::collections::HashSet;

use bevy::prelude::Resource;
use rand::Rng;

use crate::grid::{GridPosition, GRID_HEIGHT, GRID_WIDTH};
use crate::snake::Snake;

/// 食物资源，记录当前食物所在的网格坐标
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Food {
    pub position: GridPosition,
}

/// 在棋盘空格中随机生成一个食物。
/// 如果棋盘已被蛇占满（胜利条件），返回 None。
pub fn spawn_food(snake: &Snake, rng: &mut impl Rng) -> Option<Food> {
    let occupied: HashSet<GridPosition> = snake.body.iter().copied().collect();
    let total_cells = (GRID_WIDTH * GRID_HEIGHT) as usize;
    if occupied.len() >= total_cells {
        return None;
    }

    let mut empty = Vec::with_capacity(total_cells - occupied.len());
    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            let pos = GridPosition::new(x, y);
            if !occupied.contains(&pos) {
                empty.push(pos);
            }
        }
    }

    let idx = rng.gen_range(0..empty.len());
    Some(Food {
        position: empty[idx],
    })
}

#[cfg(test)]
mod tests {
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use super::*;

    /// 食物不会生成在蛇身体上
    #[test]
    fn test_food_not_on_snake() {
        let snake = Snake {
            direction: crate::grid::Direction::Right,
            next_direction: crate::grid::Direction::Right,
            body: [
                GridPosition::new(5, 5),
                GridPosition::new(4, 5),
                GridPosition::new(3, 5),
            ]
            .into_iter()
            .collect(),
        };
        let mut rng = StdRng::seed_from_u64(42);
        let food = spawn_food(&snake, &mut rng).unwrap();
        assert!(!snake.body.contains(&food.position));
    }

    /// 棋盘满时无法生成食物，返回 None
    #[test]
    fn test_food_none_when_board_full() {
        let mut body = std::collections::VecDeque::new();
        for y in 0..GRID_HEIGHT {
            for x in 0..GRID_WIDTH {
                body.push_back(GridPosition::new(x, y));
            }
        }
        let snake = Snake {
            direction: crate::grid::Direction::Right,
            next_direction: crate::grid::Direction::Right,
            body,
        };
        let mut rng = StdRng::seed_from_u64(42);
        assert!(spawn_food(&snake, &mut rng).is_none());
    }

    /// 只剩一个空格时，食物必须生成在该格
    #[test]
    fn test_food_only_empty_cell() {
        let mut body = std::collections::VecDeque::new();
        for y in 0..GRID_HEIGHT {
            for x in 0..GRID_WIDTH {
                body.push_back(GridPosition::new(x, y));
            }
        }
        body.pop_back();
        let snake = Snake {
            direction: crate::grid::Direction::Right,
            next_direction: crate::grid::Direction::Right,
            body,
        };
        let mut rng = StdRng::seed_from_u64(42);
        let food = spawn_food(&snake, &mut rng).unwrap();
        assert_eq!(
            food.position,
            GridPosition::new(GRID_WIDTH - 1, GRID_HEIGHT - 1)
        );
    }
}
