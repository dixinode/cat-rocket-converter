import { spawnSync } from 'node:child_process';

if (process.env.RUN_TAURI_E2E !== '1') {
  console.log('PixShrink E2E scaffold is present. Set RUN_TAURI_E2E=1 and start tauri-driver to execute WebdriverIO scenarios.');
  process.exit(0);
}

const result = spawnSync('npx', ['wdio', 'run', './wdio.conf.js'], {
  stdio: 'inherit',
  shell: process.platform === 'win32',
});

process.exit(result.status ?? 1);
