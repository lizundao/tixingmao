# 提醒猫（Tixingmao）

一个独立的 Windows / macOS 桌面提醒工具。平时驻留系统托盘，到点后从屏幕指定位置出现一只会动的小猫和提醒文字；鼠标移入会暂停动画并显示关闭按钮。

所有数据保存在本机，不上传网络、不需要账号、无广告、无数据收集。MIT 开源许可。

## 下载

在 [Releases](https://github.com/lizundao/tixingmao/releases) 页面下载：

| 平台 | 文件 | 说明 |
| --- | --- | --- |
| Windows 安装包 | `Tixingmao-1.0.1-setup-win-x64.exe` | 当前用户安装，无需管理员权限 |
| Windows 便携版 | `Tixingmao-1.0.1-portable-win-x64.zip` | 解压即用，不写注册表、不联网 |
| macOS (Apple 芯片) | `Tixingmao-1.0.1-aarch64.dmg` | 若 dmg 打不开可用下方 app.zip |
| macOS (Apple 芯片) | `Tixingmao-1.0.1-aarch64.app.zip` | dmg 的替代分发形式 |

未签名构建首次运行可能触发系统安全提示，详见各平台说明。

## 已实现

- 一次性、每天、固定间隔提醒
- 中国法定工作日模式：跳过法定节假日，并正确处理官方调休上班日
- 可在系统设置里手动标记“休息”或“调休上班”，也可按行批量导入放假和调休日期，新一年的安排不用等软件更新
- 到点显示透明置顶悬浮层，不抢键盘焦点
- 悬停暂停猫咪动画并显示关闭按钮
- 猫咪默认显示位置可选左上、上中、右上、左中、居中、右中、左下、下中、右下
- 新建、编辑使用弹窗，删除前二次确认；系统级猫咪设置与单条提醒设置分开
- 提醒卡片可以按住左侧圆点拖动排序，顺序会保存在本地
- 系统托盘菜单、关闭主窗口后继续后台运行
- 本地 JSON 持久化，不需要账号或云服务
- Windows NSIS 安装包与 macOS DMG/App 构建配置

内置的节假日数据覆盖 2024–2027 年（含 2027 年国务院最新放假安排），来源见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。未公布年份会暂时按周一至周五判断。

## 开发

要求：Node.js 22+、Rust 1.90+。

    npm install
    npm run tauri:dev

Rust 依赖首次编译需要联网。前端单独检查：

    npm run build

Rust 单元测试：

    cd src-tauri
    cargo test

## 打包

Windows（生成 NSIS 安装程序，需在 Windows 上执行）：

    npm run tauri:build:win

Windows 绿色版（免安装，一条命令编译 exe + 打包 zip）：

    npm run tauri:build:exe
    npm run release:portable

产物在 `release/Tixingmao-1.0.1-portable-win-x64/`：提醒猫.exe + 使用说明.txt + 许可证。Tauri 会把 `dist/` 全部编进 exe，运行时不需要 `dist/`、`node_modules/`，也不写注册表、不联网；解压后整个目录拷到任意位置（U 盘也行）双击即可。唯一外部依赖是系统自带的 WebView2 Runtime（Win10/11 一般已预装）。

macOS（生成 .app / .dmg，必须在 macOS + Xcode 环境执行）：

    npm run tauri:build:mac

Windows 不能直接交叉编译 macOS 安装包。正式分发给 Apple 用户时，还需要 Apple Developer ID 证书签名与公证（notarization）；仅自己使用可以本地签名或右键打开未公证版本。

## 许可

MIT 开源许可，详见根目录 [`LICENSE`](./LICENSE)（英文）与 [`LICENSE.zh-CN.md`](./LICENSE.zh-CN.md)（中文译本，仅供参考，以英文为准）。内置节假日数据来源见 [`THIRD_PARTY_NOTICES.md`](./THIRD_PARTY_NOTICES.md)。

## 节假日数据更新

数据由开发依赖 chinese-days 转成 Rust 可直接读取的 JSON：

    npm run holiday:sync

每年国务院公布下一年度放假安排后，升级 chinese-days 的版本并重新生成数据即可。

## 数据位置

提醒记录保存在系统应用数据目录下的 reminders.json，显示位置设置保存在同目录的 settings.json，自定义节假日/调休保存在 calendar-overrides.json，不会上传到网络。
