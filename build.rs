use std::fs;
use std::path::Path;

/// 采样率（Hz）
const SAMPLE_RATE: u32 = 22050;

/// 将 PCM 样本写入单声道 16-bit WAV 文件。
fn write_wav(path: &Path, samples: &[i16]) {
    let data_len = samples.len() * 2;
    let file_len = 36 + data_len;

    let mut bytes = Vec::with_capacity(44 + data_len);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(file_len as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes()); // 子块大小
    bytes.extend_from_slice(&1u16.to_le_bytes()); // 音频格式：PCM
    bytes.extend_from_slice(&1u16.to_le_bytes()); // 声道数：单声道
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // 字节率
    bytes.extend_from_slice(&2u16.to_le_bytes()); // 块对齐
    bytes.extend_from_slice(&16u16.to_le_bytes()); // 采样位数
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(data_len as u32).to_le_bytes());
    for s in samples {
        bytes.extend_from_slice(&s.to_le_bytes());
    }

    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

/// 生成从起始频率扫到结束频率、带包络的正弦波样本。
fn sweep(start_freq: f32, end_freq: f32, duration_sec: f32) -> Vec<i16> {
    let num_samples = (SAMPLE_RATE as f32 * duration_sec) as usize;
    let mut samples = Vec::with_capacity(num_samples);
    for i in 0..num_samples {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = i as f32 / num_samples as f32;
        let freq = start_freq + (end_freq - start_freq) * phase;
        let sample = (t * freq * 2.0 * std::f32::consts::PI).sin();
        let env = 1.0 - phase;
        samples.push((sample * env * 3000.0) as i16);
    }
    samples
}

fn main() {
    let out_dir = Path::new("assets/sfx");
    fs::create_dir_all(out_dir).unwrap();

    // 吃到食物的提示音：短促上升音，约 0.1 秒。
    write_wav(&out_dir.join("eat.wav"), &sweep(800.0, 1200.0, 0.1));
    // 死亡音效：下降音，约 0.3 秒。
    write_wav(&out_dir.join("die.wav"), &sweep(400.0, 100.0, 0.3));

    println!("cargo:rerun-if-changed=build.rs");
}
