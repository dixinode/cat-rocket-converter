import { invoke } from '@tauri-apps/api/core';
import { emit, listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';

export async function openImagePicker() {
  const selected = await open({
    multiple: true,
    filters: [
      { name: 'Images', extensions: ['heic', 'heif', 'jpg', 'jpeg', 'png'] },
    ],
  });

  if (!selected) {
    return [];
  }

  return Array.isArray(selected) ? selected : [selected];
}

export function getImageInfo(path) {
  return invoke('get_image_info', { path });
}

export function estimateOutput(params) {
  return invoke('estimate_output', { params });
}

export function convertImages(params) {
  return invoke('convert_images', { params });
}

export function playSystemSound(soundName) {
  return invoke('play_system_sound', { soundName });
}

export function takePendingOpenFiles() {
  return invoke('take_pending_open_files');
}

export function openSaveDialog(defaultDir) {
  return invoke('open_save_dialog', { defaultDir });
}

export function getParentDir(filePath) {
  return invoke('get_parent_dir', { filePath });
}

export function emitConversionCompleted() {
  return emit('conversion_completed');
}

export function listenToProgress(handler) {
  return listen('progress', (event) => handler(event.payload));
}

export function listenToFinderOpenFiles(handler) {
  return listen('finder-open-files', (event) => handler(event.payload ?? []));
}

export function syncWindowViewport(viewportWidth, viewportHeight) {
  return invoke('sync_window_viewport', { viewportWidth, viewportHeight });
}
