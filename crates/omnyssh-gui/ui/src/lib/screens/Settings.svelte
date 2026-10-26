<script lang="ts">
  // Minimal settings (tech-gui.md §4.3): the light/dark theme mirrored from the sidebar
  // (§5.1), the metric auto-refresh interval and the tray (tauri-plugin-store UI prefs).
  import { onMount } from 'svelte';
  import { Surface, Icon } from '$lib/theme';
  import { theme } from '$lib/stores/theme';
  import { streamerMode } from '$lib/stores/streamer';
  import { refreshInterval, REFRESH_OPTIONS } from '$lib/stores/settings';
  import { traySupport, trayBehavior } from '$lib/stores/tray';
  import { confirmQuit } from '$lib/stores/quit';
  import { isMac } from '$lib/platform';
  import { installedVersion } from '$lib/ipc/appInfo';

  let version = $state<string | null>(null);

  onMount(() => {
    // Not awaited, so settings never wait on it; a failed read leaves the row out.
    installedVersion().then((v) => (version = v), () => {});
  });

  const formatInterval = (secs: number): string => (secs < 60 ? `${secs}s` : `${secs / 60}m`);

  const seg =
    'rounded-lg px-3 py-1.5 text-sm transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus';
  const segState = (active: boolean): string =>
    active ? 'bg-accent text-accent-fg' : 'text-muted hover:bg-surface-inset hover:text-fg';
</script>

<section class="mx-auto min-h-full max-w-2xl p-6">
  <h1 class="mb-5 text-lg font-semibold tracking-tight">Settings</h1>

  <div class="space-y-4">
    <!-- Appearance -->
    <Surface class="p-5">
      <h2 class="mb-3 text-sm font-semibold">Appearance</h2>
      <div class="flex items-center justify-between gap-4">
        <div>
          <p class="text-sm">Theme</p>
          <p class="text-xs text-muted">Mirrors the sidebar toggle.</p>
        </div>
        <div class="flex gap-1 rounded-xl bg-surface-inset p-1">
          <button
            type="button"
            class="{seg} {segState($theme === 'light')}"
            aria-pressed={$theme === 'light'}
            onclick={() => theme.set('light')}
          >
            <span class="flex items-center gap-1.5"><Icon name="sun" size={14} /> Light</span>
          </button>
          <button
            type="button"
            class="{seg} {segState($theme === 'dark')}"
            aria-pressed={$theme === 'dark'}
            onclick={() => theme.set('dark')}
          >
            <span class="flex items-center gap-1.5"><Icon name="moon" size={14} /> Dark</span>
          </button>
        </div>
      </div>
    </Surface>

    <!-- Privacy -->
    <Surface class="p-5">
      <h2 class="mb-3 text-sm font-semibold">Privacy</h2>
      <div class="flex items-center justify-between gap-4">
        <div class="min-w-0">
          <p class="text-sm">Streamer mode</p>
          <p class="text-xs text-muted">
            Mask host addresses with realistic fakes, so real IPs stay off-screen while recording.
          </p>
        </div>
        <button
          type="button"
          role="switch"
          aria-checked={$streamerMode}
          aria-label="Streamer mode"
          onclick={() => streamerMode.toggle()}
          class="relative h-6 w-11 shrink-0 rounded-full transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus {$streamerMode
            ? 'bg-accent'
            : 'bg-surface-inset'}"
        >
          <span
            class="absolute top-0.5 h-5 w-5 rounded-full bg-surface shadow-soft transition-[left] {$streamerMode
              ? 'left-[1.375rem]'
              : 'left-0.5'}"
          ></span>
        </button>
      </div>
    </Surface>

    <!-- Dashboard -->
    <Surface class="p-5">
      <h2 class="mb-3 text-sm font-semibold">Dashboard</h2>
      <div class="flex items-center justify-between gap-4">
        <div>
          <p class="text-sm">Auto-refresh interval</p>
          <p class="text-xs text-muted">How often the dashboard forces a metric refresh.</p>
        </div>
        <div class="flex flex-wrap justify-end gap-1 rounded-xl bg-surface-inset p-1">
          {#each REFRESH_OPTIONS as secs (secs)}
            <button
              type="button"
              class="{seg} tabular-nums {segState($refreshInterval === secs)}"
              aria-pressed={$refreshInterval === secs}
              onclick={() => refreshInterval.set(secs)}
            >
              {formatInterval(secs)}
            </button>
          {/each}
        </div>
      </div>
    </Surface>

    <!-- Window: the tray keeps sessions and tunnels up with no window on screen. macOS
         has no minimize event to act on, and keeps minimized windows in the Dock. -->
    <Surface class="p-5">
      <h2 class="mb-3 text-sm font-semibold">Window</h2>
      <div class="space-y-4">
        {#if !isMac}
          {@render traySwitch(
            'Minimize to tray',
            $traySupport.available && !$traySupport.minimize
              ? 'Not on Wayland, which never tells an app its window was minimized.'
              : 'Minimizing hides the window; the tray icon brings it back.',
            $trayBehavior.minimizeToTray && $traySupport.minimize,
            $traySupport.minimize,
            () => trayBehavior.update({ minimizeToTray: !$trayBehavior.minimizeToTray })
          )}
        {/if}
        {@render traySwitch(
          isMac ? 'Close to the menu bar' : 'Close to tray',
          'Closing the window keeps OmnySSH running — terminals, transfers and tunnels stay connected. Quit from the icon.',
          $trayBehavior.closeToTray && $traySupport.available,
          $traySupport.available,
          () => trayBehavior.update({ closeToTray: !$trayBehavior.closeToTray })
        )}
        {#if !$traySupport.available}
          <p class="text-xs text-status-warn">
            This desktop has no system tray, so the window closes and minimizes as usual.
          </p>
        {:else if $trayBehavior.minimizeToTray || $trayBehavior.closeToTray}
          <!-- Some desktops keep tray icons out of sight (GNOME without the AppIndicator
               extension, an overflow menu); a second launch always finds the window. -->
          <p class="text-xs text-muted">
            No icon in sight? Opening OmnySSH again brings the window back.
          </p>
        {/if}
        {@render traySwitch(
          'Ask before quitting',
          'When terminals, transfers, tunnels or snippet runs are still open.',
          $confirmQuit,
          true,
          () => confirmQuit.toggle()
        )}
      </div>
    </Surface>

    <!-- About: selectable, so the version can be pasted into a bug report -->
    {#if version}
      <Surface class="p-5">
        <h2 class="mb-3 text-sm font-semibold">About</h2>
        <div class="flex items-center justify-between gap-4">
          <p class="text-sm">Version</p>
          <p class="select-text text-sm tabular-nums text-muted">{version}</p>
        </div>
      </Surface>
    {/if}
  </div>
</section>

{#snippet traySwitch(
  label: string,
  hint: string,
  on: boolean,
  enabled: boolean,
  toggle: () => void
)}
  <div class="flex items-center justify-between gap-4">
    <div class="min-w-0">
      <p class="text-sm">{label}</p>
      <p class="text-xs text-muted">{hint}</p>
    </div>
    <button
      type="button"
      role="switch"
      aria-checked={on}
      aria-label={label}
      disabled={!enabled}
      onclick={toggle}
      class="relative h-6 w-11 shrink-0 rounded-full transition disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus {on
        ? 'bg-accent'
        : 'bg-surface-inset'}"
    >
      <span
        class="absolute top-0.5 h-5 w-5 rounded-full bg-surface shadow-soft transition-[left] {on
          ? 'left-[1.375rem]'
          : 'left-0.5'}"
      ></span>
    </button>
  </div>
{/snippet}
