use bevy::prelude::*;

/// 游戏整体所处的高阶阶段。
#[derive(States, Clone, Copy, Default, Eq, PartialEq, Hash, Debug)]
pub enum AppState {
    /// 主菜单，可选择难度并开始游戏
    #[default]
    Menu,
    /// 游戏进行中
    Playing,
    /// 暂停
    Paused,
    /// 死亡动画播放中
    Dying,
    /// 游戏结束（撞墙或撞自己）
    GameOver,
    /// 胜利（占满棋盘）
    Victory,
}
