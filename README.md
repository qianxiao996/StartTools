# StartTools

StartTools 是一个基于 Tauri + Vue 3 的桌面工具启动器。它可以管理常用程序、脚本工具和本地 HTML 插件，并提供全局搜索、快捷键、分类标签、插件窗口、数据备份等能力。

## 功能特性

- 常用工具管理：按分类和标签管理本地程序、文件、目录或命令。
- 全局搜索：默认 `Alt+3` 打开搜索框。
- 主窗口快捷键：默认 `Alt+2` 显示或隐藏主窗口。
- 搜索前缀：
  - 普通输入：搜索常规工具。
  - `>名称`：搜索内置工具。
  - `/名称`：搜索 StartTools 插件。
- 内置工具：支持按系统区分脚本，支持 CMD、PowerShell、Bash、Sh、Zsh。
- StartTools 插件：支持本地 HTML 插件，插件文件放在 `plugins/插件ID`。
- 插件窗口：使用 Edge/Chrome App 模式打开插件页面，并支持 Preload API。
- 贴边隐藏：主窗口靠近屏幕边缘后可自动隐藏，鼠标靠边可唤出。
- 皮肤：支持浅色和深色主题。
- 数据导入导出：
  - 全量备份：数据库和插件文件合并为一个 `.stbak` 文件。
  - 内置工具单独导入导出。
  - StartTools 插件单独导入导出，包含插件记录和插件文件。

## 技术栈

- Tauri 2
- Rust
- Vue 3
- Vite
- Element Plus
- SQLite

## 开发环境

需要先安装：

- Node.js
- Rust
- Tauri 2 相关系统依赖

安装依赖：

```bash
npm install
```

启动开发模式：

```bash
npm run tauri dev
```

前端构建：

```bash
npm run build
```

Rust 构建检查：

```bash
cd src-tauri
cargo build
```

打包：

```bash
npm run tauri build
```

## 目录结构

```text
StartTools/
├─ src/                    # Vue 前端
├─ src-tauri/              # Tauri / Rust 后端
├─ plugins/                # StartTools 插件目录
├─ public/                 # 静态资源
├─ STARTTOOLS_PRELOAD_API.md
├─ package.json
└─ README.md
```

## StartTools 插件

StartTools 插件是一个本地 HTML 应用。每个插件有独立目录：

```text
plugins/
└─ 插件ID/
   ├─ index.html
   ├─ preload.js
   └─ logo.svg
```

插件记录在工具管理页面中维护，主要字段包括：

- 插件 ID
- 名称
- 入口
- Logo
- Preload
- 窗口大小
- 排序

插件打开时会通过本地 HTTP 服务访问，例如：

```text
http://127.0.0.1:13678/plugins/插件ID/index.html
```

Preload API 说明见 [STARTTOOLS_PRELOAD_API.md](./STARTTOOLS_PRELOAD_API.md)。

## 数据说明

运行时数据默认放在程序目录：

- `data.db`：分类、标签、工具、内置工具、插件记录、配置。
- `plugins/`：StartTools 插件文件。

导入全量备份时会自动备份当前数据：

- `data.db.bak`
- `plugins.bak`

## 常用操作

- 标题栏设置菜单：设置、工具管理、导入数据、导出数据、路径转换、退出。
- 工具管理：
  - 管理内置工具。
  - 管理 StartTools 插件。
  - 导入/导出内置工具。
  - 导入/导出 StartTools 插件。
- 主页面：
  - 右键工具可运行、编辑、删除、复制路径、打开目录、打开命令行。
  - 拖入外部文件可新增工具。

## 备注

当前项目主要面向 Windows 使用，部分脚本执行、图标提取、窗口行为和浏览器 App 模式依赖 Windows 环境。
