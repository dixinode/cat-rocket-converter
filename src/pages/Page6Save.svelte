<!-- Page 6 runs the final batch conversion and resets the wizard after success. -->
<script>
  import { tick } from 'svelte';
  import { onDestroy, onMount } from 'svelte';
  import { wizard, resetWizard, updateWizard } from '../stores/wizard.js';
  import { convertImages, emitConversionCompleted, getParentDir, listenToProgress, openSaveDialog, playSystemSound } from '../lib/tauri.js';

  let progress = { current: 0, total: 0 };
  let error = '';
  let unlisten;

  function renamePatternPayload(rename) {
    if (rename.pattern === 'datetime') {
      return { kind: 'datetime', prefix: rename.prefix };
    }

    if (rename.pattern === 'suffix') {
      return { kind: 'suffix', suffix: rename.suffix };
    }

    if (rename.pattern === 'sequential') {
      return { kind: 'sequential', base: rename.seqBase, start: rename.seqStart };
    }

    return { kind: 'keep' };
  }

  function summaryText() {
    const qualityText = $wizard.outputFormat === 'jpeg' ? `, q${$wizard.quality}` : '';
    return `${$wizard.outputFormat?.toUpperCase() ?? ''}${qualityText}, ${$wizard.resize.px_w} x ${$wizard.resize.px_h}`;
  }

  function waitForNextPaint() {
    return new Promise((resolve) => {
      requestAnimationFrame(() => {
        requestAnimationFrame(resolve);
      });
    });
  }

  async function handleSave() {
    if ($wizard.saving || !$wizard.files.length) {
      return;
    }

    error = '';
    progress = { current: 0, total: $wizard.files.length };
    updateWizard((state) => ({ ...state, saving: true }));

    try {
      const defaultDir = await getParentDir($wizard.files[0]);
      const outputDir = await openSaveDialog(defaultDir);

      if (!outputDir) {
        updateWizard((state) => ({ ...state, saving: false, showWaitingBackground: false }));
        return;
      }

      updateWizard((state) => ({ ...state, showWaitingBackground: true }));
      await tick();
      await waitForNextPaint();

      await convertImages({
        files: $wizard.files,
        outputDir,
        format: $wizard.outputFormat,
        quality: $wizard.quality,
        pctW: $wizard.resize.pct_w,
        pctH: $wizard.resize.pct_h,
        resizeW: $wizard.resize.px_w,
        resizeH: $wizard.resize.px_h,
        lockAspect: $wizard.resize.lockAspect,
        algorithm: $wizard.resize.algorithm,
        renamePattern: renamePatternPayload($wizard.rename),
      });

      try {
        await playSystemSound('Glass');
      } catch (soundError) {
        console.warn('Failed to play completion sound:', soundError);
      }

      await emitConversionCompleted();
      resetWizard();
      progress = { current: 0, total: 0 };
    } catch (err) {
      error = String(err);
      updateWizard((state) => ({ ...state, saving: false, showWaitingBackground: false }));
    }
  }

  onMount(async () => {
    unlisten = await listenToProgress((payload) => {
      progress = payload;
    });
  });

  onDestroy(() => {
    if (unlisten) {
      void unlisten();
    }
  });
</script>

<section class="page-shell">
  <div class="save-panel">
    <div class="checkmark">✓</div>
    <div class="ready-text">Ready - {$wizard.files.length} files</div>
    <div class="summary">{summaryText()}</div>
    <button class="button-filled save-button" disabled={$wizard.saving} type="button" on:click={handleSave}>SAVE TO FOLDER</button>

    {#if $wizard.saving}
      <div class="progress-wrap">
        <div class="progress-bar">
          <div class="progress-fill" style={`width: ${progress.total ? (progress.current / progress.total) * 100 : 0}%`}></div>
        </div>
        <div class="progress-text">{progress.current} / {progress.total}</div>
      </div>
    {/if}

    {#if error}
      <div class="error-text">{error}</div>
    {/if}
  </div>
</section>

<style>
  .save-panel {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
  }

  .checkmark {
    font-size: 32px;
    color: var(--accent);
    line-height: 1;
  }

  .ready-text {
    font-size: 20px;
    color: var(--text-primary);
  }

  .summary {
    font-size: 12px;
    color: var(--text-secondary);
    text-align: center;
  }

  .save-button {
    width: 190px;
    height: 44px;
    border-radius: 10px;
    margin-top: 6px;
  }

  .progress-wrap {
    width: 180px;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .progress-bar {
    width: 100%;
    height: 8px;
    background: var(--progress-track);
    border-radius: 999px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--accent);
    transition: width 80ms;
  }

  .progress-text,
  .error-text {
    font-size: 11px;
    text-align: center;
    color: var(--text-secondary);
  }

  .error-text {
    color: var(--danger);
    max-width: 240px;
  }
</style>
