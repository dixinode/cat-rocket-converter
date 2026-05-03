import { get, writable } from 'svelte/store';

export function createInitialWizardState() {
  return {
    files: [],
    outputFormat: null,
    quality: 100,
    resize: {
      pct_w: 100,
      pct_h: 100,
      px_w: 0,
      px_h: 0,
      lockAspect: true,
      algorithm: 'lanczos3',
    },
    originalDimensions: { w: 0, h: 0 },
    rename: {
      pattern: 'datetime',
      prefix: 'IMG_',
      suffix: '_compressed',
      seqBase: 'export_',
      seqStart: 1,
    },
    currentPage: 1,
    saving: false,
    showWaitingBackground: false,
  };
}

export const wizard = writable(createInitialWizardState());

export function updateWizard(updater) {
  wizard.update((state) => updater(structuredClone(state)));
}

export function resetWizard() {
  wizard.set(createInitialWizardState());
}

export function getWizard() {
  return get(wizard);
}

export function goToPage(page) {
  updateWizard((state) => ({ ...state, currentPage: page }));
}

export function setFiles(files, info) {
  updateWizard((state) => ({
    ...state,
    files,
    outputFormat: 'jpeg',
    currentPage: 2,
    originalDimensions: { w: info.width, h: info.height },
    resize: {
      ...state.resize,
      pct_w: 100,
      pct_h: 100,
      px_w: info.width,
      px_h: info.height,
    },
  }));
}
