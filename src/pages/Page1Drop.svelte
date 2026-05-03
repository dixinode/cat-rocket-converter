<!-- Page 1 handles file selection and initial image metadata loading. -->
<script>
  import { onDestroy, onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { setFiles } from '../stores/wizard.js';
  import { getImageInfo, openImagePicker } from '../lib/tauri.js';

  let dragActive = false;
  let loading = false;
  let error = '';
  let unlistenDrop;

  async function loadSelection(files) {
    if (!files.length) {
      return;
    }

    loading = true;
    error = '';

    try {
      const info = await getImageInfo(files[0]);
      setFiles(files, info);
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
      dragActive = false;
    }
  }

  async function handleOpen() {
    const files = await openImagePicker();
    await loadSelection(files);
  }

  onMount(async () => {
    unlistenDrop = await getCurrentWindow().onDragDropEvent((event) => {
      if (event.payload.type === 'enter' || event.payload.type === 'over') {
        dragActive = true;
        return;
      }

      if (event.payload.type === 'leave') {
        dragActive = false;
        return;
      }

      dragActive = false;
      void loadSelection(event.payload.paths);
    });
  });

  onDestroy(() => {
    if (unlistenDrop) {
      void unlistenDrop();
    }
  });
</script>

<section aria-label="Image drop zone" class:drag-active={dragActive} class="page-shell drop-shell" role="group">
  <div class="drop-content">
    <p class="drop-text">drag and drop your pictures</p>
    <button class="button-outline open-button" type="button" on:click={handleOpen}>OPEN</button>
    {#if error}
      <p class="status-text error">{error}</p>
    {/if}
  </div>

  {#if loading}
    <div class="loading-overlay">
      <div class="spinner"></div>
      <span>Loading image info…</span>
    </div>
  {/if}
</section>

<style>
  .drop-shell {
    width: 100%;
    border: 2px dashed transparent;
    box-sizing: border-box;
    transition: border-color 80ms, background 80ms;
  }

  .drop-shell.drag-active {
    border-color: var(--dropzone-active-border);
    background: var(--dropzone-active-tint);
  }

  .drop-content {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 22px;
  }

  .drop-text {
    margin: 0;
    font-size: 15px;
    font-weight: 300;
    color: var(--drop-text);
  }

  .open-button {
    width: 80px;
    height: 28px;
    border-radius: 6px;
  }

  .status-text {
    max-width: 260px;
    margin: 0;
    font-size: 11px;
    text-align: center;
    color: var(--text-secondary);
  }

  .status-text.error {
    color: var(--danger);
  }

  .loading-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    background: var(--loading-scrim);
    font-size: 12px;
    color: var(--text-primary);
  }

  .spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--spinner-track);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
