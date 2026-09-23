# 验证记录 · 0.1.0

本地验证环境：Windows x64、Rust 1.90、Node.js 24、系统已有的 Microsoft Edge WebView2。

| 检查 | 结果 |
| --- | --- |
| Rust 核心算法测试 | 21 项通过 |
| 原生文件与流式转换测试 | 15 项通过 |
| 前端测试 | 10 项通过 |
| TypeScript、Rust 检查及格式检查 | 通过 |
| 前端与 Windows release 构建 | 通过 |
| 实际便携 EXE 操作检查 | 20 项通过 |
| 三种语言的 820 × 640 和 680 × 520 布局 | 无页面溢出 |

桌面检查覆盖首次启动默认英文、文字输入、候选切换、转换路径、对比、证据、评分、复制、二次乱码恢复、丢失提示、正常文本、打开文件、Hex、另存、拒绝覆盖、语言切换、浅色偏好迁移与窗口尺寸。检查期间没有前端异常或外部页面网络请求。测试仅替代操作系统文件选择器的返回路径；读取、分析和写出均执行真实 Rust 命令。

可在项目 PowerShell 中复现：

```powershell
. ./scripts/env.ps1
./scripts/build.ps1
./scripts/test-desktop.ps1 -Executable release/MojibakeLab/MojibakeLab.exe
```

`docs/screenshots` 中的图片来自实际 release 应用。GitHub Actions 配置了 Windows、macOS 和 Linux 的验证与构建；本记录不宣称已在 macOS/Linux 真机运行。系统级文件拖放未纳入自动化桌面检查。
