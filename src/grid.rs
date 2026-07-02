use bevy::prelude::{Component, Vec2};

/// 棋盘列数
pub const GRID_WIDTH: i32 = 30;
/// 棋盘行数
pub const GRID_HEIGHT: i32 = 20;
/// 每格像素大小
pub const CELL_SIZE: f32 = 24.0;

/// 网格坐标，原点左上角，x 向右，y 向下
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct GridPosition {
    pub x: i32,
    pub y: i32,
}

impl GridPosition {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// 转换为世界坐标（格点中心）。
    ///
    /// 网格坐标原点为左上角、y 向下增长，而 Bevy 世界坐标 y 向上增长，
    /// 因此这里将 y 轴翻转，使视觉方向与网格方向一致。
    pub fn to_world(&self) -> Vec2 {
        Vec2::new(
            self.x as f32 * CELL_SIZE + CELL_SIZE / 2.0,
            (GRID_HEIGHT - 1 - self.y) as f32 * CELL_SIZE + CELL_SIZE / 2.0,
        )
    }
}

/// 移动方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    /// 是否为相反方向（用于禁止 180° 掉头）
    pub fn is_opposite(&self, other: &Direction) -> bool {
        matches!(
            (self, other),
            (Direction::Up, Direction::Down)
                | (Direction::Down, Direction::Up)
                | (Direction::Left, Direction::Right)
                | (Direction::Right, Direction::Left)
        )
    }

    /// 将方向应用到网格坐标
    pub fn apply(&self, pos: &GridPosition) -> GridPosition {
        match self {
            Direction::Up => GridPosition::new(pos.x, pos.y - 1),
            Direction::Down => GridPosition::new(pos.x, pos.y + 1),
            Direction::Left => GridPosition::new(pos.x - 1, pos.y),
            Direction::Right => GridPosition::new(pos.x + 1, pos.y),
        }
    }
}

/// 判断坐标是否越出棋盘边界
pub fn is_out_of_bounds(pos: &GridPosition) -> bool {
    pos.x < 0 || pos.x >= GRID_WIDTH || pos.y < 0 || pos.y >= GRID_HEIGHT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_world_origin() {
        let pos = GridPosition::new(0, 0);
        let world = pos.to_world();
        assert!((world.x - 12.0).abs() < f32::EPSILON);
        // 左上角对应世界坐标最上方
        assert!(
            (world.y - (GRID_HEIGHT as f32 * CELL_SIZE - CELL_SIZE / 2.0)).abs() < f32::EPSILON
        );
    }

    #[test]
    fn test_to_world_offset() {
        let pos = GridPosition::new(5, 3);
        let world = pos.to_world();
        assert!((world.x - 132.0).abs() < f32::EPSILON);
        // y 轴已翻转：(GRID_HEIGHT - 1 - 3) * CELL_SIZE + CELL_SIZE / 2
        assert!((world.y - 396.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_opposite_directions() {
        assert!(Direction::Up.is_opposite(&Direction::Down));
        assert!(Direction::Down.is_opposite(&Direction::Up));
        assert!(Direction::Left.is_opposite(&Direction::Right));
        assert!(Direction::Right.is_opposite(&Direction::Left));
    }

    #[test]
    fn test_non_opposite_directions() {
        assert!(!Direction::Up.is_opposite(&Direction::Left));
        assert!(!Direction::Up.is_opposite(&Direction::Right));
        assert!(!Direction::Down.is_opposite(&Direction::Left));
        assert!(!Direction::Right.is_opposite(&Direction::Down));
    }

    #[test]
    fn test_direction_apply() {
        let pos = GridPosition::new(5, 5);
        assert_eq!(Direction::Up.apply(&pos), GridPosition::new(5, 4));
        assert_eq!(Direction::Down.apply(&pos), GridPosition::new(5, 6));
        assert_eq!(Direction::Left.apply(&pos), GridPosition::new(4, 5));
        assert_eq!(Direction::Right.apply(&pos), GridPosition::new(6, 5));
    }

    /// 方向与世界坐标的视觉一致性：向上移动应使世界 y 增大（屏幕上方）。
    #[test]
    fn test_moving_up_increases_world_y() {
        let pos = GridPosition::new(5, 5);
        let up = Direction::Up.apply(&pos);
        assert!(up.to_world().y > pos.to_world().y);

        let down = Direction::Down.apply(&pos);
        assert!(down.to_world().y < pos.to_world().y);
    }

    #[test]
    fn test_out_of_bounds() {
        assert!(is_out_of_bounds(&GridPosition::new(-1, 5)));
        assert!(is_out_of_bounds(&GridPosition::new(30, 5)));
        assert!(is_out_of_bounds(&GridPosition::new(5, -1)));
        assert!(is_out_of_bounds(&GridPosition::new(5, 20)));
        assert!(!is_out_of_bounds(&GridPosition::new(0, 0)));
        assert!(!is_out_of_bounds(&GridPosition::new(29, 19)));
    }
}
