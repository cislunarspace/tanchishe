use std::path::Path;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// 最高分持久化文件的默认文件名。
pub const HIGH_SCORE_FILE: &str = "highscore.json";

/// 历史最高分。
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct HighScore {
    pub value: u32,
}

impl HighScore {
    /// 从指定路径加载最高分。读取或解析失败时返回默认值并记录日志。
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(contents) => match serde_json::from_str::<HighScore>(&contents) {
                Ok(score) => score,
                Err(e) => {
                    eprintln!("解析最高分文件失败: {e}");
                    Self::default()
                }
            },
            Err(e) => {
                eprintln!("读取最高分文件失败: {e}");
                Self::default()
            }
        }
    }

    /// 将最高分保存到指定路径。失败时记录日志，不抛出错误。
    pub fn save(&self, path: &Path) {
        match serde_json::to_string_pretty(self) {
            Ok(contents) => {
                if let Err(e) = std::fs::write(path, contents) {
                    eprintln!("保存最高分文件失败: {e}");
                }
            }
            Err(e) => {
                eprintln!("序列化最高分失败: {e}");
            }
        }
    }

    /// 用本局得分更新最高分。返回是否刷新了记录。
    pub fn update(&mut self, score: u32) -> bool {
        if score > self.value {
            self.value = score;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    /// 辅助：创建临时目录与文件路径
    fn temp_path() -> (std::path::PathBuf, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        (dir.path().join(HIGH_SCORE_FILE), dir)
    }

    /// 默认最高分值为 0
    #[test]
    fn test_high_score_default_is_zero() {
        let score = HighScore::default();
        assert_eq!(score.value, 0);
    }

    /// update 只在得分更高时刷新记录
    #[test]
    fn test_update_only_when_higher() {
        let mut high = HighScore { value: 50 };
        assert!(!high.update(30));
        assert_eq!(high.value, 50);
        assert!(high.update(70));
        assert_eq!(high.value, 70);
        assert!(!high.update(70));
    }

    /// 保存后能够正确加载
    #[test]
    fn test_save_and_load_roundtrip() {
        let (path, _dir) = temp_path();
        let score = HighScore { value: 120 };
        score.save(&path);
        let loaded = HighScore::load(&path);
        assert_eq!(loaded.value, 120);
    }

    /// 加载不存在文件时返回默认值
    #[test]
    fn test_load_missing_file_returns_default() {
        let (path, _dir) = temp_path();
        let loaded = HighScore::load(&path);
        assert_eq!(loaded.value, 0);
    }

    /// 加载非法 JSON 时返回默认值，不崩溃
    #[test]
    fn test_load_corrupted_file_returns_default() {
        let (path, _dir) = temp_path();
        let mut file = std::fs::File::create(&path).unwrap();
        write!(file, "not json").unwrap();
        drop(file);

        let loaded = HighScore::load(&path);
        assert_eq!(loaded.value, 0);
    }

    /// 保存到无效路径时记录日志但不 panic
    #[test]
    fn test_save_to_invalid_path_does_not_panic() {
        let score = HighScore { value: 10 };
        let invalid = std::path::Path::new("/dev/null/invalid/highscore.json");
        score.save(invalid);
    }
}
