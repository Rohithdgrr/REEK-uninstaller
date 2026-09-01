# Auto-Update (Tauri Updater)

REEK Uninstaller ships as a Tauri v2 desktop app with **built-in auto-update** via `tauri-plugin-updater`. Updates are published to GitHub Releases; the app checks `latest.json` on launch and offers a one-click update.

## How it works

```
Tag vX.Y.Z pushed → GitHub Actions (release.yml) → cargo build + tauri build
  └─ Tauri bundles: .msi/.exe (Windows), .dmg/.app (macOS), .AppImage/.deb (Linux)
  └─ Updater artifacts: latest.json + .sig files (signed with private key)
         ↓ uploaded to GitHub Release
User launches REEK → plugin-updater fetches https://github.com/Rohithdgrr/REEK-uninstaller/releases/latest/download/latest.json
  └─ If version > current, shows “Update available” → download + install + restart
```

## Configuration

`src-tauri/tauri.conf.json`:

```json
{
  "bundle": { "createUpdaterArtifacts": true },
  "plugins": {
    "updater": {
      "pubkey": "<your-generated-public-key>",
      "endpoints": ["https://github.com/Rohithdgrr/REEK-uninstaller/releases/latest/download/latest.json"],
      "windows": { "installMode": "passive" }
    }
  }
}
```

* `pubkey` — **public key** from `npm run tauri signer generate -w ~/.tauri/myapp.key` (or `cargo tauri signer generate`). Keep `myapp.key` private, commit only the pubkey.
* `endpoints` — GitHub Releases URL serving `latest.json`. Tauri updater polls this on `check()`.

Rust side registers the plugin:

```rust
// src-tauri/src/lib.rs
tauri::Builder::default()
  .plugin(tauri_plugin_updater::Builder::new().build())
  .plugin(tauri_plugin_opener::init())
  // ...
```

Frontend can trigger a check manually:

```ts
import { check } from "@tauri-apps/plugin-updater";
const update = await check();
if (update) await update.downloadAndInstall();
```

Currently the app checks on startup (you can wire `check()` in `App.tsx` `useEffect`).

## Generate signing keys locally

```bash
npm run tauri signer generate -- -w ~/.tauri/reek.key
# prints: pubkey = "dW50cnVzdGVkIGNvbW1lbnQ6..."
# Save the private key securely, never commit it.

# Optionally set pubkey in tauri.conf.json
# Add secrets to GitHub repo:
#  Settings → Secrets → Actions
#  TAURI_SIGNING_PRIVATE_KEY = contents of ~/.tauri/reek.key
#  TAURI_SIGNING_PRIVATE_KEY_PASSWORD = password you entered (or empty)
```

> **Placeholder:** the repo ships with `dW50cnVzdGVkIGNvbW1lbnQ6IHJlcGxhY2UgdGhpcyB3aXRoIHlvdXIgZ2VuZXJhdGVkIHB1YmtleSB2aWEgYG5wbSBydW4gdGF1cmkgc2lnbmVyIGdlbmVyYXRlYC4gU2VlIGRvY3MvQ0lfQ0QuIG1k` — replace it after generating.

## CI/CD pipeline

* **CI** (`.github/workflows/ci.yml`): on push/PR to `main`/`develop` — fmt, clippy, test (ubuntu/win/macos), MSRV, audit/deny, frontend `npm ci && npm run build`, Tauri config check. No signing needed.
* **Release** (`.github/workflows/release.yml`): on tag `v*.*.*`
  - `build` job: `cargo build --release` per target + SBOM + attestations.
  - `tauri` job (matrix ubuntu/win/macos): `npm ci` + `tauri-action` builds bundles + updater artifacts (`latest.json` + `.sig`) using `TAURI_SIGNING_PRIVATE_KEY` from secrets. Artifacts uploaded as `tauri-bundle-<os>`.
  - `release` job: downloads all artifacts, generates `RELEASE_NOTES.md` from `CHANGELOG.md`, creates GitHub Release with `softprops/action-gh-release`, attaching both cargo tarballs and Tauri bundles + `latest.json`. The `latest.json` URL is what the app polls.

See `docs/CI_CD.md` for job matrix and `docs/RELEASING.md` for tagging.

## Testing locally

```bash
# Dev: updater disabled (no signing)
npm run tauri dev

# Build a signed release locally (requires key):
export TAURI_SIGNING_PRIVATE_KEY_PATH=~/.tauri/reek.key
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="your-password"
npm run tauri build
# Check src-tauri/target/release/bundle/ — should contain latest.json
```

Or trigger the release workflow manually with a test tag:

```bash
git tag v0.1.1-test
git push origin v0.1.1-test
# Watch Actions → Release → artifacts → latest.json
```

## Security

* Private key never leaves GitHub Secrets; public key is in `tauri.conf.json`.
* Every release artifact is checksummed (`SHA256SUMS`) and attested via OIDC (`actions/attest-build-provenance`).
* Updater verifies `.sig` against `pubkey` before installing — tampered binaries are rejected.
* See `docs/SECURITY.md` for layered controls.

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| App says “update not available” but release exists | Ensure `latest.json` is at the endpoint URL, pubkey matches private key used to sign, version in `tauri.conf.json` < tag version. |
| Windows SmartScreen warning | Sign the `.exe`/`.msi` with an EV cert (separate from Tauri updater signing). Tauri signing only covers updater integrity. |
| `TAURI_SIGNING_PRIVATE_KEY` not found in CI | Add it in repo Settings → Secrets → Actions. Use the full file content, newlines preserved. |

## References

* Tauri Updater docs: https://tauri.app/plugin/updater/
* `tauri-action`: https://github.com/tauri-apps/tauri-action
* Signing: `npm run tauri signer -- --help`
