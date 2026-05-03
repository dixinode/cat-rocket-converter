<!-- Page 4 keeps resize percentages and pixel dimensions in sync. -->
<script>
  import { wizard, updateWizard } from '../stores/wizard.js';

  const algorithmLabels = {
    nearest: 'Nearest (fastest)',
    bilinear: 'Bilinear',
    catmullrom: 'CatmullRom',
    lanczos3: 'Lanczos3 (best quality)',
    gaussian: 'Gaussian',
  };

  function clampPositive(value) {
    return Math.max(1, Number.isFinite(value) ? Math.round(value) : 1);
  }

  function updatePercent(axis, rawValue) {
    const original = axis === 'w' ? $wizard.originalDimensions.w : $wizard.originalDimensions.h;
    const nextPct = clampPositive(Number(rawValue));
    const nextPx = clampPositive((original * nextPct) / 100);

    updateWizard((state) => {
      const resize = { ...state.resize, [`pct_${axis}`]: nextPct, [`px_${axis}`]: nextPx };

      if (resize.lockAspect) {
        // Keep the second axis locked to the same ratio to avoid drift over repeated edits.
        const otherAxis = axis === 'w' ? 'h' : 'w';
        const otherOriginal = otherAxis === 'w' ? state.originalDimensions.w : state.originalDimensions.h;
        resize[`pct_${otherAxis}`] = nextPct;
        resize[`px_${otherAxis}`] = clampPositive((otherOriginal * nextPct) / 100);
      }

      return { ...state, resize };
    });
  }

  function updatePixels(axis, rawValue) {
    const original = axis === 'w' ? $wizard.originalDimensions.w : $wizard.originalDimensions.h;
    const nextPx = clampPositive(Number(rawValue));
    const nextPct = clampPositive((nextPx / original) * 100);

    updateWizard((state) => {
      const resize = { ...state.resize, [`px_${axis}`]: nextPx, [`pct_${axis}`]: nextPct };

      if (resize.lockAspect) {
        // Pixel edits become the source of truth and back-propagate to the paired dimension.
        const otherAxis = axis === 'w' ? 'h' : 'w';
        const otherOriginal = otherAxis === 'w' ? state.originalDimensions.w : state.originalDimensions.h;
        resize[`pct_${otherAxis}`] = nextPct;
        resize[`px_${otherAxis}`] = clampPositive((otherOriginal * nextPct) / 100);
      }

      return { ...state, resize };
    });
  }

  function updateLockAspect(event) {
    const lockAspect = event.currentTarget.checked;
    updateWizard((state) => ({ ...state, resize: { ...state.resize, lockAspect } }));
  }

  function updateAlgorithm(event) {
    const algorithm = event.currentTarget.value;
    updateWizard((state) => ({ ...state, resize: { ...state.resize, algorithm } }));
  }

</script>

<section class="page-shell">
  <div class="resize-panel">
    <div class="resize-grid">
      <div class="column">
        <label class="input-label" for="pct_w">%W</label>
        <input class="text-input small-input" id="pct_w" min="1" type="number" value={$wizard.resize.pct_w} on:input={(event) => updatePercent('w', event.currentTarget.value)} />
        <label class="input-label" for="pct_h">%H</label>
        <input class="text-input small-input" id="pct_h" min="1" type="number" value={$wizard.resize.pct_h} on:input={(event) => updatePercent('h', event.currentTarget.value)} />
      </div>

      <div class="column">
        <label class="input-label" for="px_w">px W</label>
        <input class="text-input small-input" id="px_w" min="1" type="number" value={$wizard.resize.px_w} on:input={(event) => updatePixels('w', event.currentTarget.value)} />
        <label class="input-label" for="px_h">px H</label>
        <input class="text-input small-input" id="px_h" min="1" type="number" value={$wizard.resize.px_h} on:input={(event) => updatePixels('h', event.currentTarget.value)} />
      </div>
    </div>

    <label class="checkbox-row" for="lock-aspect">
      <input checked={$wizard.resize.lockAspect} id="lock-aspect" type="checkbox" on:change={updateLockAspect} />
      <span class="checkbox-box"></span>
      <span>Lock aspect ratio</span>
    </label>

    <select class="select-input algorithm-select" value={$wizard.resize.algorithm} on:change={updateAlgorithm}>
      {#each Object.entries(algorithmLabels) as [value, label]}
        <option value={value}>{label}</option>
      {/each}
    </select>
  </div>
</section>

<style>
  .resize-panel {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
    padding-top: 4px;
  }

  .resize-grid {
    display: flex;
    gap: 24px;
  }

  .column {
    display: grid;
    grid-template-columns: 1fr;
    gap: 6px;
    justify-items: center;
  }

  .input-label {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .small-input {
    width: 72px;
    height: 32px;
  }

  .checkbox-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--text-primary);
  }

  .checkbox-row input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .checkbox-box {
    width: 14px;
    height: 14px;
    border-radius: 4px;
    border: 1px solid var(--input-border);
    background: transparent;
    display: inline-grid;
    place-items: center;
  }

  .checkbox-row input:checked + .checkbox-box {
    background: var(--accent);
    border-color: var(--accent);
  }

  .checkbox-row input:checked + .checkbox-box::after {
    content: '✓';
    color: white;
    font-size: 10px;
    line-height: 1;
  }

  .algorithm-select {
    width: 240px;
    height: 34px;
    padding: 0 30px 0 12px;
    text-align: left;
  }
</style>
