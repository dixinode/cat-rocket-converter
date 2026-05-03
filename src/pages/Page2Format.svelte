<!-- Page 2 lets the user choose the output image format. -->
<script>
  import { wizard, updateWizard } from '../stores/wizard.js';

  const formats = ['jpeg', 'png', 'webp'];

  function selectFormat(outputFormat) {
    updateWizard((state) => ({ ...state, outputFormat }));
  }
</script>

<section class="page-shell">
  <div class="format-list">
    <p class="selection-count">{$wizard.files.length} {$wizard.files.length === 1 ? 'file selected' : 'files selected'}</p>

    {#each formats as format}
      <button class:selected={$wizard.outputFormat === format} class="format-button button-outline" type="button" on:click={() => selectFormat(format)}>
        {format.toUpperCase()}
      </button>
    {/each}
  </div>
</section>

<style>
  .format-list {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
  }

  .selection-count {
    margin: 0 0 4px;
    font-size: 11px;
    color: var(--text-secondary);
    letter-spacing: 0.04em;
  }

  .format-button {
    width: 200px;
    height: 44px;
    border-radius: 10px;
    font-size: 14px;
    font-weight: 600;
    letter-spacing: 0.04em;
  }

  .format-button.selected {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }
</style>
