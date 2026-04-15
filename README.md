# TermAVG (TMJ)

<!-- PROJECT SHIELDS -->
[![Contributors][contributors-shield]][contributors-url]
[![Forks][forks-shield]][forks-url]
[![Stargazers][stars-shield]][stars-url]
[![Issues][issues-shield]][issues-url]

<p align="center">
  <img src="doc/logo.png" alt="TMJ Logo" width="200" height="80">
</p>

<p align="center">
  一个使用 Rust 编写、在终端渲染的像素风文字冒险（AVG）引擎。<br />
  引擎以脚本解释器驱动剧情，支持对话、角色、音频和存档流程。<br />
  <a href="https://github.com/rabitank/TerminalLove">查看游戏《终末之爱》示例项目</a>
</p>

## 目录

- [TermAVG (TMJ)](#termavg-tmj)
  - [目录](#目录)
  - [项目特性](#项目特性)
  - [项目结构](#项目结构)
  - [快速开始](#快速开始)
    - [环境要求](#环境要求)
    - [克隆与构建](#克隆与构建)
    - [运行](#运行)
  - [配置文件](#配置文件)
  - [脚本说明](#脚本说明)
    - [分段规则](#分段规则)
    - [常见命令](#常见命令)
    - [最小示例](#最小示例)
  - [开发说明](#开发说明)
  - [依赖](#依赖)
  - [贡献](#贡献)

## 项目特性

- 终端渲染：基于 `ratatui` + `crossterm` 的 TUI 绘制与事件处理。
- 脚本驱动：内置脚本解析器，支持赋值、调用、`wait`、链式调用等语法。
- 多模块工作区：`tmj_app`、`tmj_core`、`tmj_macro` 。
- 可配置启动：通过 `setting.toml` 指定分辨率、资源路径和布局参数。
- 音频与资源：支持角色立绘、表情资源和音频播放。

## 项目结构

```text
tmj/
├─ src/                # 入口（main）
├─ tmj_app/            # 游戏逻辑、页面、脚本变量与渲染流程
├─ tmj_core/           # 脚本系统、事件系统、资源路径与通用能力
├─ tmj_macro/          # 过程宏
├─ resource/           # 脚本与资源文件（示例角色等）
├─ setting.toml        # 运行配置
└─ README.md
```

## 快速开始

### 环境要求

- Rust 工具链（建议稳定版，支持 `edition = "2024"`）
- Windows / Linux / macOS 终端环境

### 克隆与构建

```bash
git clone https://github.com/rabitank/TermAVG.git
cd TermAVG
cargo build
```

### 运行

```bash
cargo run
```
建议使用,否则会出现debug模式下日志打印和游戏界面冲突
```bash
cargo run 2> debug.txt
```

首次运行时如果没有 `setting.toml`，程序会按默认配置自动创建。

## 配置文件

`setting.toml` 关键字段示例：

```toml
resolution = [240, 80]
is_force_skipable = false
save_dir = "save"
entre_script = "resource/script.fs"
default_bg_img = "resource/default_background_img.png"
default_face_img = "resource/default_face_img.png"
```

- `resolution`: 终端渲染尺寸（字符单位）。
- `save_dir`: 存档目录。
- `entre_script`: 入口脚本路径。
- `default_bg_img` / `default_face_img`: 默认背景和头像资源,是必须的

> 注意：路径均相对于项目根目录解析。

## 脚本说明

### 分段规则

脚本使用 `#数字` 作为段落分隔标记，例如 `#1`、`#2`。引擎按段读取剧情内容。
脚本后缀为`fss`, 你也可以在setting中指定需要自动添加`#`序号的文件,会在游戏开始时处理生成对应的`fss`文件.目前默认路径在`resource`下

### 常见命令

根据当前解析器，脚本支持以下形式：

- `变量 = 值`（赋值）
- `变量 = 命令 参数...`（命令返回值赋值）
- `对象.方法 参数...`（调用）
- `set 路径 参数...`（设置）
- `once 路径 参数...`（一次性命令,在该段落结束后会还原）
- `wait 0.5`（等待时间）
- `命令1 -> 命令2`（链式调用）

### 最小示例

可以先在 `resource/script.fs` 中写入类似内容进行测试：

```txt
#1
title = "TermAVG Demo"
wait 0.5
wait click

#2
wait 0.2
```

## 开发说明

- 工作区包含多个 crate，建议在项目根目录执行 `cargo check` / `cargo test`。
- 与脚本相关的核心代码位于 `tmj_core/src/script/`。
- 引擎页面和渲染流程位于 `tmj_app/src/pages/` 与 `tmj_app/src/game.rs`。

## 依赖

- TUI: [ratatui](https://github.com/ratatui/ratatui), [crossterm](https://github.com/crossterm-rs/crossterm)
- 序列化: [serde](https://github.com/serde-rs/serde), [toml](https://github.com/toml-rs/toml)
- 音频: [rodio](https://github.com/RustAudio/rodio)
- 其他: `tracing`, `anyhow`, `image`, `strum`

## 贡献

目前添加功能,完善示例中,欢迎提交 Issue 和 PR：
- 功能建议：请描述使用场景和预期行为

<!-- links -->
[contributors-shield]: https://img.shields.io/github/contributors/rabitank/TermAVG.svg?style=flat-square
[contributors-url]: https://github.com/rabitank/TermAVG/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/rabitank/TermAVG.svg?style=flat-square
[forks-url]: https://github.com/rabitank/TermAVG/network/members
[stars-shield]: https://img.shields.io/github/stars/rabitank/TermAVG.svg?style=flat-square
[stars-url]: https://github.com/rabitank/TermAVG/stargazers
[issues-shield]: https://img.shields.io/github/issues/rabitank/TermAVG.svg?style=flat-square
[issues-url]: https://github.com/rabitank/TermAVG/issues
