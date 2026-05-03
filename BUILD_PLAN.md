# PixShrink — AI Agent Build Plan

> macOS image converter with system tray, wizard UI, and fixed 428×318 canvas

---

## Core Concept & Canvas Rule

**The app window is ALWAYS 428×318 px — fixed, non-resizable.**  
Every page of the wizard renders inside this canvas. The background image
`page1_background.png` (428×318 px, located in the project root) is always
present on every page, slightly blurred, giving the app a consistent branded
look throughout all steps.

```
Window size:  428 × 318 px (fixed, resizable = false)
Canvas:       428 × 318 px (100% width/height, no scroll)
Background:   page1_background.png — always visible, CSS filter: blur(5px) brightness(0.45)
Overlay:      Each page renders its UI on top of the blurred background
```

This means every page is a **layered UI**: blurred background image at the
bottom, a semi-transparent dark overlay for readability, and the page content
on top. The background never disappears — it is the visual identity of the app.

---

## Technology Stack

| Layer | Choice | Reason |
|---|---|---|
| App shell | **Tauri v2** (Rust) | Native tray, tiny binary (~8 MB), cross-platform |
| Image processing | **Rust `image` crate** + `mozjpeg` + `webp` | Fast, zero-copy, all formats |
| HEIC decoding | **`libheif-rs`** | Best HEIC support on macOS |
| Parallelism | **`rayon`** | Par-iter over file batches |
| Frontend | **Svelte 5** (single .svelte bundle) | Minimal JS, reactive, no overhead |
| IPC | Tauri Commands (`#[tauri::command]`) | Type-safe Rust↔JS bridge |
| Resize algorithms | `image::imageops::FilterType` | Nearest/Bilinear/CatmullRom/Lanczos3/Gaussian |

**Why not Electron?** Binary would be 150 MB+. Tauri gives native performance
at 5–15 MB and ports to Windows 11 with zero code changes — just change the
build target.

---

## Project Structure

```
pixshrink/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs           # App entry, tray setup, window lifecycle
│   │   ├── converter.rs      # Image load / resize / encode / save
│   │   ├── commands.rs       # Tauri IPC commands (called from frontend)
│   │   └── file_utils.rs     # Rename pattern logic
│   ├── icons/
│   │   ├── tray.png          # 22×22 tray icon
│   │   └── icon.png          # App icon
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/
│   ├── App.svelte            # Root: page router + background layer
│   ├── pages/
│   │   ├── Page1Drop.svelte  # Drop zone + OPEN button
│   │   ├── Page2Format.svelte
│   │   ├── Page3Quality.svelte
│   │   ├── Page4Resize.svelte
│   │   ├── Page5Rename.svelte
│   │   └── Page6Save.svelte
│   ├── stores/
│   │   └── wizard.js         # Global reactive wizard state
│   └── lib/
│       └── tauri.js          # Typed wrappers around invoke()
├── page1_background.png      # 428×318 brand background (project root)
└── package.json
```

---

## AGENT 1 — Developer

### tauri.conf.json

```json
{
  "windows": [{
    "title": "PixShrink",
    "width": 428,
    "height": 318,
    "resizable": false,
    "decorations": true,
    "transparent": false,
    "visible": false
  }],
  "systemTray": {
    "iconPath": "icons/tray.png",
    "iconAsTemplate": true
  }
}
```

`visible: false` — window starts hidden; shown only when user clicks tray icon.

---

### main.rs responsibilities

1. Build `SystemTray` with menu items: `Show` and `Quit`
2. On tray left-click or "Show" menu item → `window.show()` + `window.center()`
3. Override window close event (`on_window_event`) → instead of quitting,
   call `window.hide()` so the app stays alive in tray
4. Expose all `commands.rs` functions via `.invoke_handler(tauri::generate_handler![...])`
5. After successful save (emitted event from frontend) → `window.hide()`
6. On "Quit" menu item → `std::process::exit(0)`

---

### converter.rs — Public API

```rust
/// Load any supported image (HEIC, PNG, JPG) into DynamicImage
pub fn load_image(path: &str) -> Result<DynamicImage, String>

/// Resize image with chosen algorithm
pub fn resize_image(
    img: DynamicImage,
    width: u32,
    height: u32,
    algorithm: ResizeAlgorithm,
) -> DynamicImage

/// Encode and save to disk; returns actual file size in bytes
pub fn save_image(
    img: &DynamicImage,
    path: &str,
    format: OutputFormat,
    quality: u8,         // 1–100; ignored for PNG, used for JPEG and WebP
) -> Result<u64, String>

/// Estimate output file size without writing to disk
/// Used for real-time preview on Page 4
pub fn estimate_file_size(
    img: &DynamicImage,
    format: OutputFormat,
    quality: u8,
    target_width: u32,
    target_height: u32,
) -> u64

pub enum ResizeAlgorithm {
    Nearest,      // fastest, pixelated
    Bilinear,     // fast, smooth
    CatmullRom,   // good balance
    Lanczos3,     // best quality, slower
    Gaussian,     // soft/blurry
}

pub enum OutputFormat { Jpeg, Png, Webp }
```

**Implementation notes:**
- HEIC: decode via `libheif_rs::HeifContext`, convert to `image::DynamicImage`
- JPEG encoding: use `mozjpeg` for quality control (not the default encoder)
- WebP: use `webp` crate, `Encoder::from_image(&img).encode(quality as f32)`
- PNG: always lossless, ignore quality parameter
- `estimate_file_size`: encode to an in-memory `Vec<u8>` buffer, return its length.
  Do NOT write to disk.

---

### file_utils.rs — Rename Patterns

```rust
pub enum RenamePattern {
    /// IMG_20250503_143022_001.jpg
    DateTimePrefix { prefix: String },

    /// original_name_suffix.ext
    OriginalSuffix { suffix: String },

    /// base_001.ext, base_002.ext, ...
    Sequential { base: String, start: u32 },

    /// No rename — keep original filename
    KeepOriginal,
}

/// Apply pattern to a list of source file paths, return new filenames (no dirs)
pub fn apply_rename_pattern(
    files: &[String],
    pattern: &RenamePattern,
    output_ext: &str,
) -> Vec<String>
```

DateTimePrefix format: `{prefix}{YYYYMMDD}_{HHMMSS}_{NNN}.{ext}`  
where NNN is zero-padded index within the batch.

---

### commands.rs — Tauri IPC

All functions are `#[tauri::command]` and called from frontend via `invoke()`.

```rust
// Returns width, height, file size in bytes, detected format string
get_image_info(path: String) -> Result<ImageInfo, String>

// Convert all files with given params; returns list of saved file paths
convert_images(params: ConvertParams) -> Result<Vec<String>, String>

// Estimate output size for real-time Page 4 preview (debounced on frontend)
estimate_output(params: EstimateParams) -> SizeEstimate

// Open macOS Save Panel pointing to the given default directory
// Returns chosen directory path or None if cancelled
open_save_dialog(default_dir: String) -> Option<String>

// Return the directory of the first dropped file (for default save location)
get_parent_dir(file_path: String) -> String
```

`ConvertParams` struct fields:
- `files: Vec<String>` — source paths
- `output_dir: String`
- `format: String` — "jpeg" | "png" | "webp"
- `quality: u8`
- `resize_w: u32`, `resize_h: u32`
- `algorithm: String`
- `rename_pattern: RenamePatternDto` (serialized enum)

---

### wizard.js — Global State Store (Svelte)

```javascript
import { writable, derived } from 'svelte/store';

export const wizard = writable({
  // Page 1
  files: [],              // array of file path strings

  // Page 2
  outputFormat: null,     // 'jpeg' | 'png' | 'webp'

  // Page 3 (JPEG only)
  quality: 100,

  // Page 4
  resize: {
    pct_w: 100,
    pct_h: 100,
    px_w: 0,
    px_h: 0,
    lockAspect: true,
    algorithm: 'lanczos3',
  },
  originalDimensions: { w: 0, h: 0 },   // set after Page 1 load

  // Page 5
  rename: {
    pattern: 'datetime',    // 'datetime' | 'suffix' | 'sequential' | 'keep'
    prefix: 'IMG_',
    suffix: '_compressed',
    seqBase: 'export_',
    seqStart: 1,
  },

  // Navigation
  currentPage: 1,
  saving: false,
});
```

---

### App.svelte — Background Layer Architecture

Every page is rendered inside a fixed 428×318 container. The structure:

```html
<!-- Always present, every page -->
<div class="canvas">
  <img class="bg" src="/page1_background.png" alt="" />
  <!-- bg has: position absolute, 100% w/h, object-fit cover,
       filter: blur(5px) brightness(0.45) -->

  <div class="overlay">
    <!-- semi-transparent dark overlay for text readability -->
    <!-- background: rgba(10,10,10,0.55) -->
  </div>

  <div class="content">
    <!-- current wizard page renders here -->
    {#if $wizard.currentPage === 1}<Page1Drop />
    {:else if $wizard.currentPage === 2}<Page2Format />
    <!-- etc -->
    {/if}
  </div>
</div>
```

CSS for `.canvas`: `width: 428px; height: 318px; overflow: hidden; position: relative;`

---

### Page-by-Page Frontend Spec

#### Page 1 — Drop Zone
- Center text: `"drag and drop your pictures"` — 15px, color `#d0d0d0`, font-weight 300
- Button `OPEN` — bottom center, `80×28px`, border-radius `6px`, outlined white style
  - On click: use Tauri `dialog.open({ multiple: true, filters: [HEIC,PNG,JPG] })`
- Drag-over state: white dashed border `2px` around entire canvas + `#0a84ff` tint
- On files dropped/selected: store in `wizard.files`, call `get_image_info` on first
  file to populate `wizard.originalDimensions`, then advance to Page 2

#### Page 2 — Format Selection
- Three buttons centered vertically with `gap: 12px`:
  `JPEG`, `PNG`, `WEBP`
- Button style: `200×44px`, `border-radius: 10px`, outline by default
- Selected: filled `#0a84ff`, white text
- On select: store in `wizard.outputFormat`
- NEXT button enabled only after selection

#### Page 3 — JPEG Quality
- **Skip this page automatically if format is PNG or WEBP** (go straight from Page 2 → Page 4)
- Title: `"JPEG Quality"` — small, gray, uppercase
- Slider: range 1–100, centered, 300px wide, default 100
- Below slider: large number showing current value (`36px bold white`)
- Below that: small label `"Higher = larger file, better quality"`

#### Page 4 — Resize & Algorithm
- Layout: two columns, centered block, `gap: 24px`
  ```
  Column 1 (%)     Column 2 (px)
  [ %W  input ]    [ px W  input ]
  [ %H  input ]    [ px H  input ]
  ```
- Input size: `72×32px`, centered text, border-radius 6px
- Default: all 100%, px values = original dimensions from `originalDimensions`
- When pct changes → recalculate px. When px changes → recalculate pct.
- `lockAspect` checkbox below: if checked, changing W auto-updates H proportionally
- Dropdown — resize algorithm:
  - `Nearest (fastest)` | `Bilinear` | `CatmullRom` | `Lanczos3 (best quality)` | `Gaussian`
  - Default: `Lanczos3`
- Footer info (gray, 11px), updated in real-time (debounced 300ms via `estimate_output`):
  ```
  → 1920 × 1080 px
  → ~2.4 MB
  ```
  Format sizes dynamically as `B / KB / MB` depending on magnitude.

#### Page 5 — Rename
- Title: `"Rename files"` — small, gray, uppercase
- Four radio options:
  1. `Date + Time prefix` → shows prefix text input (default `IMG_`)
     → preview: `IMG_20250503_143022_001.jpg`
  2. `Original name + suffix` → shows suffix text input (default `_compressed`)
     → preview: `photo_compressed.jpg`
  3. `Sequential` → shows base input + start number (default `export_`, `1`)
     → preview: `export_001.jpg`
  4. `Keep original names`
- Live preview: show first 3 filenames from the batch using the chosen pattern
- Default selected: option 1 (DateTime prefix)

#### Page 6 — Save
- Center content:
  - Large `✓` icon, `32px`, color `#0a84ff`
  - Text: `"Ready — N files"` where N = file count
  - Small summary: format, quality (if JPEG), dimensions
- Button `SAVE` — `160×44px`, filled `#0a84ff`, border-radius `10px`
  - On click: call `open_save_dialog(parentDirOfFirstFile)`
  - If dialog confirmed: call `convert_images(params)`
  - Show inline spinner/progress during conversion
  - On success: emit close event → window hides → wizard resets to Page 1

---

### Navigation Bar (all pages)

- Fixed to bottom of canvas, `height: 44px`
- Left: `BACK` button (hidden on Page 1) — text style, `#888`, `12px uppercase`
- Center: 6 dots progress indicator — current dot `#0a84ff`, rest `#444`
- Right: `NEXT` button (hidden on Page 6) — filled `#0a84ff`, `80×28px`,
  border-radius `6px`, disabled + `opacity: 0.4` if page conditions not met

---

## AGENT 2 — Tester

### Rust Unit Tests (`#[cfg(test)]` in each module)

**converter.rs:**

| Test | What to verify |
|---|---|
| `test_load_jpeg` | Load valid JPG, check width/height > 0 |
| `test_load_png` | Load valid PNG, dimensions correct |
| `test_load_heic` | Load HEIC, does not panic, produces DynamicImage |
| `test_resize_50pct` | 1000×500 image → resize to 50% → 500×250 |
| `test_resize_aspect_exact` | Lanczos3 result dimensions match requested |
| `test_save_jpeg_q100` | File exists on disk, size > 0 |
| `test_save_jpeg_q10_smaller` | q=10 file size < q=100 by at least 50% |
| `test_save_png_lossless` | Encode PNG, reload, compare pixel at (0,0) — must match |
| `test_save_webp` | File exists, size > 0 |
| `test_estimate_accuracy` | estimate_file_size within ±25% of actual saved size |

**file_utils.rs:**

| Test | What to verify |
|---|---|
| `test_datetime_pattern` | Output matches regex `IMG_\d{8}_\d{6}_\d{3}\.jpg` |
| `test_datetime_batch` | 5 files → 5 unique names (no collisions) |
| `test_suffix_pattern` | `photo.jpg` → `photo_compressed.jpg` |
| `test_sequential_pattern` | 3 files → `export_001.jpg`, `export_002.jpg`, `export_003.jpg` |
| `test_keep_original` | Output names == input names |
| `test_extension_replaced` | Input `.png`, outputExt `.jpg` → output ends with `.jpg` |

### Integration Tests

1. **Full JPEG pipeline**: drop PNG → format JPEG → quality 80 → resize 50% Lanczos3
   → rename datetime → save → verify 1 file on disk, dimensions halved, size < original
2. **Full PNG pipeline**: JPEG in → PNG out → skip quality page → lossless round-trip check
3. **Full WebP pipeline**: HEIC in → WebP out → file exists, extension correct
4. **Batch of 5 files**: all 5 saved, all 5 renamed with sequential pattern, no name collisions
5. **Cancel save dialog**: `open_save_dialog` returns `None` → no files written

### Frontend E2E Tests (WebdriverIO + tauri-driver)

| Scenario | Steps | Expected |
|---|---|---|
| Tray click shows window | Click tray icon | Window visible, centered |
| Close hides window | Click × | Window hidden, process alive |
| Drag & drop PNG | Drop file on Page 1 | File stored, navigate to Page 2 |
| OPEN button | Click OPEN, pick file | Same as drop |
| Format selection | Click PNG | Button filled, NEXT enabled |
| Skip quality for PNG | PNG → NEXT | Jumps to Page 4, not Page 3 |
| JPEG shows quality page | JPEG → NEXT | Page 3 visible |
| Aspect ratio lock W→H | Page 4, change %W=50, lock=on | %H becomes 50 |
| Aspect ratio unlock | lock=off, change %W=50 | %H unchanged |
| px↔pct sync | Change px_w | pct_w updates correctly |
| File size preview | Change quality/dimensions | Footer text updates within 500ms |
| Rename preview live | Change prefix text | Preview names update instantly |
| Save flow | Page 6 → SAVE → pick dir | Files written, window hides |
| Window resets | After save, click tray | Page 1 shown, wizard reset |

---

## AGENT 3 — Optimizer

### Performance

1. **Parallel batch conversion** — use `rayon::par_iter()` in `convert_images` command.
   Each file processed in its own thread. Emit Tauri event `"progress"` with
   `{ current: usize, total: usize }` after each file completes. Frontend shows
   a progress bar on Page 6 during save.

2. **Debounce estimate calls** — Page 4 inputs call `estimate_output` at most once
   per 300ms. Implement debounce in Svelte using a timeout store helper.
   Cancel pending call if new input arrives before timeout fires.

3. **Lightweight estimation** — `estimate_file_size` encodes to memory buffer.
   For resize preview, encode a downscaled proxy (max 200px wide) and
   extrapolate: `estimated = proxy_bytes * (target_w * target_h) / (proxy_w * proxy_h)`.
   This is ~10× faster than full-resolution encode on large files.

4. **HEIC loading indicator** — HEIC decoding can be slow (1–3s for large files).
   After drop/OPEN, show a spinner overlay on Page 1 while `get_image_info` runs.
   The command runs async; do not block the UI thread.

5. **Memory discipline** — after each file is saved in the batch loop, explicitly
   `drop(img)` the `DynamicImage` before moving to the next file. Never hold
   more than one decoded image in memory at a time.

### Binary size

Add to `Cargo.toml`:
```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

Target binary size goal: **< 12 MB** on macOS arm64.

### UI smoothness

- Page transitions: CSS `transform: translateX()` slide animation, 180ms ease-out.
  Use Svelte `transition:` directive with custom params.
- Input focus states: instant (`transition: none`) — never delay user input feedback.
- Buttons: hover state changes within 80ms (`transition: background 80ms`).

---

## AGENT 4 — Documenter

### Files to create

**README.md** — root of project:
- 1-paragraph description
- Screenshot strip: all 6 wizard pages side-by-side
- Requirements: macOS 12 Monterey+ (arm64 or x86_64), Windows 11 (future)
- Install: download `.dmg`, drag to Applications
- Build from source:
  ```bash
  npm install
  cargo tauri build
  ```
- Supported input formats: HEIC, JPG, PNG
- Supported output formats: JPEG, PNG, WebP
- Resize algorithm comparison table (Nearest/Bilinear/CatmullRom/Lanczos3/Gaussian)

**CLAUDE.md** — project root (for Claude Code context):
```markdown
# pixshrink

## Architecture
- src-tauri/src/converter.rs — all image processing (load/resize/save/estimate)
- src-tauri/src/commands.rs — Tauri IPC layer (called from frontend via invoke())
- src-tauri/src/file_utils.rs — rename pattern logic
- src/stores/wizard.js — single global state store for wizard flow
- src/App.svelte — root component, always renders blurred background + page router

## Canvas rule
Window is ALWAYS 428×318 px, non-resizable.
page1_background.png is always the background on every page.
Never change window dimensions.

## Key commands
cargo tauri dev         # dev server with hot reload
cargo tauri build       # production .dmg / .exe
cargo test              # Rust unit tests
npm test                # frontend E2E tests

## IPC contract
All Tauri commands are in commands.rs.
Frontend calls them via: import { invoke } from '@tauri-apps/api'
Types are documented in each command's doc-comment.
```

**Inline code comments (English only):**
- Every `pub fn` in Rust: `///` doc comment with params + return description
- Non-obvious logic (aspect-ratio recalc, size estimation shortcut): inline `//` comment
- Each Svelte page file: top-of-file comment block explaining page's role in wizard

---

## AGENT 5 — UI/UX Designer

### Design System

**Color palette** (CSS variables in `:root`):
```css
--bg-overlay: rgba(10, 10, 10, 0.55);
--accent: #0a84ff;
--accent-hover: #0071e3;
--text-primary: #f0f0f0;
--text-secondary: #888888;
--text-hint: #555555;
--btn-outline-border: rgba(255,255,255,0.35);
--btn-outline-hover: rgba(255,255,255,0.12);
--input-bg: rgba(255,255,255,0.08);
--input-border: rgba(255,255,255,0.18);
--input-border-focus: #0a84ff;
```

**Typography:**
- Font stack: `-apple-system, 'SF Pro Display', 'Helvetica Neue', sans-serif`
- Main labels: 13px, `--text-primary`, weight 400
- Page titles: 10px, `--text-secondary`, weight 600, `letter-spacing: 0.12em`, uppercase
- Big number (Page 3 quality): 36px, weight 700, `--text-primary`
- Footer info (Page 4): 11px, `--text-secondary`
- Drop zone text: 15px, `--text-secondary`, weight 300

**Global canvas styles:**
```css
.canvas {
  width: 428px;
  height: 318px;
  overflow: hidden;
  position: relative;
  user-select: none;
}
.canvas .bg {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  filter: blur(5px) brightness(0.45);
  z-index: 0;
}
.canvas .overlay {
  position: absolute;
  inset: 0;
  background: rgba(10, 10, 10, 0.55);
  z-index: 1;
}
.canvas .content {
  position: absolute;
  inset: 0;
  z-index: 2;
  display: flex;
  flex-direction: column;
  align-items: center;
}
```

### Component Styles

**Buttons — filled (NEXT, SAVE, format selected):**
```css
background: var(--accent);
color: white;
border: none;
border-radius: 8px;
font-size: 13px;
font-weight: 500;
cursor: pointer;
transition: background 80ms;
```
Hover: `background: var(--accent-hover)`
Disabled: `opacity: 0.4; cursor: default`

**Buttons — outlined (OPEN, BACK, format unselected):**
```css
background: transparent;
color: var(--text-primary);
border: 1px solid var(--btn-outline-border);
border-radius: 8px;
transition: background 80ms, border-color 80ms;
```
Hover: `background: var(--btn-outline-hover); border-color: rgba(255,255,255,0.6)`

**Text inputs:**
```css
background: var(--input-bg);
border: 1px solid var(--input-border);
border-radius: 6px;
color: var(--text-primary);
font-size: 13px;
text-align: center;
outline: none;
transition: border-color 120ms;
```
Focus: `border-color: var(--input-border-focus)`

**Dropdown select:**
Same as text input + custom arrow via `appearance: none` + SVG background-image.

**Checkbox (Lock aspect ratio):**
Use custom styled checkbox: hidden native input + CSS pseudo-element square.
Checked state: `background: var(--accent)`, checkmark via `content: '✓'`.

### Page Transitions

```css
/* Svelte transition params */
/* Enter from right, exit to left */
in: x=30, duration=180ms, easing=cubicOut
out: x=-30, duration=180ms, easing=cubicIn
```

### Navigation Bar

Fixed to bottom of `.content`, `height: 44px`, `padding: 0 16px`:
- `display: flex; align-items: center; justify-content: space-between`
- BACK: left — text button, `--text-secondary`, 12px uppercase
- Dots: center — 6 dots `8×8px`, `border-radius: 50%`,
  current: `var(--accent)`, rest: `#333`, gap `6px`
- NEXT/SAVE: right — filled button, `80×28px`

---

## Build & Release Checklist

- [ ] `page1_background.png` copied to `src/assets/` and referenced as `/page1_background.png`
- [ ] Window size locked in `tauri.conf.json` (428×318, resizable=false)
- [ ] Tray icon exists at `src-tauri/icons/tray.png` (22×22, template-style monochrome)
- [ ] App icon set (generate with `cargo tauri icon icon.png`)
- [ ] `cargo clippy` passes with no warnings
- [ ] All 6 wizard pages render correctly at 428×318 without overflow/scroll
- [ ] Background image visible (blurred) on every page
- [ ] HEIC, JPG, PNG all accepted on Page 1
- [ ] Quality page auto-skipped for PNG and WebP
- [ ] Aspect ratio lock works for both pct and px inputs
- [ ] File size estimate updates in real-time on Page 4
- [ ] Rename preview updates in real-time on Page 5
- [ ] Save dialog defaults to source file directory
- [ ] Window hides after successful save
- [ ] Tray icon click restores window and resets wizard to Page 1
- [ ] Release build tested: `cargo tauri build --target aarch64-apple-darwin`
- [ ] Binary size < 12 MB

---

*Plan version 1.0 — generated for PixShrink image converter*
