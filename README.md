# OneLauncher Ely

**Unofficial community fork** of [Polyfrost/OneLauncher](https://github.com/Polyfrost/OneLauncher). Not affiliated with, endorsed by, or supported by Polyfrost.

It adds:

- **Offline accounts** — create a local account without a Microsoft account (singleplayer, LAN, `online-mode=false` servers)
- **Ely.by accounts** — free community authentication & skin server with visible online skins/capes on Ely.by-enabled servers
- **Linux builds** — `.deb` and AppImage plus Windows NSIS installer, produced by GitHub Actions
- Microsoft accounts still work exactly as before

**Official project:** please download the official launcher here: https://github.com/Polyfrost/OneLauncher
This repository exists for people who specifically want the offline/Ely.by features in a community build.

## Install

Download the latest release from the [Releases page](https://github.com/AlanNobita/OneLauncher-Ely/releases).

| Platform | How to install |
|---|---|
| Windows 10/11 (x86_64) | Download `OneLauncher Ely_*_x64-setup.exe`, run it, click through the installer. It is **not Authenticode-signed**, so SmartScreen may warn you — **More info → Run anyway**. |
| Debian / Ubuntu / Mint | Download `*.deb`, then `sudo apt install ./onelauncher-ely_*_amd64.deb` |
| Other Linux (x86_64) | Download `*.AppImage`, `chmod +x`, run it |

Windows unsigned builds are expected. Verify checksums below if you're cautious.

## Verify checksums

Each release includes SHA-256 checksums for every artifact. On Linux:
```sh
sha256sum onelauncher-ely*.{deb,AppImage,exe}
cat SHA256SUMS.txt
```
On Windows (PowerShell):
```powershell
Get-FileHash .\*-setup.exe -Algorithm SHA256
Get-Content SHA256SUMS.txt
```

## What offline and Ely.by do and don't do

| Feature | Microsoft | Offline | Ely.by |
|---|---|---|---|
| Original Mojang online servers (Hypixel, etc.) | ✅ | ❌ no | ❌ no |
| Ely.by-authenticated servers | ❌ | ❌ | ✅ |
| LAN / `online-mode=false` server | ✅ | ✅ | ✅ |
| Singleplayer worlds | ✅ | ✅ | ✅ |
| Custom skins visible to others | ✅ official | ❌ local only | ✅ on Ely.by servers |
| PolyPlus (chat, lobby, wardrobe, cosmetics) | ✅ | ❌ no | ❌ no |

Offline accounts:
- Username must be 3–16 characters, only letters/digits/underscore
- UUID is `MD5("OfflinePlayer:<name>")` — the same algorithm vanilla uses
- Work for singleplayer, LAN, and servers with `online-mode=false`
- No Mojang validation is performed

Ely.by accounts:
- Login with your Ely.by username/email + password (TOTP supported)
- The launcher downloads and signature-checks `authlib-injector.jar` on first launch
- Visible skins/capes on Ely.by servers

## Building from source

Requires Rust 1.97+.

```sh
cargo run -p oneclient_app
cargo build --release -p oneclient_app
cargo packager --release -p oneclient_app --formats nsis
cargo packager --release -p oneclient_app --formats deb,appimage
```

On Linux you need system build dependencies: cmake, clang/gcc, pkg-config, libfontconfig, libfreetype, libgl/mesa, X11/Wayland/kbcommon dev packages.

## License & attribution

GPL-3.0-only. See [LICENSE](./LICENSE) and [ATTRIBUTION.md](./ATTRIBUTION.md).

Original work by Polyfrost and Contributors. This fork preserves attribution and license headers. Polyfrost branding (logo, wordmarks, banners) is removed and is **not** used for this fork.
