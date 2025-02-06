import { test, expect } from '@playwright/test';

[
  { name: 'Vite packed', port: 4151 },

  { name: 'Webpack packed', port: 4152 },

  { name: 'Parcel preview', port: 4153 },
  { name: 'Parcel packed', port: 4154 },

  { name: 'Parcel_packageExports preview', port: 4155 },
  { name: 'Parcel_packageExports packed', port: 4156 },

  { name: 'ESBuild packed', port: 4157 },
].forEach(({ name, port }) => {
  test(name, async ({ page }) => {
    await page.goto(`http://localhost:${port}/`);

    const s = await page.locator('#selectors')
    await expect(s).toHaveText('fae7ab82');

    const a = await page.locator('#arguments')
    await expect(a).toHaveText('uint32');

    const m = await page.locator('#state_mutability')
    await expect(m).toHaveText('pure');

    const d = await page.locator('#disassembled')
    await expect(d).toHaveText(/0 PUSH1 80.*/);
  });
});
