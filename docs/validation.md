# Validation record

Validated in the Linux cloud workspace on 2026-10-28 with Rust 1.99.1 and Node.js 24.19.0. Node.js 22+ is the supported frontend baseline; CI uses 22.

- Core tests: cache bounds/eviction/deduplication, partial message edit/deletion, a real local WebSocket handshake with IDENTIFY/READY/sequence heartbeat/ACK/reconnect, RESUME and missing-ACK reconnection, and REST input validation. No Discord service or credential is used by these tests.
- Frontend: strict TypeScript check and Vite production build passed; theme validation tests passed (built-ins and rejection of malformed/injected styles).
- Rust core Clippy with warnings denied passed; workspace formatting applied and checked.
- Starter desktop icon generated with Tauri's icon tool.
- Native desktop check attempted but blocked at `glib-sys`: this Linux host lacks GTK/WebKit development libraries. The host has no sudo and package installation fails on system-directory permissions. This does not prove the Tauri adapter compiles or runs on any OS.
- Windows MSI/NSIS and macOS universal app/DMG builds are configured in GitHub Actions but were not run locally. CI results must be reviewed independently after push. Signing, notarization, Homebrew distribution, performance comparisons and live Discord messaging are unverified.
- No bot credential was supplied; no live Discord login, message read or message send was attempted.

The Rust core and frontend build are validated. The complete native application remains a scaffold awaiting native-host validation and the production hardening described in the architecture guide.
