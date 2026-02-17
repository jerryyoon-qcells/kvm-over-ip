# KVM-Over-IP — Packaging and Installer Guide

This document explains how to build installer packages for all three supported platforms.

---

## Table of Contents

1. [Overview](#overview)
2. [What Each Installer Contains](#what-each-installer-contains)
3. [Platform Requirements](#platform-requirements)
4. [Building Installers Locally](#building-installers-locally)
   - [Windows (MSI)](#windows-msi)
   - [Linux (.deb)](#linux-deb)
   - [macOS (.dmg)](#macos-dmg)
5. [Code Signing](#code-signing)
   - [Windows Code Signing](#windows-code-signing)
   - [Linux Package Signing](#linux-package-signing)
   - [macOS Code Signing and Notarisation](#macos-code-signing-and-notarisation)
6. [CI/CD Release Automation](#cicd-release-automation)
7. [Troubleshooting](#troubleshooting)

---

## Overview

The KVM-Over-IP project uses three different packaging systems, one per platform:

| Platform | Installer Format | Tool        | Source                                       |
|----------|-----------------|-------------|----------------------------------------------|
| Windows  | `.msi`          | cargo-wix   | `src/crates/kvm-master/wix/main.wxs`         |
|          |                 |             | `src/crates/kvm-client/wix/main.wxs`         |
| Linux    | `.deb`          | cargo-deb   | `src/crates/kvm-client/Cargo.toml` `[package.metadata.deb]` |
|          |                 |             | `src/crates/kvm-web-bridge/Cargo.toml`       |
| macOS    | `.dmg`          | cargo-bundle + hdiutil | `build/macos/Info.plist`        |

The local packaging scripts in `build/` let you test packaging before pushing
a release tag to GitHub.  The `release.yml` workflow automates everything for
official releases.

---

## What Each Installer Contains

### Windows kvm-master MSI

Installs the master application that runs on the machine with the physical
keyboard and mouse (Windows only because it uses Win32 keyboard/mouse hooks).

**Installed files:**

```
C:\Program Files\KVM-Over-IP\
    kvm-master.exe          — main application binary
```

**Start Menu shortcuts:**

```
Start Menu → KVM-Over-IP → KVM-Over-IP Master
```

**Optional Desktop shortcut:** shown during installation, pre-selected by default.

**Add/Remove Programs entry:** yes — appears in "Programs and Features".

---

### Windows kvm-client MSI

Installs the client application that runs on machines that will receive
forwarded keyboard/mouse input.

**Installed files:**

```
C:\Program Files\KVM-Over-IP\
    kvm-client.exe          — main application binary
```

**Start Menu shortcuts:** yes.  **Desktop shortcut:** optional.

---

### Linux kvm-client .deb

**Installed files:**

```
/usr/bin/kvm-client                                     — binary (executable by all users)
/usr/share/applications/kvm-client.desktop              — desktop launcher for GNOME/KDE
/usr/lib/systemd/user/kvm-client.service                — systemd user service unit
```

**Runtime dependencies:** `libx11-6`, `libxtst6`, `libc6`

The systemd user service allows kvm-client to start automatically on user login:
```bash
systemctl --user enable kvm-client
systemctl --user start kvm-client
```

---

### Linux kvm-web-bridge .deb

**Installed files:**

```
/usr/bin/kvm-web-bridge                                 — binary
/lib/systemd/system/kvm-web-bridge.service              — systemd system service unit
/etc/default/kvm-web-bridge                             — configuration file (created by postinst)
```

**System user:** the `postinst` script creates a `kvm-bridge` system user to run
the service in isolation.

The service is automatically enabled and started after installation:
```bash
sudo systemctl status kvm-web-bridge
sudo journalctl -u kvm-web-bridge -f
```

To change ports, edit `/etc/default/kvm-web-bridge` and restart:
```bash
sudo nano /etc/default/kvm-web-bridge
sudo systemctl restart kvm-web-bridge
```

---

### macOS kvm-client .dmg

**Bundle contents:**

```
KVM-Over-IP Client.app/
    Contents/
        Info.plist              — application metadata
        MacOS/
            kvm-client          — compiled Rust binary
        Resources/
            AppIcon.icns        — application icon
```

**Installation:** open the `.dmg`, drag `KVM-Over-IP Client.app` to the `/Applications` shortcut.

**Accessibility permission:** on first launch, macOS will prompt for Accessibility access
(required for CGEvent input injection).  The user must click "Open System Preferences"
and toggle the switch next to `KVM-Over-IP Client` in:

> System Preferences → Privacy & Security → Accessibility

---

## Platform Requirements

### Windows Build Machine

| Requirement       | Version     | Purpose                              |
|-------------------|-------------|--------------------------------------|
| Windows           | 10 or later | Required OS for Win32 hook code      |
| Rust stable       | 1.75+       | Compile the Rust crates              |
| cargo-wix         | 0.3.x       | Generate MSI from .wxs files         |
| WiX Toolset v3    | 3.11+       | WiX compiler/linker (`candle`/`light`) |

Install cargo-wix:
```powershell
cargo install cargo-wix --version "0.3"
```

Download WiX Toolset v3: https://wixtoolset.org/releases/

---

### Linux Build Machine

| Requirement       | Version     | Purpose                              |
|-------------------|-------------|--------------------------------------|
| Linux (Debian/Ubuntu) | 22.04+  | Required for dpkg/apt tooling        |
| Rust stable       | 1.75+       | Compile the Rust crates              |
| cargo-deb         | 2.x         | Generate .deb packages               |
| libx11-dev        | any         | Compile-time X11 headers             |
| libxtst-dev       | any         | Compile-time XTest headers           |

Install cargo-deb:
```bash
cargo install cargo-deb
```

Install X11 headers:
```bash
sudo apt install libx11-dev libxtst-dev
```

---

### macOS Build Machine

| Requirement             | Version     | Purpose                              |
|-------------------------|-------------|--------------------------------------|
| macOS                   | 12.0+       | Required for CoreGraphics APIs       |
| Xcode Command Line Tools | latest     | Compiler, hdiutil, codesign          |
| Rust stable             | 1.75+       | Compile the Rust crates              |
| cargo-bundle            | 0.6+        | Generate .app bundle                 |
| create-dmg (optional)   | any         | Polished DMG (falls back to hdiutil) |
| Inkscape (optional)     | any         | Convert AppIcon.svg → AppIcon.icns   |

Install cargo-bundle:
```bash
cargo install cargo-bundle
```

Install create-dmg and Inkscape via Homebrew:
```bash
brew install create-dmg inkscape
```

---

## Building Installers Locally

### Windows (MSI)

Run from the project root in a PowerShell terminal:

```powershell
.\build\package-windows.ps1
```

Options:
```powershell
.\build\package-windows.ps1 -SkipBuild     # Use existing binaries
.\build\package-windows.ps1 -OnlyMaster    # Only build kvm-master.msi
.\build\package-windows.ps1 -OnlyClient    # Only build kvm-client.msi
```

Output: `dist\windows\*.msi` and `dist\windows\*.exe`

To install the produced MSI:
```powershell
msiexec /i dist\windows\kvm-master-0.1.0-x86_64.msi
```

To install silently (no UI):
```powershell
msiexec /i dist\windows\kvm-master-0.1.0-x86_64.msi /quiet
```

**IMPORTANT — icon files:** cargo-wix expects icon files at:
- `src/crates/kvm-master/wix/kvm-master.ico`
- `src/crates/kvm-client/wix/kvm-client.ico`

If these files are absent, remove the `<Icon>` elements from the `.wxs` files
or provide placeholder `.ico` files before building.

---

### Linux (.deb)

Run from the project root in a bash terminal:

```bash
./build/package-linux.sh
```

Options:
```bash
./build/package-linux.sh --skip-build     # Use existing binaries
./build/package-linux.sh --only-client    # Only kvm-client .deb
./build/package-linux.sh --only-bridge    # Only kvm-web-bridge .deb
```

Output: `dist/linux/*.deb`

To install on the build machine:
```bash
sudo apt install ./dist/linux/kvm-client_*.deb
sudo apt install ./dist/linux/kvm-web-bridge_*.deb
```

To inspect the package contents without installing:
```bash
dpkg --info dist/linux/kvm-client_*.deb       # Show package metadata
dpkg --contents dist/linux/kvm-client_*.deb   # List installed files
```

---

### macOS (.dmg)

Run from the project root in a bash terminal:

```bash
./build/package-macos.sh
```

Options:
```bash
./build/package-macos.sh --skip-build              # Use existing binaries
./build/package-macos.sh --version 1.2.3           # Override version string
./build/package-macos.sh --sign "Developer ID Application: Name (TEAMID)"
```

Output: `dist/macos/kvm-over-ip-<version>-macos.dmg`

To install, double-click the DMG and drag the `.app` to `/Applications`.

---

## Code Signing

Unsigned builds work for local development and testing but will trigger security
warnings on end-user machines.  For public distribution, each platform requires
a different signing approach.

### Windows Code Signing

Windows uses Authenticode code signing via a code signing certificate from a
trusted Certificate Authority (CA).

**Prerequisites:**
- A code signing certificate from a CA (e.g., DigiCert, Sectigo, GlobalSign).
- Microsoft's `signtool.exe` (included in the Windows SDK).

**Sign the binary before packaging:**
```powershell
# Sign the executable with a PFX certificate file.
signtool sign `
    /f "certificate.pfx" `
    /p "your-pfx-password" `
    /fd SHA256 `
    /tr "http://timestamp.digicert.com" `
    /td SHA256 `
    "src\target\release\kvm-master.exe"
```

**Sign the MSI after building:**
```powershell
signtool sign `
    /f "certificate.pfx" `
    /p "your-pfx-password" `
    /fd SHA256 `
    /tr "http://timestamp.digicert.com" `
    /td SHA256 `
    "dist\windows\kvm-master-0.1.0-x86_64.msi"
```

**For CI/CD** (store the certificate as a GitHub Secret):
```yaml
- name: Sign binary
  env:
    PFX_BASE64: ${{ secrets.WINDOWS_SIGNING_CERT_BASE64 }}
    PFX_PASSWORD: ${{ secrets.WINDOWS_SIGNING_CERT_PASSWORD }}
  run: |
    echo "$PFX_BASE64" | base64 --decode > cert.pfx
    signtool sign /f cert.pfx /p "$PFX_PASSWORD" /fd SHA256 ...
    rm cert.pfx
```

**Recommendation:** Timestamping (`/tr`) is essential.  Without a timestamp,
the signature becomes invalid when the certificate expires.

---

### Linux Package Signing

Linux .deb packages can be signed with a GPG key so apt repositories can
verify package integrity.

**Sign a .deb file:**
```bash
# Install the dpkg-sig tool.
sudo apt install dpkg-sig

# Sign the package with your GPG key.
dpkg-sig --sign builder dist/linux/kvm-client_*.deb
```

**For an apt repository**, the repository index file (Release) must be signed:
```bash
gpg --armor --detach-sign --output Release.gpg Release
```

See the [Debian Repository HOWTO](https://wiki.debian.org/DebianRepository) for
full instructions on setting up a signed apt repository.

---

### macOS Code Signing and Notarisation

macOS requires code signing with an Apple Developer ID and notarisation for
distribution outside the Mac App Store.  Without these, Gatekeeper will block
the application on first launch.

**Step 1 — Obtain a Developer ID certificate:**
1. Enrol in the [Apple Developer Program](https://developer.apple.com/programs/)
   (costs USD 99/year).
2. In Xcode → Settings → Accounts → Manage Certificates, create a
   "Developer ID Application" certificate.
3. Export the certificate as a `.p12` file.

**Step 2 — Sign the .app bundle:**
```bash
codesign \
    --deep \
    --force \
    --sign "Developer ID Application: Your Name (TEAMID)" \
    --entitlements build/macos/entitlements.plist \
    --options runtime \
    "src/target/release/bundle/osx/KVM-Over-IP Client.app"
```

Or use the `--sign` flag in the packaging script:
```bash
./build/package-macos.sh --sign "Developer ID Application: Your Name (TEAMID)"
```

**Step 3 — Sign the .dmg:**
```bash
codesign \
    --sign "Developer ID Application: Your Name (TEAMID)" \
    "dist/macos/kvm-over-ip-*.dmg"
```

**Step 4 — Notarise with Apple:**
```bash
xcrun notarytool submit "dist/macos/kvm-over-ip-*.dmg" \
    --apple-id your@email.com \
    --team-id YOUR_TEAM_ID \
    --wait
```

**Step 5 — Staple the notarisation ticket:**
```bash
xcrun stapler staple "dist/macos/kvm-over-ip-*.dmg"
```

After stapling, the DMG can be verified offline by Gatekeeper even without
an internet connection.

**For CI/CD** (store certificate in GitHub Secrets):
```yaml
- name: Import signing certificate
  env:
    CERTIFICATE_P12: ${{ secrets.MACOS_CERTIFICATE }}
    CERTIFICATE_PASSWORD: ${{ secrets.MACOS_CERTIFICATE_PWD }}
  run: |
    echo "$CERTIFICATE_P12" | base64 --decode > certificate.p12
    security create-keychain -p "" build.keychain
    security import certificate.p12 -k build.keychain -P "$CERTIFICATE_PASSWORD" -T /usr/bin/codesign
    security set-key-partition-list -S apple-tool:,apple: -s -k "" build.keychain
    security list-keychain -d user -s build.keychain
```

**Accessibility permission note:** Even with signing and notarisation, users must
manually grant Accessibility permission to kvm-client via:

> System Preferences → Privacy & Security → Accessibility

This is a macOS security requirement that cannot be bypassed.

---

## CI/CD Release Automation

Official releases are triggered by pushing a semantic version tag:

```bash
git tag v1.2.3
git push origin v1.2.3
```

The `.github/workflows/release.yml` workflow:

1. **build-windows** — builds binaries and MSI installers, uploads as artefacts.
2. **build-linux** — builds binaries and .deb packages, uploads as artefacts.
3. **build-macos** — builds binaries, .app bundle, and .dmg, uploads as artefacts.
4. **create-github-release** — downloads all artefacts, creates a GitHub Release,
   and attaches all archives and installer packages as release assets.

The workflow produces unsigned installers.  To add code signing to CI:

- **Windows:** store PFX certificate and password as GitHub Secrets, add a
  `signtool` step after building the binary and MSI.
- **macOS:** store the Developer ID certificate as a GitHub Secret, import
  it into a temporary keychain, pass `--sign` to codesign, then notarise.

See the comments in `.github/workflows/release.yml` for guidance.

---

## Troubleshooting

### Windows: "candle.exe not found"

WiX Toolset is not on the system PATH.  After installing WiX v3, add its
`bin` directory to PATH:

```
C:\Program Files (x86)\WiX Toolset v3.11\bin
```

Verify with: `candle.exe /?`

---

### Windows: Icon file not found (cargo-wix)

cargo-wix requires `wix\kvm-master.ico` (or `kvm-client.ico`) to be present.
Create a placeholder icon or temporarily remove the `<Icon>` and
`<Property Id="ARPPRODUCTICON">` elements from `main.wxs` to build without an icon.

---

### Linux: "libx11-dev not found"

```bash
sudo apt install libx11-dev libxtst-dev
```

On Fedora/RHEL: `sudo dnf install libX11-devel libXtst-devel`

---

### macOS: "App is damaged and can't be opened"

The app is unsigned and macOS Gatekeeper has quarantined it.  For local testing:

```bash
xattr -dr com.apple.quarantine "/Applications/KVM-Over-IP Client.app"
```

For distribution, sign and notarise the app as described in the [Code Signing](#code-signing) section.

---

### macOS: cargo-bundle fails with "No such file or directory (os error 2)"

Ensure you are running `cargo bundle` from the `src/` directory (where
`Cargo.toml` is), not from the project root.  The `package-macos.sh` script
handles this automatically.

---

### CI: "No .msi files found in src/target/wix"

cargo-wix requires icon files at `wix\kvm-master.ico` (and `kvm-client.ico`).
The CI `build-windows` job will fail if these files are absent.  Add placeholder
`.ico` files to the repository, or modify `main.wxs` to remove the `<Icon>` element.

---

*This document was last updated for KVM-Over-IP version 0.1.0.*
