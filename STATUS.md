# Qwen3-ASR 集成 — 当前状态

## 项目背景

基于 Handy（Tauri 2.x 桌面语音输入应用）进行二次开发，保留其音频输入框架，将语音识别后端替换为 **Qwen3-ASR（Python sidecar + PyTorch CUDA）**，并修复 GNOME Wayland 下的快捷键触发和粘贴问题。

---

## 架构

```
录音(CPAL) → VAD(Silero) → 16kHz音频 → [Python sidecar] → Qwen3-ASR(CUDA) → 文本 → 粘贴
                Rust 主进程                   子进程(Python3.12)            Rust 主进程
```

### Qwen3-ASR 调用实现

**Python sidecar** (`python_sidecar/transcriber.py`):
- 从 ModelScope 缓存加载 `Qwen3-ASR-0.6B` 模型
- 使用 `torch.bfloat16` + `cuda:0` 推理，RTX 4090 ~869ms 完成转录
- 通过 stdin/stdout JSON 行协议与 Rust 主进程通信
- 启动时发送 `{"ready": true}`，之后持续监听 stdin
- 输入格式：`{"audio": "<base64_pcm_f32>", "sample_rate": 16000}`
- 输出格式：`{"text": "<transcription>", "language": "<lang>"}`
- 单例长驻进程，不重复加载模型

**Rust 侧车管理器** (`src-tauri/src/python_sidecar.rs`):
- `PythonSidecar` 结构体管理子进程生命周期
- `spawn()`: 启动 `venv/bin/python transcriber.py`，等待 ready 信号
- `transcribe()`: 发送 base64 音频，阻塞等待 JSON 返回
- 进程崩溃自动重启

**转录流程** (`src-tauri/src/managers/transcription.rs`):
- `LoadEngine::Qwen3Python { sidecar: PythonSidecar }`
- `transcribe()`: VAD 滤波后音频 → sidecar.transcribe() → 过滤自定义词 → clean text

### 自动粘贴实现

粘贴在 GNOME Wayland 下默认使用 **Ctrl+V 模式**，完整链路：

```
转录完成 → paste() 调用
  → paste_via_clipboard()
    → wl-copy 写入系统剪贴板（用户确认文字在剪贴板）
    → ydotoold 发 Ctrl+V 键组合（keycode 56 + V，适配 XKB 键盘映射）
    → 文字出现在当前焦点窗口
```

**键码适配**（`src-tauri/src/utils.rs`）：
- 检测 XKB 键盘布局选项（如 `ctrl:swap_lalt_lctl_lwin`）
- 根据实际映射返回正确的 Ctrl 键码：
  - 标准布局：keycode 29（KEY_LEFTCTRL）
  - `swap_lalt_lctl_lwin`：keycode 56（KEY_LEFTALT，映射为 Ctrl）
  - `swap_lwin_lctl`：keycode 125（KEY_LEFTMETA，映射为 Ctrl）
- 支持 `wtype`/`dotool`/`ydotool`/`xdotool`/enigo 多重回退

**GNOME 快捷键触发**（`~/.local/bin/handy-toggle`）：
- 包装脚本：source `~/.profile` → `pkill -SIGUSR2` 信号触发已有实例 → 回退启动新实例
- 绕过 `tauri_plugin_single_instance` 的 D-Bus IPC（在 GNOME 环境下不可靠）

---

## 已验证的功能

| 功能 | 状态 | 备注 |
|------|------|------|
| Rust 编译 | ✅ | `cargo build` 通过 |
| 前端显示 | ✅ | 显示 Qwen3-ASR 0.6B 模型 |
| Python sidecar 启动 | ✅ | 加载模型到 RTX 4090 |
| 中文语音转写 | ✅ | ~500-869ms，识别准确 |
| CLI 触发 (`--toggle-transcription`) | ✅ | 录音 + 转写正常 |
| GNOME 快捷键触发 | ✅ | 通过 wrapper 脚本 + SIGUSR2 信号 |
| 自动粘贴（Ctrl+V 模式） | ✅ | wl-copy 写剪贴板 → ydotool 模拟 Ctrl+V |
| 键盘映射适配 | ✅ | 支持 `swap_lalt_lctl_lwin`、`swap_lwin_lctl` |
| 工具链回退 | ✅ | wtype(跳过GNOME) → dotool → ydotool → enigo |
| 首次实例 CLI 处理 | ✅ | 启动时 `--toggle-transcription` 直接触发录音 |
| 剪贴板写入 | ✅ | wl-copy 写入成功（用户确认） |

---

## 存在的问题

### 1. 多次录音后采样为 0  ❌

**现象**：连续录音 1-2 次后，后续录音 `sample count: 0`，无法识别。

**分析**：这是 Handy 原项目的 bug，与我们的改动无关。
- `stop_recording()` 调用 `rec.close()` 销毁 CPAL 音频流
- 下次 `start_recording()` 重新创建流
- 在 PipeWire/ALSA 上，流关闭再打开 2-3 次后，新流不再产生音频数据
- CPAL 的 `stop_flag` pause/resume 机制也可能是原因之一

**尝试过的修复（均已回滚）**：
- 设置 `lazy_stream_close = true` 保留流不关闭 → 第二次录音仍然 0 samples
- 去掉 drain loop 和 stop_flag → 不解决问题
- PW pipewire 重启可临时恢复

**当前方案**：音频代码完全保持原版，录音遇到问题重启 PipeWire。

### 2. wl-copy 剪贴板协议警告

**现象**：日志中每次启动出现 `arboard::platform::linux` 警告：
```
Tried to initialize the wayland data control protocol clipboard, but failed.
```

**原因**：GNOME 不支持 `ext-data-control` / `wlr-data-control` 协议，`arboard` 回退到 X11 协议。不影响实际使用，因为粘贴流中使用 `wl-copy` 写剪贴板（外部二进制，不依赖 data-control 协议）。

### 3. Layer Shell 协议不支持

**现象**：日志中每次启动出现：
```
It appears your Wayland compositor does not support the Layer Shell protocol
```

**原因**：GNOME Mutter 不支持 `wlr-layer-shell` 协议。悬浮窗（recording overlay）无法正常显示，不影响核心功能。

### 4. 单次粘贴后可能需手动调整

**现象**：自动粘贴可能贴到 Handy 自身窗口而非目标应用。

**原因**：悬浮窗（overlay）的 show/hide 操作可能导致 GNOME 将焦点还给 Handy 主窗口。关闭悬浮窗后可避免此问题。

---

## 系统依赖

| 组件 | 说明 |
|------|------|
| Python 3.12 | 通过 uv 管理，venv 在 `python_sidecar/venv/` |
| PyTorch CUDA | `torch torchvision torchaudio` + CUDA 12.4 |
| `qwen-asr` | PyPI 包，自动下载模型 |
| ModelScope | 模型缓存 `~/.cache/modelscope/hub/models/Qwen/Qwen3-ASR-0___6B` |
| `wl-clipboard` | 提供 `wl-copy`/`wl-paste`，Wayland 剪贴板操作 |
| `ydotoold` | uinput 守护进程，systemd user service |
| `uinput` | 内核模块，开机自动加载（`/etc/modules-load.d/uinput.conf`）|

---

## 修改的文件

### 新增
| 文件 | 说明 |
|------|------|
| `python_sidecar/transcriber.py` | Python 推理侧车脚本 |
| `python_sidecar/setup.sh` | Python 环境安装脚本 |
| `src-tauri/src/python_sidecar.rs` | Rust 侧车进程管理器 |
| `~/.local/bin/handy-toggle` | GNOME 快捷键包装脚本 |
| `~/.config/systemd/user/ydotoold.service` | ydotool 守护进程 |

### 修改
| 文件 | 说明 |
|------|------|
| `src-tauri/src/lib.rs` | 首次实例 CLI 处理、注册 python_sidecar 模块 |
| `src-tauri/src/settings.rs` | GNOME Wayland 下 paste_method 自动切换为 CtrlV |
| `src-tauri/src/clipboard.rs` | GNOME 跳过 wtype、工具链非致命失败回退、动态 Ctrl 键码 |
| `src-tauri/src/utils.rs` | `is_gnome()`、XKB 键盘映射检测、Ctrl keycode 缓存 |
| `src-tauri/src/managers/transcription.rs` | 替换 sherpa-onnx 为 PythonSidecar |
| `src-tauri/src/managers/model.rs` | 简化模型管理 |
| `src-tauri/Cargo.toml` | 移除 sherpa-onnx/bzip2，添加 base64/bytemuck |
| `src-tauri/src/commands/models.rs` | 移除旧模型加载参数 |
| `src/components/settings/AccelerationSelector.tsx` | 简化加速器 UI |
| `src/i18n/locales/en/translation.json` | 模型翻译键 |
| `src/i18n/locales/zh/translation.json` | 模型翻译键 |

### 移除的依赖
| 依赖 | 原因 |
|------|------|
| `transcribe-rs` | 替换为 sherpa-onnx → Python sidecar |
| `sherpa-onnx` | CPU-only 预编译库，不支持 4090 CUDA |
| `bzip2` | 不需要解压模型归档 |

---

## 运行方式

```bash
# 开发模式（推荐）
cd /home/darkness/TOOLS/Handy-main
bun run tauri dev

# 手动触发转录（调试用）
./src-tauri/target/debug/handy --toggle-transcription

# 只编译
cd src-tauri && cargo build

# 重置音频设备（录音异常时）
systemctl --user restart pipewire
```

## 正确测试流程

### 首次测试或重启后的完整测试

```bash
# 1. 确保 PipeWire 音频设备正常
systemctl --user restart pipewire

# 2. 开发模式启动（自动启动 Vite 前端 + Rust 后端）
cd ~/TOOLS/Handy-main
bun run tauri dev

# 3. Handy 窗口显示后，在设置中关闭悬浮窗（overlay）
#    避免粘贴时焦点被 Handy 窗口抢走

# 4. 打开文本编辑器并聚焦

# 5. 按 GNOME 快捷键触发录音（说话）→ 再按停止
```

### 注意事项

1. **不要频繁强制杀掉 Handy 进程** — `pkill -9`、`kill`、`fuser -k` 会导致 PipeWire 音频设备状态异常。
   - 如果需要重启，让 `bun run tauri dev` 自动管理进程
   - 或者用 `kill $(pgrep -x handy)` 正常关闭

2. **连续录音异常时** — 如果第 2-3 次录音开始出现 `sample count: 0`，重启 PipeWire：
   ```bash
   systemctl --user restart pipewire
   ```
   然后重新启动 Handy（不要杀掉旧进程再开，而是完整重来）。

3. **悬浮窗（overlay）务必关闭** — 否则粘贴时 overlay 的 show/hide 操作会导致 GNOME 把焦点还给 Handy 窗口，ydotool 的 Ctrl+V 会发到 Handy 自身而非目标应用。

4. **GNOME 快捷键配置** — 设置 → 键盘 → 自定义快捷键 → 添加：
   - 命令：`/home/darkness/.local/bin/handy-toggle`
   - 绑定按键后即可使用

5. **键盘映射适配** — 如果你的 XKB 有 modifier swap（如 `swap_lalt_lctl_lwin`），系统会自动检测并适配 Ctrl 键码。日志可见：`Detected Ctrl keycode: X`。

## Python 环境

- 路径: `python_sidecar/venv/`
- Python 3.12（通过 uv 管理）
- 关键包: `torch`(CUDA), `qwen-asr`, `modelscope`
- 模型缓存: `~/.cache/modelscope/hub/models/Qwen/Qwen3-ASR-0___6B`
- 重装: `bash python_sidecar/setup.sh`
