# 系统依赖

仅 **Arch Linux + Wayland + GNOME** 实机测试通过。

## 必须

| 组件 | 说明 | 安装 |
|------|------|------|
| Rust (stable) | 编译后端 | `rustup` |
| Bun | 包管理器 + 前端构建 | `pacman -S bun` |
| Python 3.12 | Sidecar 推理（uv 管理 venv） | `pacman -S python` + `uv` |
| PyTorch CUDA | GPU 推理 | pip 自动安装（见 setup.sh） |
| uinput 内核模块 | ydotool 所需，开机加载 | `/etc/modules-load.d/uinput.conf` |
| wl-clipboard | Wayland 剪贴板（wl-copy/wl-paste） | `pacman -S wl-clipboard` |
| ydotool | 模拟 Ctrl+V 按键（需 ydotoold 守护） | `pacman -S ydotool` |

## 可选

| 组件 | 说明 |
|------|------|
| wtype | Wayland 文本输入（GNOME 不支持） |
| dotool | uinput 文本输入 |
| xdotool | X11 下文本输入 |

## 验证

```bash
# pipewire 必须运行
pactl info | grep "Server Name"

# uinput 必须加载
lsmod | grep uinput

# ydotool 守护进程
systemctl --user status ydotoold
```
