# pixshrink

## Architecture
- `src-tauri/src/converter.rs` - all image processing (load/resize/save/estimate)
- `src-tauri/src/commands.rs` - Tauri IPC layer (called from frontend via invoke())
- `src-tauri/src/file_utils.rs` - rename pattern logic
- `src/stores/wizard.js` - single global state store for wizard flow
- `src/App.svelte` - root component, always renders blurred background + page router

## Canvas rule
Window is ALWAYS 428x318 px, non-resizable.
`page1_background.png` is always the background on every page.
Never change window dimensions.

## Key commands
`cargo tauri dev`         - dev server with hot reload
`cargo tauri build`       - production .dmg / .exe
`cargo test`              - Rust unit tests
`npm test`                - frontend E2E tests scaffold

## IPC contract
All Tauri commands are in `commands.rs`.
Frontend calls them via wrappers in `src/lib/tauri.js`.
Types are documented in each command doc-comment.
