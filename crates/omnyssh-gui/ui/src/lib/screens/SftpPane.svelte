<script lang="ts">
  // One side of the dual-pane SFTP browser (tech-gui.md §3.2): a current-path header
  // with a parent-supplied toolbar, then the entry list. Clicking a directory (or the
  // `..` row) navigates; clicking a file previews; Shift-click selects a range and
  // Cmd/Ctrl-click toggles one row (the parent owns both). The leading checkbox marks
  // a row for a batch transfer/delete. A press on a row may start a drag (the parent
  // owns the drag and marks the pane `dropActive` while it is the target); `data-pane`
  // and `data-dir-path` are how the parent finds the pane and folder under the pointer.
  // A listing on its way dims the current one: a remote one can wait behind a transfer.
  // Semantic tokens only — no colour literals (§5.1).
  import { onDestroy } from 'svelte';
  import type { Snippet } from 'svelte';
  import { Icon } from '$lib/theme';
  import type { FileEntryDto } from '$lib/bindings';
  import {
    formatBytes,
    formatDate,
    pathTrail,
    type Pane,
    type PaneSide
  } from '$lib/stores/sftp';
  import { lastError } from '$lib/stores/notifications';
  import { maskText } from '$lib/stores/streamer';

  let {
    title,
    side,
    pane,
    dropActive = false,
    dropDir,
    onNavigatePath,
    onToggleMark,
    onRowClick,
    onDragStart,
    onContextMenu,
    toolbar
  }: {
    title: string;
    side: PaneSide;
    pane: Pane;
    /** The pane is the target of a drag in progress. */
    dropActive?: boolean;
    /** The folder row under the pointer while this pane is the drop target. */
    dropDir?: string;
    /** Open a directory path from the breadcrumb trail. */
    onNavigatePath: (path: string) => void;
    /** A checkbox click; the event's modifiers make it a range or a toggle. */
    onToggleMark: (path: string, event: MouseEvent) => void;
    /** A row click: opens it, or selects it with Shift / Cmd held. */
    onRowClick: (entry: FileEntryDto, event: MouseEvent) => void;
    onDragStart?: (entry: FileEntryDto, event: PointerEvent) => void;
    /** Right-click on a row: the parent opens its actions menu at the pointer. */
    onContextMenu?: (entry: FileEntryDto, event: MouseEvent) => void;
    toolbar?: Snippet;
  } = $props();

  const trail = $derived(pathTrail(pane.path));

  let copied = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | undefined;

  onDestroy(() => {
    if (copyTimer !== undefined) clearTimeout(copyTimer);
  });

  async function copyPath(): Promise<void> {
    if (!pane.path) return;
    try {
      await navigator.clipboard.writeText(pane.path);
    } catch (err) {
      lastError.set(`Copy failed: ${err instanceof Error ? err.message : String(err)}`);
      return;
    }
    copied = true;
    if (copyTimer !== undefined) clearTimeout(copyTimer);
    copyTimer = setTimeout(() => (copied = false), 1200);
  }

  const rowBase =
    'flex w-full min-w-0 items-center gap-2 rounded px-2 py-1.5 text-left text-sm transition ' +
    'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus';
  // The date column appears once the pane is wide enough to keep names readable. It
  // never wraps: under GDK_DPI_SCALE the rem it is sized in can shrink below its text.
  const modifiedCol = 'hidden min-w-36 shrink-0 whitespace-nowrap text-right lg:inline';
</script>

<section
  aria-label={title}
  aria-busy={pane.loading}
  data-pane={side}
  class="flex min-h-0 min-w-0 flex-1 flex-col transition
    {dropActive ? 'bg-surface-inset ring-2 ring-inset ring-accent' : ''}"
>
  <header class="shrink-0 border-b border-default px-3 py-2.5">
    <div class="flex items-center justify-between gap-2">
      <h2
        title={title}
        class="truncate text-xs font-semibold uppercase tracking-[0.14em] text-muted"
      >
        {title}
      </h2>
      <div class="flex shrink-0 items-center gap-1">
        {@render toolbar?.()}
      </div>
    </div>
    <div class="mt-1 flex items-center gap-2 text-xs text-faint">
      {#if pane.path}
        <nav
          class="flex min-w-0 flex-1 flex-wrap items-center gap-x-1 gap-y-0.5"
          aria-label="Current path"
          title={pane.path}
        >
          {#each trail as crumb, i (crumb.path)}
            {#if i > 0}
              <span class="shrink-0" aria-hidden="true">›</span>
            {/if}
            {#if i === trail.length - 1}
              <span class="font-mono font-medium text-muted" aria-current="page">{crumb.label}</span>
            {:else}
              <button
                type="button"
                class="rounded px-1 py-0.5 font-mono transition hover:bg-surface-inset hover:text-fg
                  focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
                title={crumb.path}
                onclick={() => onNavigatePath(crumb.path)}
              >
                {crumb.label}
              </button>
            {/if}
          {/each}
        </nav>
        <button
          type="button"
          class="shrink-0 rounded p-1 transition hover:bg-surface-inset hover:text-fg
            focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus
            {copied ? 'text-status-ok' : ''}"
          title={copied ? 'Copied' : 'Copy path'}
          aria-label="Copy current path"
          onclick={() => void copyPath()}
        >
          <Icon name={copied ? 'check' : 'copy'} size={12} />
        </button>
      {:else}
        <span>—</span>
      {/if}
      {#if pane.loading && pane.entries.length > 0}
        <span class="shrink-0">Loading…</span>
      {/if}
    </div>
  </header>

  <div class="min-h-0 flex-1 overflow-y-auto px-1.5 py-1.5">
    {#if pane.error}
      <p class="px-2 py-6 text-center text-sm text-status-crit">{$maskText(pane.error)}</p>
    {:else if pane.loading && pane.entries.length === 0}
      <p class="px-2 py-6 text-center text-sm text-faint">Loading…</p>
    {:else if pane.entries.length === 0}
      <p class="px-2 py-6 text-center text-sm text-faint">Empty directory</p>
    {:else}
      <div
        class="flex items-center gap-2 px-2 pb-1 pl-[2.125rem] text-[0.65rem] font-semibold uppercase
          tracking-[0.12em] text-faint"
        aria-hidden="true"
      >
        <span class="min-w-0 flex-1 pl-[1.4rem]">Name</span>
        <span class="shrink-0">Size</span>
        <span class={modifiedCol}>Modified</span>
      </div>
      <ul class="space-y-0.5 transition-opacity {pane.loading ? 'opacity-50' : ''}">
        {#each pane.entries as entry, i (i)}
          {@const isParent = entry.name === '..'}
          {@const marked = pane.marked.has(entry.path)}
          {@const dropHere = dropActive && entry.isDir && dropDir === entry.path}
          <li
            class="flex items-center gap-1.5 rounded transition
              {dropHere ? 'ring-2 ring-accent' : ''}
              {marked ? 'bg-surface-inset ring-1 ring-inset ring-accent' : ''}"
            data-dir-path={entry.isDir ? entry.path : undefined}
          >
            {#if isParent}
              <span class="h-4 w-4 shrink-0"></span>
            {:else}
              <button
                type="button"
                role="checkbox"
                aria-checked={marked}
                aria-label="Mark {entry.name}"
                class="grid h-4 w-4 shrink-0 place-items-center rounded border transition
                  focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus
                  {marked ? 'border-accent bg-accent text-accent-fg' : 'border-strong text-transparent'}"
                onclick={(e) => onToggleMark(entry.path, e)}
              >
                {#if marked}<Icon name="check" size={11} />{/if}
              </button>
            {/if}
            <button
              type="button"
              class="{rowBase} text-muted hover:bg-surface-inset hover:text-fg"
              title={entry.name}
              onclick={(e) => onRowClick(entry, e)}
              onpointerdown={(e) => onDragStart?.(entry, e)}
              oncontextmenu={(e) => {
                e.preventDefault();
                e.stopPropagation();
                onContextMenu?.(entry, e);
              }}
            >
              <Icon name={entry.isDir ? 'folder' : 'file'} size={15} />
              <span class="min-w-0 flex-1 truncate {entry.isDir ? 'font-medium text-fg' : ''}">
                {entry.name}
              </span>
              {#if !entry.isDir}
                <span class="shrink-0 tabular-nums text-xs text-faint">{formatBytes(entry.size)}</span>
              {/if}
              {#if !isParent}
                <span
                  class="{modifiedCol} tabular-nums text-xs text-faint"
                  title={entry.modified != null ? `Modified ${formatDate(entry.modified, true)}` : undefined}
                >
                  {formatDate(entry.modified)}
                </span>
              {/if}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</section>
