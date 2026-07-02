use bevy::prelude::*;
use bevy_kira_audio::prelude::*;

/// 游戏音频插件。
///
/// `enabled` 控制是否真正初始化 Kira 音频后端与加载资源。
/// 集成测试可传入 `false`，仅注册事件与静音状态，避免依赖音频后端。
pub struct GameAudioPlugin {
    pub enabled: bool,
}

impl GameAudioPlugin {
    /// 生产环境使用：启用真实音频。
    pub fn enabled() -> Self {
        Self { enabled: true }
    }

    /// 测试环境使用：仅注册事件与静音资源。
    pub fn disabled() -> Self {
        Self { enabled: false }
    }
}

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioMuted>()
            .add_event::<FoodEaten>()
            .add_event::<DeathEvent>();

        if self.enabled {
            app.add_plugins(AudioPlugin)
                .init_resource::<AudioAssets>()
                .add_systems(Startup, load_audio)
                .add_systems(PostUpdate, (play_eat_sound, play_death_sound))
                .add_systems(Update, toggle_mute);
        } else {
            app.add_systems(Update, toggle_mute);
        }
    }
}

/// 蛇吃到食物时触发的事件。
#[derive(Event, Debug, Clone, Copy)]
pub struct FoodEaten;

/// 蛇死亡时触发的事件。
#[derive(Event, Debug, Clone, Copy)]
pub struct DeathEvent;

/// 全局静音开关。
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct AudioMuted {
    pub muted: bool,
}

/// 音效资源句柄。
#[derive(Resource, Default)]
pub struct AudioAssets {
    pub eat: Handle<AudioSource>,
    pub die: Handle<AudioSource>,
}

const EAT_SOUND_PATH: &str = "sfx/eat.wav";
const DIE_SOUND_PATH: &str = "sfx/die.wav";

/// 加载程序化生成的音效资源。
fn load_audio(asset_server: Res<AssetServer>, mut assets: ResMut<AudioAssets>) {
    assets.eat = asset_server.load(EAT_SOUND_PATH);
    assets.die = asset_server.load(DIE_SOUND_PATH);
}

/// 播放吃食物音效。
fn play_eat_sound(
    mut events: EventReader<FoodEaten>,
    audio: Res<Audio>,
    assets: Res<AudioAssets>,
    muted: Res<AudioMuted>,
) {
    if muted.muted {
        return;
    }
    for _ in events.read() {
        audio.play(assets.eat.clone());
    }
}

/// 播放死亡音效。
fn play_death_sound(
    mut events: EventReader<DeathEvent>,
    audio: Res<Audio>,
    assets: Res<AudioAssets>,
    muted: Res<AudioMuted>,
) {
    if muted.muted {
        return;
    }
    for _ in events.read() {
        audio.play(assets.die.clone());
    }
}

/// 全局静音切换：Ctrl+M 或 Ctrl+S。
fn toggle_mute(
    keys: Res<ButtonInput<KeyCode>>,
    mut muted: ResMut<AudioMuted>,
    mut query: Query<&mut Text, With<MuteButtonText>>,
) {
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let pressed = ctrl && (keys.just_pressed(KeyCode::KeyM) || keys.just_pressed(KeyCode::KeyS));
    if !pressed {
        return;
    }
    muted.muted = !muted.muted;
    for mut text in &mut query {
        text.0 = mute_button_text(muted.muted);
    }
}

/// 菜单中静音按钮的文字子节点标记。
#[derive(Component)]
pub struct MuteButtonText;

/// 根据静音状态生成按钮文字。
pub fn mute_button_text(muted: bool) -> String {
    if muted {
        "静音：开".to_string()
    } else {
        "静音：关".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 初始未静音。
    #[test]
    fn test_audio_muted_default() {
        let muted = AudioMuted::default();
        assert!(!muted.muted);
    }

    /// mute_button_text 随状态变化。
    #[test]
    fn test_mute_button_text() {
        assert_eq!(mute_button_text(false), "静音：关");
        assert_eq!(mute_button_text(true), "静音：开");
    }
}
