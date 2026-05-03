export const config = {
  runner: 'local',
  framework: 'mocha',
  specs: ['./tests/e2e/**/*.e2e.js'],
  maxInstances: 1,
  reporters: ['spec'],
  mochaOpts: {
    ui: 'bdd',
    timeout: 60000,
  },
  hostname: process.env.TAURI_DRIVER_HOST ?? '127.0.0.1',
  port: Number(process.env.TAURI_DRIVER_PORT ?? 4444),
  path: '/',
  capabilities: [
    {
      browserName: 'wry',
      'tauri:options': {
        application: process.env.TAURI_WDIO_BINARY ?? './src-tauri/target/debug/pixshrink',
      },
    },
  ],
};
