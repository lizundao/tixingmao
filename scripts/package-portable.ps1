# 提醒猫 · Windows 绿色便携版打包
# 前提：先执行 npx tauri build --no-bundle 生成 src-tauri/target/release/tixingmao.exe
# exe 已把前端资源全部内嵌，解压即用，不写注册表、不联网。
$ErrorActionPreference = 'Stop'

$version = '1.0.1'
$root = Split-Path -Parent $PSScriptRoot
$exeSrc = Join-Path $root 'src-tauri/target/release/tixingmao.exe'
$releaseDir = Join-Path $root 'release'
$folderName = "Tixingmao-$version-portable-win-x64"
$outDir = Join-Path $releaseDir $folderName
$zipPath = Join-Path $releaseDir "$folderName.zip"

if (-not (Test-Path $exeSrc)) {
  throw "未找到编译产物：$exeSrc`n请先运行：npx tauri build --no-bundle"
}

New-Item -ItemType Directory -Force -Path $outDir | Out-Null
Copy-Item $exeSrc (Join-Path $outDir '提醒猫.exe') -Force
foreach ($name in @('README.md', 'LICENSE', 'LICENSE.zh-CN.md', 'THIRD_PARTY_NOTICES.md')) {
  $src = Join-Path $root $name
  if (Test-Path $src) { Copy-Item $src $outDir -Force }
}

# 使用说明用 UTF-8 BOM，记事本直接打开不乱码
$notePath = Join-Path $outDir ([char]0x4f7f + [char]0x7528 + [char]0x8bf4 + [char]0x660e + '.txt')
$note = @"
提醒猫 · 绿色版 v$version
========================================

【运行】
双击「提醒猫.exe」即可，不用安装、不写注册表、不联网。
关闭主窗口不会退出，提醒猫会继续在桌面右下角托盘里运行。

【新建提醒】
点右上角「新建提醒」，起个名字，选方式：
  · 仅一次   —— 指定日期时间，到点提醒一次
  · 每天     —— 指定每天几点
  · 间隔提醒 —— 每隔 N 分钟 / 小时 / 天
勾选「只在法定工作日提醒」会自动跳过周末和国家法定节假日，
并正确处理官方调休上班日；新一年的放假安排可在「系统设置」里手动补。

【猫咪出现】
到点后一只会动的小猫从屏幕指定位置弹出，鼠标移入会暂停并显示关闭按钮。
出现位置可在「系统设置」里选择（左上/上中/右上/左中/居中/右中/左下/下中/右下）。
想先看效果，点「预览猫咪」。

【托盘】
右键托盘图标：打开提醒列表 / 新建提醒 / 测试悬浮提醒 / 退出提醒猫。
左键单击托盘图标：打开提醒列表。

【数据】
提醒记录、显示位置、自定义节假日都保存在本机应用数据目录，
不会上传网络，不需要账号。

【运行环境】
Windows 10 / 11 64 位，需要系统已安装 WebView2 Runtime
（Win10 / Win11 一般自带；若提示缺少，安装微软 WebView2 Runtime 后重开即可）。

【关于】
点主窗口右上角「关于」可查看版本、开源许可（MIT）、
第三方声明与官网 inkdot.cn。软件由墨点 inkdot 提供。

官网：inkdot.cn    源码：github.com/lizundao/tixingmao
"@
[System.IO.File]::WriteAllText($notePath, $note, [System.Text.UTF8Encoding]::new($true))

if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
Compress-Archive -Path $outDir -DestinationPath $zipPath -Force

Write-Host "绿色版目录：$outDir"
Get-ChildItem $outDir | ForEach-Object { Write-Host ('  ' + $_.Name + '  ' + [math]::Round($_.Length / 1MB, 2) + ' MB') }
Write-Host "压缩包：$zipPath  $([math]::Round((Get-Item $zipPath).Length / 1MB, 2)) MB"
