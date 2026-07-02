use crate::grid::{Direction, GridPosition, GRID_HEIGHT, GRID_WIDTH};

/// 根据当前蛇头位置返回哈密顿回路中的下一步方向。
///
/// 回路覆盖整个 `GRID_WIDTH × GRID_HEIGHT` 棋盘：
/// - 第 0 行从左向右；
/// - 中间奇数行从右向左、偶数行从左向右，形成蛇形；
/// - 右边缘整体向下遍历；
/// - 左边缘从底部返回顶部，最终闭合回起点 `(0, 0)`。
///
/// 在初始蛇头 `(GRID_WIDTH / 2, GRID_HEIGHT / 2)` 处返回 `Direction::Right`，
/// 与蛇的初始方向兼容，不会被 `change_direction` 的 180° 掉头过滤。
pub fn next_direction(head: GridPosition) -> Direction {
    let x = head.x;
    let y = head.y;

    // 左边缘：从底部返回起点，但起点 (0, 0) 向右出发。
    if x == 0 {
        if y == 0 {
            return Direction::Right;
        }
        return Direction::Up;
    }

    // 第 0 行：从左往右走到右边缘后向下。
    if y == 0 {
        if x == GRID_WIDTH - 1 {
            return Direction::Down;
        }
        return Direction::Right;
    }

    // 奇数行：从右往左遍历，到 x == 1 时转向下方；
    // 若已是最底行，则拐进左边缘。
    if y % 2 == 1 {
        if x == 1 {
            if y == GRID_HEIGHT - 1 {
                return Direction::Left;
            }
            return Direction::Down;
        }
        return Direction::Left;
    }

    // 偶数行（y >= 2）：从 x == 1 往右走到右边缘，再向下。
    if x == GRID_WIDTH - 1 {
        Direction::Down
    } else {
        Direction::Right
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// 验证哈密顿回路访问所有格点一次且最终回到起点。
    #[test]
    fn test_hamiltonian_cycle() {
        let start = GridPosition::new(0, 0);
        let mut pos = start;
        let mut visited = HashSet::new();
        visited.insert(pos);

        for step in 1..=600 {
            let dir = next_direction(pos);
            let next = dir.apply(&pos);

            assert!(
                !crate::grid::is_out_of_bounds(&next),
                "第 {} 步越界: 从 {:?} 向 {:?} 移动到 {:?}",
                step,
                pos,
                dir,
                next
            );

            let dx = (next.x - pos.x).abs();
            let dy = (next.y - pos.y).abs();
            assert_eq!(
                dx + dy,
                1,
                "第 {} 步非相邻移动: 从 {:?} 到 {:?}",
                step,
                pos,
                next
            );

            pos = next;
            if step < 600 {
                assert!(visited.insert(pos), "第 {} 步重复访问 {:?}", step, pos);
            }
        }

        assert_eq!(pos, start, "600 步后未回到起点");
        assert_eq!(visited.len(), 600, "应恰好访问 600 个不同格点");
    }

    /// 验证初始蛇头位置返回的方向与初始方向 Right 兼容。
    #[test]
    fn test_initial_head_compatible_with_right() {
        let head = GridPosition::new(GRID_WIDTH / 2, GRID_HEIGHT / 2);
        let dir = next_direction(head);
        assert!(
            !dir.is_opposite(&Direction::Right),
            "初始位置 AI 方向 {:?} 不能与 Right 相反",
            dir
        );
    }
}
