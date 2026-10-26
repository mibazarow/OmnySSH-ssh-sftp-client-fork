import { expect, test, type Page } from '@playwright/test';

// Settings (tech-gui.md §4.3). e2e runs against the static SPA with Tauri absent, so we
// stub `__TAURI_INTERNALS__` at the boundary (§6.4). `plugin:app|version` is Tauri's
// `getVersion`.
const HOSTS = [
  { name: 'web-1', hostname: 'web-1.example.com', user: 'deploy', port: 22, tags: [], hasKey: true, localForwards: [], tunnelAutostart: false, forwardAgent: false }
];

const INSTALLED = '1.0.0';

async function boot(
  page: Page,
  opts: {
    traySupport?: { available: boolean; minimize: boolean };
    version?: 'fails' | 'hangs';
  } = {}
): Promise<void> {
  await page.addInitScript(
    ({ hosts, installed, traySupport, version }) => {
      let cbid = 0;
      const listeners: Record<string, number[]> = {};
      const state = {
        hosts: hosts.map((h) => ({ ...h }))
      };
      const win = window as unknown as Record<string, unknown>;

      function fire(event: string, payload: unknown): void {
        for (const id of listeners[event] ?? []) {
          const cb = win[`__cb${id}`] as ((e: unknown) => void) | undefined;
          cb?.({ event, id, payload });
        }
      }

      (win as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
        invoke: (cmd: string, args: Record<string, unknown>) => {
          switch (cmd) {
            case 'list_hosts':
              return Promise.resolve([...state.hosts]);
            case 'reload_hosts':
              setTimeout(() => {
                fire('hosts-loaded', [...state.hosts]);
              }, 0);
              return Promise.resolve(null);
            case 'refresh_metrics':
              return Promise.resolve(null);
            case 'plugin:app|version':
              if (version === 'fails') return Promise.reject('no Tauri runtime');
              if (version === 'hangs') return new Promise(() => {});
              return Promise.resolve(installed);
            case 'set_tray_behavior':
              ((win.__tray ??= []) as unknown[]).push({ ...args });
              return Promise.resolve({ ...traySupport });
            case 'plugin:event|listen': {
              const { event, handler } = args as { event: string; handler: number };
              (listeners[event] ||= []).push(handler);
              return Promise.resolve(cbid);
            }
            default:
              return Promise.resolve(null);
          }
        },
        transformCallback: (cb: unknown) => {
          const id = ++cbid;
          win[`__cb${id}`] = cb;
          return id;
        }
      };
    },
    {
      hosts: HOSTS,
      installed: INSTALLED,
      traySupport: opts.traySupport ?? { available: true, minimize: true },
      version: opts.version
    }
  );
  await page.goto('/');
  await expect(page.getByText('web-1', { exact: true })).toBeVisible();
}

test('the footer gear opens Settings; theme and interval prefs work', async ({ page }) => {
  await boot(page);

  await page.getByRole('button', { name: 'Settings' }).click();
  await expect(page.getByRole('heading', { name: 'Settings' })).toBeVisible();

  // Theme mirrors the sidebar toggle (§5.1): the app boots dark; picking Light flips it.
  // `exact` avoids the sidebar toggle whose label reads "Switch to light theme".
  await page.getByRole('button', { name: 'Light', exact: true }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await page.getByRole('button', { name: 'Dark', exact: true }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');

  // Auto-refresh interval is a segmented pref.
  const tenSec = page.getByRole('button', { name: '10s', exact: true });
  await tenSec.click();
  await expect(tenSec).toHaveAttribute('aria-pressed', 'true');
});

test('About shows the installed version, selectable for a bug report', async ({ page }) => {
  await boot(page);
  await page.getByRole('button', { name: 'Settings' }).click();

  const about = page.getByRole('heading', { name: 'About' }).locator('..');
  await expect(about.getByText('Version', { exact: true })).toBeVisible();
  const version = about.getByText(INSTALLED, { exact: true });
  await expect(version).toBeVisible();
  await expect(version).toHaveCSS('user-select', 'text');
});

for (const version of ['fails', 'hangs'] as const) {
  test(`a version read that ${version} leaves no row`, async ({ page }) => {
    await boot(page, { version });
    await page.getByRole('button', { name: 'Settings' }).click();

    await expect(page.getByRole('heading', { name: 'About' })).toHaveCount(0);
    await expect(page.getByText('Version', { exact: true })).toHaveCount(0);
  });
}

test('scrolled to the end, Settings keeps its bottom padding', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 480 });
  await boot(page);
  await page.getByRole('button', { name: 'Settings' }).click();

  const last = page.getByRole('heading', { name: 'About' }).locator('..');
  await expect(last).toBeVisible();
  const { overflows, gap } = await last.evaluate((card) => {
    const scroller = card.closest('.overflow-auto') as HTMLElement;
    scroller.scrollTop = scroller.scrollHeight;
    return {
      overflows: scroller.scrollHeight > scroller.clientHeight,
      gap: scroller.getBoundingClientRect().bottom - card.getBoundingClientRect().bottom
    };
  });
  expect(overflows).toBe(true);
  expect(gap).toBeGreaterThanOrEqual(24);
});

const trayCalls = (page: Page) =>
  page.evaluate(() => (window as unknown as { __tray?: unknown[] }).__tray ?? []);

// The Desktop Chrome device reports a Windows user agent.
test('the tray settings reach the backend and survive a restart', async ({ page }) => {
  await boot(page);
  // Off by default, and the backend is told so on start.
  await expect.poll(() => trayCalls(page)).toEqual([{ minimizeToTray: false, closeToTray: false }]);

  await page.getByRole('button', { name: 'Settings' }).click();
  const close = page.getByRole('switch', { name: 'Close to tray' });
  await expect(page.getByRole('switch', { name: 'Minimize to tray' })).toHaveAttribute(
    'aria-checked',
    'false'
  );
  await close.click();
  await expect(close).toHaveAttribute('aria-checked', 'true');
  await expect
    .poll(async () => (await trayCalls(page)).at(-1))
    .toEqual({ minimizeToTray: false, closeToTray: true });

  await page.reload();
  await expect(page.getByText('web-1', { exact: true })).toBeVisible();
  await expect.poll(() => trayCalls(page)).toEqual([{ minimizeToTray: false, closeToTray: true }]);
});

test('without a system tray the settings say so and stay off', async ({ page }) => {
  await boot(page, { traySupport: { available: false, minimize: false } });

  await page.getByRole('button', { name: 'Settings' }).click();
  await expect(page.getByText('This desktop has no system tray')).toBeVisible();
  await expect(page.getByRole('switch', { name: 'Close to tray' })).toBeDisabled();
  await expect(page.getByRole('switch', { name: 'Minimize to tray' })).toBeDisabled();
});

test('under Wayland the window closes to the tray but cannot minimize into it', async ({
  page
}) => {
  await boot(page, { traySupport: { available: true, minimize: false } });

  await page.getByRole('button', { name: 'Settings' }).click();
  await expect(page.getByRole('switch', { name: 'Minimize to tray' })).toBeDisabled();
  await expect(page.getByText(/Not on Wayland/)).toBeVisible();
  await expect(page.getByRole('switch', { name: 'Close to tray' })).toBeEnabled();
});

test.describe('on macOS', () => {
  test.use({
    userAgent:
      'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.0.0 (KHTML, like Gecko)'
  });

  test('the window closes to the menu bar, and minimizing stays with the Dock', async ({
    page
  }) => {
    await boot(page);

    await page.getByRole('button', { name: 'Settings' }).click();
    await expect(page.getByRole('switch', { name: 'Close to the menu bar' })).toBeVisible();
    await expect(page.getByRole('switch', { name: 'Minimize to tray' })).toHaveCount(0);
  });
});
