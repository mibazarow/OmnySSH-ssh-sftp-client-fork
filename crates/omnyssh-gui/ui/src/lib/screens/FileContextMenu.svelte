<script lang="ts">
  // A right-click menu over an SFTP row (tech-gui.md §3.2). Positioned at the pointer,
  // kept inside the window, and dismissed by anything else — a press anywhere, Escape,
  // a resize, or the window losing focus.
  import { onMount } from 'svelte';
  import { Icon } from '$lib/theme';
  import type { MenuItem } from './fileMenu';

  let {
    x,
    y,
    items,
    onClose
  }: { x: number; y: number; items: MenuItem[]; onClose: () => void } = $props();

  let root = $state<HTMLDivElement>();

  // Focus the menu itself so Escape reaches it and the window handlers do not
  // steal the first key from the row that opened it.
  onMount(() => root?.focus());

  const WIDTH = 208;
  const ITEM_HEIGHT = 34;
  const left = $derived(Math.max(8, Math.min(x, window.innerWidth - WIDTH - 8)));
  const top = $derived(
    Math.max(8, Math.min(y, window.innerHeight - items.length * ITEM_HEIGHT - 16))
  );

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape') onClose();
  }
</script>

<svelte:window
  onpointerdown={onClose}
  onkeydown={onKeydown}
  onresize={onClose}
  onblur={onClose}
/>

<div
  bind:this={root}
  role="menu"
  aria-label="File actions"
  tabindex="-1"
  class="fixed z-[60] overflow-hidden rounded-xl border border-default bg-surface-raised py-1 shadow-soft"
  style="left: {left}px; top: {top}px; width: {WIDTH}px;"
  onpointerdown={(e) => e.stopPropagation()}
  oncontextmenu={(e) => e.preventDefault()}
>
  {#each items as item (item.label)}
    <button
      type="button"
      role="menuitem"
      class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-sm transition
        {item.danger
        ? 'text-status-crit hover:bg-surface-inset'
        : 'text-muted hover:bg-surface-inset hover:text-fg'}"
      onclick={() => {
        onClose();
        item.run();
      }}
    >
      <Icon name={item.icon} size={14} />
      <span class="truncate">{item.label}</span>
    </button>
  {/each}
</div>
