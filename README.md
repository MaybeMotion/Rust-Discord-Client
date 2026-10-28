# Rust Discord Starter

A Tauri 2 desktop scaffold with a Rust Discord transport/cache and a small TypeScript UI. Targets Windows and macOS; the core is platform-independent. This is a starting implementation, not a complete replacement for Discord.

**Account model:** Discord does not offer a supported third-party client API for regular user accounts. Automating a user account (a self-bot) violates Discord's rules and risks account termination. This implementation uses **bot authentication only**. OAuth2 sign-in does not grant a user Gateway session or general user messaging. You see and send only what your bot can access; messages are sent as the bot. Never paste a personal Discord token into this app.

## Quick start

Install [Rust stable](https://rustup.rs/), Node.js 22+, and [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/):

- Windows: Visual Studio Build Tools with Desktop development with C++, Windows SDK, and WebView2 Runtime.
- macOS: Xcode Command Line Tools (`xcode-select --install`). WKWebView is supplied by macOS.
- Linux (development only): compiler/build tools, pkg-config, WebKitGTK 4.1 development files, appindicator, librsvg and patchelf. See the Tauri guide for your distribution.

```sh
git clone https://github.com/MaybeMotion/Rust-Discord-Client.git
cd Rust-Discord-Client
cargo test -p discord-core --locked
cd apps/desktop
npm ci
npm test
npm run build
npm run tauri dev
```

The native app starts in a read-only offline demo when `DISCORD_BOT_TOKEN` is absent. `npm run dev` alone serves the frontend but cannot execute native commands. A browser-only session is not a functional Discord client.

For real messaging, create an application and bot in the Discord Developer Portal. Enable **Message Content Intent** (approval may be required for verified bots); this scaffold requests Guilds, Guild Messages, Direct Messages and Message Content intents. Invite the bot to a test guild using the `bot` OAuth2 scope with View Channels, Read Message History and Send Messages permissions. Enable only the permissions you need; administrator is unnecessary.

Supply `DISCORD_BOT_TOKEN` to the **native backend process** using your local secret manager or a temporary shell environment, then run `npm run tauri dev`. The app does not load `.env` files. Restart the app after changing credentials. Do not put tokens in `VITE_*` variables, frontend storage, Git, screenshots, command history, or distributed builds. This scaffold intentionally has no token-entry UI. The credential is not exposed by IPC. A distributable multi-user product needs OS-keychain-backed bot credential entry and lifecycle controls.

Select a text channel, load recent history and send plain text. Mentions are disabled for outgoing messages by default. Theme selection and validated JSON import work offline. `docs/example-theme.json` is an importable example.

## Layout

```text
Cargo.toml                       workspace, optimized release profile
crates/discord-core/src/
  model.rs                       string snowflakes and wire models
  gateway.rs                     HELLO, IDENTIFY/RESUME, heartbeat, ACK, reconnect
  rest.rs                        authenticated v10 REST, bounded 429 retries
  state.rs                       guild/channel state and bounded message cache
apps/desktop/
  src/main.ts                    channel navigation, history, composer, native IPC
  src/theme.ts                   versioned JSON validation → CSS variables
  src/style.css                  dark/light/neon glass presentation
  src-tauri/src/main.rs          native command facade and bounded event consumer
  src-tauri/tauri.conf.json      native window, CSP and packaging
  src-tauri/capabilities/        minimum default core capability
.github/workflows/ci.yml         core/frontend checks and native desktop builds
```

Read [the architectural blueprint](docs/architecture.md), [development roadmap](docs/roadmap.md), [packaging guide](docs/packaging.md), and [validation record](docs/validation.md).

## Checks

```sh
cargo test -p discord-core --locked
cargo clippy -p discord-core --locked -- -D warnings
cargo fmt --all -- --check
cd apps/desktop
npm ci
npm test
npm run build
npm run tauri build
```

Core tests use a local fake WebSocket peer, not Discord credentials. Real Discord and platform packaging checks require the appropriate host and credentials; consult the validation record for what was actually run. No memory or CPU advantage over the official client is claimed until measured on identical workloads.

## Scope and next steps

Implemented: plain-text rendering and sending, recent history, selected Gateway state events, heartbeat/resume/reconnect skeleton, bounded in-memory cache, JSON themes, native packaging configuration. Missing: normal-user accounts (unsupported), voice/video, attachments, rich embeds/Markdown, reactions, threads, search, unread/read state, full permission filtering, presence, typing, bulk deletion, offline disk cache, robust per-route rate buckets, production telemetry, signing and updater infrastructure. Never expose the IPC webview to remote pages. Treat all message content as untrusted; this UI uses `textContent`.

Licensed under MIT. Not affiliated with Discord.
