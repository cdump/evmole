import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'firefox',
      use: { ...devices['Desktop Firefox'] },
    },
    {
      name: 'webkit',
      use: { ...devices['Desktop Safari'] },
    },
  ],
  webServer: [
    // vite packed
    {
      command: 'npm --prefix ../examples/vite run build && python3 webserver.py 4151 ../examples/vite/dist',
      url: 'http://localhost:4151',
    },

    // webpack packed
    {
      command: 'npm --prefix ../examples/webpack run build && python3 webserver.py 4152 ../examples/webpack/dist',
      url: 'http://localhost:4152',
    },

    // parcel preview
    {
      command: 'npm --prefix ../examples/parcel run start -- --port 4153',
      url: 'http://localhost:4153',
    },

    // parcel packed
    {
      command: 'npm --prefix ../examples/parcel run build && python3 webserver.py 4154 ../examples/parcel/dist',
      url: 'http://localhost:4154',
    },

    // parcel_packageExports preview
    {
      command: 'npm --prefix ../examples/parcel_packageExports run start -- --port 4155',
      url: 'http://localhost:4155',
    },

    // parcel_packageExports packed
    {
      command: 'npm --prefix ../examples/parcel_packageExports run build && python3 webserver.py 4156 ../examples/parcel_packageExports/dist',
      url: 'http://localhost:4156',
    },

    // esbuild packed
    {
      command: 'npm --prefix ../examples/esbuild run build && python3 webserver.py 4157 ../examples/esbuild/dist',
      url: 'http://localhost:4157',
    },
  ]
});
