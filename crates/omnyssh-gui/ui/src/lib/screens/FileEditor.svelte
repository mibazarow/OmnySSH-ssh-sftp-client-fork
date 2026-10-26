<script lang="ts">
  // The remote file editor (tech-gui.md §3.2): the whole file in a textarea, saved
  // straight back over SFTP. `onSave` rejects with the server's reason; the dialog
  // stays open on failure so nothing typed is lost. Cmd/Ctrl+S saves, Tab indents.
  import { onMount } from 'svelte';
  import { Button, Icon } from '$lib/theme';
  import Modal from '$lib/components/Modal.svelte';
  import { maskText } from '$lib/stores/streamer';
  import { octal } from './fileMode';

  let {
    path,
    content,
    mode,
    onSave,
    onClose
  }: {
    path: string;
    content: string;
    mode?: number;
    onSave: (content: string) => Promise<void>;
    onClose: () => void;
  } = $props();

  // The editor is mounted per file: the props are the starting point, not a
  // value that follows the parent.
  // svelte-ignore state_referenced_locally
  let value = $state(content);
  // svelte-ignore state_referenced_locally
  let saved = $state(content);
  let saving = $state(false);
  let error = $state<string | undefined>(undefined);
  let confirming = $state(false);
  let area = $state<HTMLTextAreaElement>();

  const dirty = $derived(value !== saved);
  const name = $derived(path.slice(path.lastIndexOf('/') + 1) || path);

  onMount(() => area?.focus());

  async function save(): Promise<void> {
    if (!dirty || saving) return;
    saving = true;
    error = undefined;
    try {
      await onSave(value);
      saved = value;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      saving = false;
    }
  }

  function requestClose(): void {
    if (dirty) confirming = true;
    else onClose();
  }

  function onSaveShortcut(e: KeyboardEvent): void {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 's') {
      e.preventDefault();
      void save();
    }
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key !== 'Tab') return;
    // Keep the keyboard in the editor: Tab indents instead of moving focus.
    e.preventDefault();
    const el = e.currentTarget as HTMLTextAreaElement;
    const start = el.selectionStart;
    const end = el.selectionEnd;
    value = `${value.slice(0, start)}  ${value.slice(end)}`;
    queueMicrotask(() => el.setSelectionRange(start + 2, start + 2));
  }
</script>

<svelte:window onkeydown={onSaveShortcut} />

<Modal label="Edit {name}" onClose={requestClose} size="xl" tall>
  <header class="flex shrink-0 items-center gap-3 border-b border-default px-5 py-3">
    <Icon name="file" size={15} />
    <div class="min-w-0 flex-1">
      <h2 class="truncate font-mono text-sm text-fg" title={path}>{name}</h2>
      <p class="truncate font-mono text-[0.65rem] text-faint" title={path}>{path}</p>
    </div>
    {#if mode != null}
      <span class="shrink-0 rounded-full border border-default px-2 py-0.5 font-mono text-[0.65rem] text-muted">
        {octal(mode)}
      </span>
    {/if}
    {#if dirty}
      <span class="shrink-0 text-[0.65rem] font-medium text-status-warn">unsaved</span>
    {/if}
  </header>

  <div class="flex min-h-0 flex-1 flex-col border-b border-default">
    <textarea
      bind:this={area}
      bind:value
      onkeydown={onKeydown}
      class="h-full min-h-0 w-full flex-1 resize-none overflow-auto whitespace-pre bg-surface-inset
        px-4 py-3 font-mono text-xs leading-relaxed text-fg outline-none placeholder:text-faint"
      wrap="off"
      spellcheck="false"
      autocapitalize="off"
      autocomplete="off"
      aria-label="File contents"
      placeholder="Empty file."
    ></textarea>
  </div>

  {#if error}
    <p class="shrink-0 border-b border-default px-5 py-2 text-xs text-status-crit">
      {$maskText(error)}
    </p>
  {/if}

  <footer class="flex shrink-0 items-center justify-between gap-3 px-5 py-3">
    {#if confirming}
      <p class="text-sm text-muted">Discard the unsaved changes?</p>
      <div class="flex shrink-0 gap-2">
        <Button variant="ghost" onclick={() => (confirming = false)}>Keep editing</Button>
        <Button variant="primary" onclick={onClose}>Discard</Button>
      </div>
    {:else}
      <p class="text-xs text-faint">
        {saving ? 'Saving…' : dirty ? '⌘S to save' : 'All changes saved'}
      </p>
      <div class="flex shrink-0 gap-2">
        <Button variant="ghost" onclick={requestClose}>Close</Button>
        <Button variant="primary" disabled={!dirty || saving} onclick={() => void save()}>
          {saving ? 'Saving…' : 'Save'}
        </Button>
      </div>
    {/if}
  </footer>
</Modal>
