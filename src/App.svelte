<script>
  import { onMount } from 'svelte';
  import { cubicIn, cubicOut } from 'svelte/easing';
  import { debugLog, getImageInfo, listenToFinderOpenFiles, syncWindowViewport, takePendingOpenFiles } from './lib/tauri.js';
  import { wizard, goToPage, resetWizard, setFiles } from './stores/wizard.js';
  import waitForConversionBackground from '../wait_for_conversion.png';
  import Page1Drop from './pages/Page1Drop.svelte';
  import Page2Format from './pages/Page2Format.svelte';
  import Page3Quality from './pages/Page3Quality.svelte';
  import Page4Resize from './pages/Page4Resize.svelte';
  import Page5Rename from './pages/Page5Rename.svelte';
  import Page6Save from './pages/Page6Save.svelte';

  let unlistenFinderOpen;

  function slidePage(node, { x = 30, duration = 180, easing = cubicOut } = {}) {
    return {
      duration,
      easing,
      css: (t) => `transform: translateX(${(1 - t) * x}px); opacity: ${t};`,
    };
  }

  function canGoNext(state) {
    if (state.currentPage === 1) {
      return state.files.length > 0;
    }

    if (state.currentPage === 2) {
      return Boolean(state.outputFormat);
    }

    if (state.currentPage === 3) {
      return state.quality >= 1 && state.quality <= 100;
    }

    if (state.currentPage === 4) {
      return state.resize.px_w > 0 && state.resize.px_h > 0;
    }

    if (state.currentPage === 5) {
      return true;
    }

    return false;
  }

  function handleNext() {
    if (!canGoNext($wizard)) {
      return;
    }

    if ($wizard.currentPage === 2) {
      goToPage($wizard.outputFormat === 'jpeg' ? 3 : 4);
      return;
    }

    if ($wizard.currentPage < 6) {
      goToPage($wizard.currentPage + 1);
    }
  }

  function handleBack() {
    if ($wizard.currentPage === 4 && $wizard.outputFormat !== 'jpeg') {
      goToPage(2);
      return;
    }

    if ($wizard.currentPage > 1) {
      goToPage($wizard.currentPage - 1);
    }
  }

  async function loadFinderSelection(files) {
    if (!files.length) {
      return;
    }

    try {
      const info = await getImageInfo(files[0]);
      resetWizard();
      setFiles(files, info);
    } catch (error) {
      void debugLog('error', 'failed to load files opened from Finder', {
        files,
        error: String(error),
      });
    }
  }

  async function consumePendingFinderFiles() {
    const files = await takePendingOpenFiles();
    await loadFinderSelection(files);
  }

  onMount(() => {
    let disposed = false;

    const logCanvasMetrics = () => {
      const canvas = document.querySelector('.canvas');
      const rect = canvas?.getBoundingClientRect();

      void debugLog('info', 'canvas metrics', {
        windowInnerWidth: window.innerWidth,
        windowInnerHeight: window.innerHeight,
        canvasWidth: rect ? Math.round(rect.width) : null,
        canvasHeight: rect ? Math.round(rect.height) : null,
      });
    };

    const syncViewport = () => {
      void syncWindowViewport(window.innerWidth, window.innerHeight);
    };

    requestAnimationFrame(() => {
      syncViewport();
      logCanvasMetrics();
      setTimeout(logCanvasMetrics, 120);
    });
    window.addEventListener('resize', logCanvasMetrics);

    void (async () => {
      unlistenFinderOpen = await listenToFinderOpenFiles(() => {
        void consumePendingFinderFiles();
      });

      if (!disposed) {
        await consumePendingFinderFiles();
      }
    })();

    return () => {
      disposed = true;
      window.removeEventListener('resize', logCanvasMetrics);
      if (unlistenFinderOpen) {
        void unlistenFinderOpen();
      }
    };
  });
</script>

<div class:waiting={$wizard.showWaitingBackground} class="canvas">
  <img class="bg default-bg" src="/page1_background.png" alt="" />
  <img class="bg waiting-bg" src={waitForConversionBackground} alt="" />
  <div class="overlay"></div>

  {#if !$wizard.showWaitingBackground}
    <div class="content">
      {#key $wizard.currentPage}
        <div class="page-transition" in:slidePage={{ x: 30, duration: 180, easing: cubicOut }} out:slidePage={{ x: -30, duration: 180, easing: cubicIn }}>
          {#if $wizard.currentPage === 1}
            <Page1Drop />
          {:else if $wizard.currentPage === 2}
            <Page2Format />
          {:else if $wizard.currentPage === 3}
            <Page3Quality />
          {:else if $wizard.currentPage === 4}
            <Page4Resize />
          {:else if $wizard.currentPage === 5}
            <Page5Rename />
          {:else}
            <Page6Save />
          {/if}
        </div>
      {/key}

      <nav class="nav-bar">
        <div class="nav-slot">
          {#if $wizard.currentPage > 1}
            <button class="nav-text" type="button" on:click={handleBack}>Back</button>
          {/if}
        </div>

        <div class="dots" aria-label="Wizard progress">
          {#each Array.from({ length: 6 }, (_, index) => index + 1) as page}
            <span class:active={page === $wizard.currentPage} class="dot"></span>
          {/each}
        </div>

        <div class="nav-slot right">
          {#if $wizard.currentPage !== 6}
            <button class="nav-text nav-next" disabled={!canGoNext($wizard)} type="button" on:click={handleNext}>NEXT</button>
          {/if}
        </div>
      </nav>
    </div>
  {/if}
</div>
