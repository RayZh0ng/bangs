# 发一个版本

每次发版就这几步，脚本都在这个目录里。

```bash
scripts/version.sh 0.2.0        # 三个文件一起改（package.json / tauri.conf.json / Cargo.toml）
git commit -am "Release v0.2.0"
git tag v0.2.0 && git push origin main --tags

scripts/build-release.sh        # mac 本机打包 + ssh 到 Windows 打包，产物收到 dist/release/v0.2.0/
scripts/publish-site.sh         # 站点推到 gh-pages
```

产物：

| 文件 | 平台 |
| --- | --- |
| `Bangs_<版本>_aarch64.dmg` | macOS，拖进「应用程序」 |
| `Bangs_<版本>_aarch64.app.zip` | macOS，不想挂载磁盘映像的人 |
| `Bangs_<版本>_x64-setup.exe` | Windows，NSIS 安装包 |
| `Bangs_<版本>_x64_en-US.msi` | Windows，MSI（给有策略要求的场合） |

然后在 GitHub 上建 Release：标签填 `v0.2.0`，把 `dist/release/v0.2.0/` 里的四个文件全传上去。
**资产文件名必须保留 `.dmg` / `.app.zip` / `setup.exe` / `.msi` 后缀** —— 官网靠后缀认哪个是哪个平台的包
（见 `site/app.js` 里的 `MAC` / `WINDOWS` 两个正则），应用内的「检查更新」只认 `tag_name`。

## 两端都没签名

- macOS：第一次打开要去「系统设置 → 隐私与安全性 → 仍要打开」，或者
  `xattr -dr com.apple.quarantine /Applications/Bangs.app`。
- Windows：SmartScreen 会拦，点「更多信息 → 仍要运行」。

## Windows 构建机

`scripts/build-release.sh` 默认用 ssh 别名 `win-gxl`（改 `BANGS_WIN_HOST` 可以换别的机器）。
那台机器上**所有缓存都在 D 盘**：`CARGO_HOME`、`CARGO_TARGET_DIR`、pnpm store，
连 Tauri 自己下载 NSIS/WiX 的 `%LOCALAPPDATA%` 都被指到了 `D:\Develop\build-cache`。
它需要预装：Rust（MSVC toolchain）、Node + pnpm、Git、WebView2 运行时。

ssh 登进 Windows 落在 session 0，画不出界面，所以要在那台机器桌面上**看**刘海得用交互式计划任务：

```powershell
schtasks /create /tn Bangs /tr "D:\Develop\Bangs\bangs.exe" /sc once /st 00:00 /it /f
schtasks /run /tn Bangs
```
