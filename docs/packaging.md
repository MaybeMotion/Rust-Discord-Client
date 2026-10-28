# Native packaging

Run from `apps/desktop` after `npm ci`. Native targets require native build hosts; Linux cannot prove Windows/macOS compatibility. A generated starter icon is included. Icons can be replaced with your own artwork using `npm run tauri icon path/to/icon.png` (a square high-resolution PNG); update `bundle.icon` in `src-tauri/tauri.conf.json` to the generated PNG/ICO/ICNS paths before release.

## Windows (x64)

Install Visual Studio C++ Build Tools, Windows SDK, WebView2 and the Rust MSVC toolchain. Then:

```powershell
rustup target add x86_64-pc-windows-msvc
npm ci
npm run build
npm run tauri build -- --target x86_64-pc-windows-msvc --bundles nsis,msi
```

Outputs live under the workspace root `target/x86_64-pc-windows-msvc/release/bundle/`: NSIS produces an installer EXE and WiX produces MSI. Configure WebView2 installation policy for offline deployments; Tauri's default bootstrapper may download it. Validate both installers on a clean Windows machine. Use an Authenticode certificate and the supported Tauri Windows signing configuration or a secured signing service for production. Do not commit certificates/passwords. CI currently produces unsigned artifacts; SmartScreen trust is not established.

## macOS universal application and DMG

Install Xcode Command Line Tools on a macOS build machine:

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
npm ci
npm run build
npm run tauri build -- --target universal-apple-darwin --bundles app,dmg
lipo -info ../../target/universal-apple-darwin/release/bundle/macos/*.app/Contents/MacOS/discord-desktop
```

The output root is `target/universal-apple-darwin/release/bundle/` at the repository root. Tauri combines the Intel and Apple Silicon builds; every future native dependency must support both architectures. Test the actual bundle on both CPU families. Set a supported minimum macOS version and test it before release.

Unsigned artifacts are for development. For distribution outside the App Store, use a Developer ID Application certificate, hardened runtime, appropriate entitlements, notarization and stapling. Configure supported Tauri signing/notarization environment variables in your CI secret store (`APPLE_SIGNING_IDENTITY`, certificate/password bindings and notarization credentials or App Store Connect API-key flow as appropriate). Consult the current [Tauri macOS signing guide](https://v2.tauri.app/distribute/sign/macos/) rather than embedding secrets in scripts. Verify with `codesign --verify --deep --strict`, `spctl --assess --type execute` and `xcrun stapler validate` on the shipped artifact. Signing is not performed by this starter.

## Homebrew cask

Upload the signed and notarized universal DMG to a versioned GitHub Release. Compute its SHA-256 (`shasum -a 256 ...dmg`) and create a cask in your own tap:

```ruby
cask "rust-discord-starter" do
  version "0.1.0"
  sha256 "REPLACE_WITH_ACTUAL_RELEASE_DMG_SHA256"
  url "https://github.com/MaybeMotion/Rust-Discord-Client/releases/download/v#{version}/Rust.Discord.Starter_#{version}_universal.dmg"
  name "Rust Discord Starter"
  desc "Rust-powered bot messaging desktop"
  homepage "https://github.com/MaybeMotion/Rust-Discord-Client"
  app "Rust Discord Starter.app"
end
```

Use the actual uploaded filename; the URL is illustrative and no release exists merely because a build ran. Test the cask with `brew install --cask <tap>/rust-discord-starter` and run Homebrew audit before publishing. Users install through your tap; acceptance into Homebrew's official cask repository is a separate review. No release, signing identity, Homebrew tap or updater endpoint is created by this project.
