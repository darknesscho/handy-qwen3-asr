# Handy-Qwen3-ASR

基于 [Handy](https://github.com/cjpais/Handy) 的桌面语音输入工具，将 Whisper 后端替换为 **Qwen3-ASR 0.6B**（PyTorch CUDA），并在 GNOME Wayland 下做了大量兼容性修复。

**模型仅支持 `Qwen3-ASR-0.6B`。仅在 Arch Linux + Wayland + GNOME 下实机测试通过。**

> 原始 Handy 项目由 [cjpais](https://github.com/cjpais/Handy) 开发，MIT 协议。本项目在此基础上进行了后端替换和 Linux 兼容性修复。

## 特性

- 基于 Tauri 2.x 桌面框架，Rust 后端 + React 前端
- **Qwen3-ASR 0.6B** 模型，PyTorch CUDA 推理（Python sidecar）
- Silero VAD 语音活动检测（自动过滤静音和噪声）
- GNOME Wayland 下自动粘贴（wl-copy + ydotool）
- 全局快捷键触发录音，可选按键说话模式
- 支持自定义热词替换和 LLM 后处理

## 架构

```
录音(CPAL) → VAD(Silero) → 16kHz 音频 → Python sidecar → Qwen3-ASR(CUDA) → 文本 → 粘贴
 ──────── Rust 主进程 ────────    ── 子进程(Python 3.12) ──    ── Rust 主进程 ──
```

## 安装

### 1. 克隆仓库

```bash
git clone https://github.com/YOUR_USERNAME/handy-qwen3-asr.git
cd handy-qwen3-asr
```

### 2. 安装系统依赖

参见 [DEPENDENCIES.md](DEPENDENCIES.md)。Arch Linux 快速安装：

```bash
pacman -S bun python rustup wl-clipboard ydotool

# uinput（粘贴模拟按键所需，需重启生效）
echo "uinput" | sudo tee /etc/modules-load.d/uinput.conf
sudo modprobe uinput

# ydotoold 守护进程（开机自启）
systemctl --user enable --now ydotoold
```

### 3. 配置 Python 环境

```bash
bash python_sidecar/setup.sh
```

### 4. 下载模型

```bash
uv run --python python_sidecar/venv/bin/python \
  modelscope download --model Qwen/Qwen3-ASR-0.6B
```

### 5. 编译运行

```bash
bun install
bun run tauri dev      # 开发模式
bun run tauri build     # 生产编译
```

## 使用

### GNOME 快捷键

在 GNOME 设置 → 键盘 → 自定义快捷键中添加：

```
名称：Handy 语音输入
命令：<项目目录>/handy-toggle
```

按一次开始录音（显示浮动窗口），再按一次停止并自动粘贴转写文字。

### 关闭浮动窗口

GNOME Wayland 下浮动窗口可能抢焦点，导致 Ctrl+V 贴到 Handy 自身窗口。在设置中关闭「录音浮窗」即可正常粘贴。

### 麦克风增益

如果录音一直为空，检查 PipeWire 增益——过高会削波导致 VAD 无法识别：

```bash
wpctl status | grep -A5 "Sources"    # 查看输入源 ID
wpctl set-volume <ID> 0.5             # 调节增益到 50%
```

## 已知限制

- **仅支持 Qwen3-ASR 0.6B**，不支持其他模型
- **仅 GNOME Wayland 测试过**，KDE/Hyprland 下粘贴可能不工作
- 浮动窗口开启时自动粘贴可能失败
- 依赖 NVIDIA CUDA GPU

## 协议

[MIT](LICENSE) — 原始 Handy 项目版权 (c) 2025 cjpais。
