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

## 签名与公证（macOS）

`build-release.sh` 会自动拿钥匙串里第一张 **Developer ID Application** 证书签名，
连带 app 里那个 MediaRemote 桥接 dylib（公证不接受只有 ad-hoc 签名的二进制）。
硬化运行时（hardened runtime）是打开的，`src-tauri/entitlements.plist` 里那条
`com.apple.security.automation.apple-events` 不能删 —— 少了它，公证后的版本
控制 Spotify 会被系统直接拒掉。

公证凭据存一次就够，**密码只经过你的手**：

```bash
xcrun notarytool store-credentials bangs-notary \
  --apple-id <你的 Apple ID> --team-id W8L8ZJ3N2P --password <App 专用密码>
export BANGS_NOTARY_PROFILE=bangs-notary   # 建议写进 shell 配置
```

App 专用密码在 https://account.apple.com 的「登录与安全 → App 专用密码」里生成。
之后 `scripts/build-release.sh` 会自动公证 `.app` 和 `.dmg` 并 staple 票据，
最后跑一次 `spctl` 验证。没设 `BANGS_NOTARY_PROFILE` 就只签名、不公证。

装好之后确认一句话就够：`spctl -a -vv /Applications/Bangs.app` 说
`accepted / source=Notarized Developer ID` 就对了。

## Windows 没签名

SmartScreen 会拦，点「更多信息 → 仍要运行」。Windows 的签名要另买证书（EV 或 OV），
目前没有。

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
