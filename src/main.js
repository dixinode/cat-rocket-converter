import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';
import { debugLog } from './lib/tauri.js';

window.addEventListener('error', (event) => {
  void debugLog('error', 'window error', {
    message: event.message,
    filename: event.filename,
    line: event.lineno,
    column: event.colno,
  });
});

window.addEventListener('unhandledrejection', (event) => {
  void debugLog('error', 'unhandled rejection', {
    reason: String(event.reason),
  });
});

let app;

async function bootstrap() {
  await debugLog('info', 'frontend bootstrap start');

  app = mount(App, {
    target: document.getElementById('app'),
  });

  await debugLog('info', 'frontend bootstrap complete');
}

bootstrap().catch((error) => {
  console.error(error);
  void debugLog('error', 'frontend bootstrap failed', {
    reason: String(error),
  });
});

export default app;
