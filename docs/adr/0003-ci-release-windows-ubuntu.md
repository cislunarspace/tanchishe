# CI 与发布仅覆盖 Windows 和 Ubuntu

---
status: accepted
---

项目 CI 和自动发布只针对 **Windows** 和 **Ubuntu** 两个平台。GitHub Actions 在 push `v*.*.*` 标签时自动构建 release 并上传两个平台的压缩包；同时保留 `workflow_dispatch` 手动触发入口用于测试构建。

最初考虑过加入 macOS 和 Web/WASM，但为了控制 v0.1.0 的复杂度，决定先聚焦 Windows 和 Ubuntu。这两个平台覆盖了主要用户群，且 Bevy 在它们上的构建流程最稳定。macOS 和 Web 版放到后续版本评估，不阻塞初版发布。
