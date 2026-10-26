<script lang="ts">
  // A centered dialog over a scrim (tech-gui.md §2.2 forms/results). Same overlay
  // language as the command palette — role="dialog", Escape to dismiss, backdrop
  // click to close — but a solid raised surface for readable forms. Content is
  // passed in; the chrome (scrim, box, key handling) is fixed here so the snippet
  // dialogs don't each re-implement it.
  import { onDestroy, type Snippet } from 'svelte';
  import { get } from 'svelte/store';
  import { dialogs } from '$lib/stores/dialogs';

  let {
    label,
    onClose,
    backdropCloses = true,
    size = 'md',
    tall = false,
    children
  }: {
    label: string;
    onClose: () => void;
    backdropCloses?: boolean;
    /** `md` for a form or result, `xl` for the file editor's textarea. */
    size?: 'md' | 'xl';
    /** A tall dialog takes 90% of the window's height, so its content can flex to it. */
    tall?: boolean;
    children: Snippet;
  } = $props();

  // Dialogs can stack (the passphrase prompt opens on its own over any other):
  // Escape closes only the top one, and the one opened last is drawn on top.
  const id = Symbol('dialog');
  dialogs.update((open) => [...open, id]);
  onDestroy(() => dialogs.update((open) => open.filter((d) => d !== id)));
  const layer = $derived(50 + $dialogs.indexOf(id));

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape' && get(dialogs).at(-1) === id) {
      e.preventDefault();
      onClose();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div
  class="fixed inset-0 flex justify-center px-4
    {tall ? 'items-center py-[5vh]' : 'items-start pt-[12vh]'}"
  style="z-index: {layer};"
  role="dialog"
  aria-modal="true"
  aria-label={label}
>
  <button
    type="button"
    tabindex="-1"
    aria-label="Dismiss"
    class="absolute inset-0 bg-overlay"
    onclick={() => backdropCloses && onClose()}
  ></button>

  <div
    class="relative flex w-full flex-col overflow-hidden rounded-2xl border border-default bg-surface-raised shadow-soft
      {size === 'xl' ? 'max-w-5xl' : 'max-w-lg'} {tall ? 'h-full' : 'max-h-[76vh]'}"
  >
    {@render children()}
  </div>
</div>
