# Session Notes

## Project
- App: **Cat-rocket-converter** (display name), `pixshrink` (internal)
- Stack: Tauri v2 + Rust backend + Svelte frontend
- Canvas rule: window and canvas stay `428x318`, non-resizable
- Main background asset: `page1_background.png`
- **GitHub repo:** https://github.com/dixinode/cat-rocket-converter
- **Releases:** https://github.com/dixinode/cat-rocket-converter/releases

## Current State
- Main wizard flow works end-to-end
- App hides to tray after successful conversion, does not quit
- App name in `/Applications` is `Cat-rocket-converter`
- Rust tests pass, frontend build passes
- GitHub release `v0.1.1` contains the current fixed `.dmg`
- Production startup crash on another Mac was fixed by embedding the tray icon into the binary
- Finder "Open With" shows app for jpg/jpeg/png/heic/heif
- Latest reported state from user: "Everything works as expected"

## Important Architecture
- `src/App.svelte`: root canvas, page switching, global background rendering + Finder file intake
- `src/stores/wizard.js`: single wizard state store
- `src/pages/Page1Drop.svelte`: file picking and drag/drop
- `src/pages/Page6Save.svelte`: save dialog, waiting state, conversion trigger
- `src/lib/tauri.js`: frontend wrappers for Tauri commands
- `src-tauri/src/commands.rs`: IPC commands, `PendingOpenFiles` state, conversion orchestration
- `src-tauri/src/converter.rs`: image load/resize/save with HEIC orientation handling (sequential, not parallel)
- `src-tauri/src/file_utils.rs`: rename pattern generation
- `src-tauri/src/main.rs`: tray behavior, hide-on-close, `RunEvent::Opened` handler
- `src-tauri/tauri.conf.json`: app config, product name, file associations, bundle settings

## Key Features

### 1. Waiting Screen During Conversion
- Asset: `wait_for_conversion.png` in project root
- After folder selection + before conversion: normal UI hidden, wait image shown
- Both backgrounds pre-mounted in DOM, CSS opacity toggle (no src-swap bug)
- Pending files handled in `PendingOpenFiles` backend state (Mutex)

### 2. Completion Sound
- macOS `Glass.aiff` via `afplay /System/Library/Sounds/Glass.aiff`
- Tauri command `play_system_sound`
- Plays before `conversion_completed` event, window hides after

### 3. Deterministic Save Order
- Conversion loop is sequential `iter()`, not parallel `par_iter()`
- Files numbered and saved in order: `001, 002, 003...`

### 4. Mixed Portrait / Landscape Batch Safety
- With `Lock aspect ratio` enabled, batch resize uses per-file percentages
- Portrait and landscape photos in one batch keep their own geometry
- The first file no longer forces later files into the same aspect ratio

### 5. Finder "Open With" Integration
- File associations: `jpg`, `jpeg`, `png`, `heic`, `heif` in `tauri.conf.json`
- Backend: `tauri::RunEvent::Opened { urls }` in `main.rs`
- Frontend: listener `finder-open-files` + `take_pending_open_files()` in `App.svelte`
- Works for cold launch AND already-running-in-tray

### 6. HEIC / iPhone Behavior
- iPhone portrait `HEIC` files convert with correct orientation
- HEIC decode goes through macOS `sips`
- If decoded bitmap dimensions disagree with Finder/Spotlight display dimensions, the bitmap is rotated once to match the display geometry

### 7. Production Build
- Command: `npm run tauri build` (NOT `cargo tauri build`)
- Output:
  - `.app` → `src-tauri/target/release/bundle/macos/Cat-rocket-converter.app`
  - `.dmg` → `src-tauri/target/release/bundle/dmg/Cat-rocket-converter_0.1.1_aarch64.dmg`
- Important release fix: after `npm run tauri build`, run manual re-sign on the built app:
  - `codesign --force --deep --sign - "src-tauri/target/release/bundle/macos/Cat-rocket-converter.app"`
- Then rebuild the dmg manually from the re-signed app, because the raw Tauri-produced bundle had a broken ad-hoc signature (`Info.plist not bound`, `Sealed Resources=none`)
- The broken bundle caused macOS to show: `"...app" is damaged and can't be opened`
- Additional release fix in `v0.1.1`: tray icon is embedded into the binary, so the app no longer crashes on another Mac with `No such file or directory (os error 2)` during startup
- Current release asset: `Cat-rocket-converter_0.1.1_aarch64.dmg`

## Waiting-State Details
- Store flag: `showWaitingBackground` (turned on after folder selection, reset on error/reset)
- `saving` flag still controls conversion lifecycle independently
- `Page6Save.svelte` waits for `tick()` + 2x `requestAnimationFrame` before conversion starts

## UI Notes
- Page 4 no longer shows live estimated output size; it was removed to avoid resize-page stutter
- Page 6 action button label is `SAVE TO FOLDER`
- Page 1 drag-and-drop uses Tauri-native drop events, not browser `dataTransfer` paths
- Progress dots sit on a theme-aware pill background for readability

## Behavior That Must Stay Intact
- Window: 428x318, non-resizable, always that size
- Tray: hide-on-close, quit from menu, left-click to show
- On success: play Glass.aiff, reset state, hide window
- On cancel folder picker: no conversion, no waiting screen, stay open
- Save order: sequential, not parallel
- Do not rename app back to ALL CAPS

## Git
- Repo initialized, remote `origin` = `https://github.com/dixinode/cat-rocket-converter`
- Branch: `main`
- Commit msg style: short description in English
- `.gitignore` excludes: `node_modules/`, `dist/`, `src-tauri/target/`, `.DS_Store`, `*.log`

## Build Cleanup
- `src-tauri/target/` can be deleted any time — regenerates on next build
- `dist/` can also be deleted any time — regenerates on next frontend build
- Safe delete: `rm -rf src-tauri/target/ dist/`
- After cleanup: project was reduced back to roughly `150M`-`170M`
- We cleaned build artifacts more than once during this session; this is safe and expected

## Release Caveat
- DMG is buildable/distributable but not Apple-signed/notarized
- Fresh fixed app bundle verifies with `codesign`, but Gatekeeper still rejects it because it is not notarized
- Friends may need: Right Click → Open on first launch, or quarantine removal if Gatekeeper is too aggressive
- For seamless install: need Developer ID signing + notarization

## Commands Quick Reference
```bash
npm install              # install frontend deps
npm run dev              # dev server (Tauri hot reload)
npm run tauri build      # production .app + .dmg
cargo test               # Rust unit tests (in src-tauri/)
npm run build            # Vite frontend build only
```

## Good First Read Next Session
1. `SESSION_NOTES.md`
2. `CLAUDE.md`
3. `README.md`
4. `src/App.svelte`
5. `src-tauri/src/main.rs`
6. `src-tauri/src/commands.rs`

## Suggested Prompt Next Time
"Read `SESSION_NOTES.md` and continue from the current Cat-rocket-converter state without changing working behavior unless requested."
