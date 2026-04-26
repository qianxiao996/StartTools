# StartTools Preload API

插件页面可通过 `window.startTools` 使用 StartTools 暴露的能力。
`window.startToolsPlugin` 是同一套 API 的兼容别名。

```js
const st = window.startTools;
```

## meta

当前插件信息。

```js
st.meta
```

示例返回：

```js
{
  id: "plugin_id",
  name: "Plugin Name",
  main: "index.html",
  preload: "preload.js"
}
```

## dialog

系统文件/目录选择。

```js
await st.dialog.openFile(options)
await st.dialog.openFiles(options)
await st.dialog.openDirectory(options)
await st.dialog.saveFile(options)
```

常用参数：

```js
{
  title: "选择文件",
  defaultPath: "C:/Users/name",
  directoryPath: "C:/Users/name",
  fileName: "test.txt"
}
```

返回值：

- 单选文件/目录：路径字符串，取消时为 `null`
- 多选文件：路径数组，取消时为空数组

## fs

文件系统能力。

```js
await st.fs.readText(path)
await st.fs.writeText(path, content)
await st.fs.readBytes(path)
await st.fs.writeBytes(path, base64)
await st.fs.exists(path)
await st.fs.stat(path)
await st.fs.mkdir(path)
await st.fs.remove(path)
await st.fs.copy(from, to)
await st.fs.rename(from, to)
await st.fs.readDir(path)
```

说明：

- `readText(path)` 返回文本内容。
- `writeText(path, content)` 写入文本。
- `readBytes(path)` 返回 base64 字符串。
- `writeBytes(path, base64)` 写入 base64 内容。
- `exists(path)` 返回布尔值。
- `stat(path)` 返回文件状态。
- `readDir(path)` 返回目录列表。

`stat` 返回示例：

```js
{
  path: "C:/test/a.txt",
  isFile: true,
  isDir: false,
  isSymlink: false,
  len: 123,
  modified: 1710000000000,
  created: 1710000000000
}
```

`readDir` 返回示例：

```js
[
  {
    name: "a.txt",
    path: "C:/test/a.txt",
    isFile: true,
    isDir: false,
    len: 123
  }
]
```

## path

纯前端路径处理，统一使用 `/`。

```js
st.path.join(...parts)
st.path.dirname(path)
st.path.basename(path)
st.path.extname(path)
st.path.normalize(path)
```

示例：

```js
st.path.basename("C:/test/a.txt") // "a.txt"
st.path.extname("C:/test/a.txt")  // ".txt"
```

## shell

系统打开与命令执行。

```js
await st.shell.openPath(path)
await st.shell.showInFolder(path)
await st.shell.openUrl(url)
await st.shell.exec(command, args, options)
```

示例：

```js
const res = await st.shell.exec("cmd.exe", ["/C", "echo hello"]);
```

返回：

```js
{
  status: 0,
  success: true,
  stdout: "hello\r\n",
  stderr: ""
}
```

## clipboard

剪贴板能力。

```js
await st.clipboard.writeText(text)
await st.clipboard.readText()
await st.clipboard.writeImage(base64)
await st.clipboard.readImage()
await st.clipboard.writeFiles(paths)
await st.clipboard.clear()
```

说明：

- `writeText/readText` 使用浏览器文本剪贴板。
- `writeImage(base64)` 走 Rust bridge，当前会保存临时 PNG 并复制临时文件路径。
- `readImage()` 已预留，当前暂未完整实现。
- `writeFiles(paths)` 当前会把路径列表写入剪贴板文本。
- `clear()` 清空剪贴板文本。

## input

系统输入模拟。

```js
await st.input.typeText(text)
await st.input.pasteText(text)
await st.input.pasteFile(path)
```

说明：

- 当前主要面向 Windows。
- `typeText(text)` 模拟键盘输入。
- `pasteText(text)` 写入剪贴板后模拟 `Ctrl+V`。
- `pasteFile(path)` 当前写入文件路径文本后模拟 `Ctrl+V`。

## screen

屏幕能力。

```js
await st.screen.getDisplays()
await st.screen.getPrimaryDisplay()
await st.screen.screenshot()
await st.screen.pickColor()
```

说明：

- `getDisplays()` 当前返回主屏列表。
- `getPrimaryDisplay()` 返回主屏信息。
- `screenshot()` 当前截取 Windows 主屏，返回 PNG base64/dataUrl。
- `pickColor()` 取当前鼠标位置颜色。

`getPrimaryDisplay` 返回示例：

```js
{
  id: "primary",
  primary: true,
  x: 0,
  y: 0,
  width: 1920,
  height: 1080,
  scaleFactor: 1
}
```

`screenshot` 返回示例：

```js
{
  width: 1920,
  height: 1080,
  type: "image/png",
  base64: "...",
  dataUrl: "data:image/png;base64,..."
}
```

`pickColor` 返回示例：

```js
{
  x: 100,
  y: 100,
  r: 255,
  g: 255,
  b: 255,
  hex: "#FFFFFF"
}
```

## system

系统信息与提示。

```js
await st.system.notification(title, body)
await st.system.osInfo()
await st.system.env(name)
await st.system.envs()
await st.system.cpu()
await st.system.memory()
await st.system.disk()
await st.system.network()
await st.system.battery()
```

说明：

- `notification(title, body)` 当前使用系统弹窗提示。
- `osInfo()` 返回 `{ os, arch, family }`。
- `env(name)` 获取单个环境变量。
- `envs()` 获取全部环境变量。
- `cpu()` 获取 CPU 信息。
- `memory()` 获取内存信息。
- `disk()` 获取磁盘信息。
- `network()` 获取网络信息。
- `battery()` 获取电池信息。

说明：`cpu/memory/disk/network/battery` 当前主要面向 Windows，通过 PowerShell/CIM 获取。

## process

进程能力。

```js
await st.process.spawn(command, args, options)
await st.process.kill(pid)
await st.process.list()
```

`spawn` 示例：

```js
const child = await st.process.spawn("notepad.exe")
```

返回：

```js
{
  pid: 1234
}
```

`kill` 示例：

```js
await st.process.kill(1234)
```

`list` 返回当前进程列表。Windows 下通过 PowerShell 获取。

## image

图片处理能力。

```js
await st.image.size(path)
await st.image.resize(input, output, options)
await st.image.toBase64(path)
await st.image.fromBase64(base64, output)
```

`size` 返回：

```js
{
  width: 800,
  height: 600
}
```

`resize` 示例：

```js
await st.image.resize(
  "C:/test/input.png",
  "C:/test/output.png",
  { width: 300, height: 200 }
)
```

说明：

- `resize` 支持只传 `width` 或只传 `height`，会按比例缩放。
- `toBase64` 返回 `{ base64, type, dataUrl }`。
- `fromBase64` 将 base64 写入输出路径。

## app

应用路径。

```js
await st.app.getPath(name)
```

支持：

```text
home
desktop
documents
downloads
appData
temp
exe
plugin
```

示例：

```js
const desktop = await st.app.getPath("desktop");
```

## storage

插件隔离本地存储，基于 `localStorage`，key 前缀按插件 id 区分。

```js
st.storage.get(key, fallback)
st.storage.set(key, value)
st.storage.remove(key)
st.storage.clear()
```

## db

当前 `db` 使用同一套插件隔离存储。

```js
st.db.get(key, fallback)
st.db.set(key, value)
st.db.remove(key)
st.db.keys()
st.db.clear()
```

示例：

```js
st.db.set("settings", { theme: "dark" });
const settings = st.db.get("settings");
```

## http

HTTP 请求。

```js
st.http.request(options)
```

当前是浏览器 `fetch` 的简单包装。

示例：

```js
await st.http.request({
  url: "https://example.com",
  method: "GET"
});
```

## 快捷别名

```js
await st.selectFile()
await st.selectFiles()
await st.selectDirectory()
st.downloadText(filename, content, type)
```

这些别名用于更快编写简单插件。
