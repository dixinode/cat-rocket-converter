<!-- Page 3 captures JPEG quality before conversion continues to resizing. -->
<script>
  import { wizard, updateWizard } from '../stores/wizard.js';

  function handleInput(event) {
    const quality = Number(event.currentTarget.value);
    updateWizard((state) => ({ ...state, quality }));
  }
</script>

<section class="page-shell">
  <div class="quality-panel">
    <p class="page-title">JPEG Quality</p>
    <input aria-label="JPEG quality" max="100" min="1" type="range" value={$wizard.quality} on:input={handleInput} />
    <div class="quality-value">{$wizard.quality}</div>
    <p class="quality-hint">Higher = larger file, better quality</p>
  </div>
</section>

<style>
  .quality-panel {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
  }

  input[type='range'] {
    width: 300px;
  }

  .quality-value {
    font-size: 36px;
    font-weight: 700;
    color: var(--text-primary);
    line-height: 1;
  }

  .quality-hint {
    margin: 0;
    font-size: 11px;
    color: var(--text-secondary);
  }
</style>
