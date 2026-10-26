<script lang="ts">
  // A remote path's properties (tech-gui.md §3.2): its permission bits as three
  // r/w/x checkboxes per class, saved with one chmod. The setuid/setgid/sticky bits
  // are shown as a hint and sent back unchanged.
  import { Button, Icon } from '$lib/theme';
  import Modal from '$lib/components/Modal.svelte';
  import { maskText } from '$lib/stores/streamer';
  import { formatBytes, formatDate } from '$lib/stores/sftp';
  import type { FileMetaDto } from '$lib/bindings';
  import { granted, octal, specialBits, symbolic, withGrant, type Perm, type Who } from './fileMode';

  let {
    meta,
    loading,
    error,
    onSave,
    onClose
  }: {
    meta: FileMetaDto | null;
    loading: boolean;
    error?: string;
    onSave: (mode: number) => Promise<void>;
    onClose: () => void;
  } = $props();

  const ROWS: Array<{ who: Who; label: string }> = [
    { who: 'owner', label: 'Owner' },
    { who: 'group', label: 'Group' },
    { who: 'others', label: 'Others' }
  ];
  const PERMS: Array<{ perm: Perm; label: string }> = [
    { perm: 'read', label: 'Read' },
    { perm: 'write', label: 'Write' },
    { perm: 'execute', label: 'Execute' }
  ];

  let mode = $state(0);
  let saving = $state(false);
  let saveError = $state<string | undefined>(undefined);

  // The server's bits, or a sensible default when it sent none to show.
  $effect(() => {
    if (meta) mode = meta.mode ?? (meta.isDir ? 0o755 : 0o644);
  });

  const name = $derived(meta ? meta.path.slice(meta.path.lastIndexOf('/') + 1) || meta.path : '');
  const special = $derived(specialBits(mode));

  async function save(): Promise<void> {
    if (!meta || saving) return;
    saving = true;
    saveError = undefined;
    try {
      await onSave(mode);
    } catch (e) {
      saveError = e instanceof Error ? e.message : String(e);
    } finally {
      saving = false;
    }
  }
</script>

<Modal label="Properties of {name}" onClose={onClose}>
  <header class="flex items-center gap-3 border-b border-default px-5 py-3.5">
    <Icon name={meta?.isDir ? 'folder' : 'file'} size={15} />
    <div class="min-w-0 flex-1">
      <h2 class="truncate font-mono text-sm text-fg" title={meta?.path}>{name}</h2>
      <p class="truncate font-mono text-[0.65rem] text-faint" title={meta?.path}>{meta?.path}</p>
    </div>
  </header>

  <div class="min-h-0 flex-1 overflow-y-auto px-5 py-4">
    {#if loading}
      <p class="py-6 text-center text-sm text-faint">Reading the file's details…</p>
    {:else if !meta}
      <p class="py-6 text-center text-sm text-status-crit">{$maskText(error ?? 'Not available.')}</p>
    {:else}
      <dl class="mb-4 grid grid-cols-[7rem_1fr] gap-x-3 gap-y-1.5 text-sm">
        <dt class="text-muted">Kind</dt>
        <dd>{meta.isDir ? 'Folder' : 'File'}</dd>
        <dt class="text-muted">Size</dt>
        <dd class="tabular-nums">{formatBytes(meta.size)}</dd>
        <dt class="text-muted">Modified</dt>
        <dd>{formatDate(meta.modified, true)}</dd>
        {#if meta.uid != null || meta.gid != null}
          <dt class="text-muted">Owner</dt>
          <dd class="font-mono text-xs">
            {meta.uid != null ? `uid ${meta.uid}` : 'uid ?'}{meta.gid != null
              ? ` · gid ${meta.gid}`
              : ''}
          </dd>
        {/if}
      </dl>

      <h3 class="mb-2 text-sm font-semibold">Permissions</h3>
      {#if meta.mode == null}
        <p class="mb-3 text-xs text-status-warn">
          The server did not report this file's permissions. Setting them still works; the
          boxes start from a default.
        </p>
      {/if}
      <table class="w-full max-w-sm text-sm">
        <thead>
          <tr class="text-[0.65rem] uppercase tracking-[0.12em] text-faint">
            <th class="pb-1.5 text-left font-semibold">Class</th>
            {#each PERMS as column (column.perm)}
              <th class="pb-1.5 text-center font-semibold">{column.label}</th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each ROWS as row (row.who)}
            <tr class="border-t border-default">
              <th class="py-2 text-left font-medium text-muted">{row.label}</th>
              {#each PERMS as column (column.perm)}
                <td class="py-2 text-center">
                  <input
                    type="checkbox"
                    class="h-4 w-4 accent-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
                    checked={granted(mode, row.who, column.perm)}
                    aria-label="{row.label} {column.label}"
                    onchange={(e) =>
                      (mode = withGrant(mode, row.who, column.perm, e.currentTarget.checked))}
                  />
                </td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>

      <p class="mt-3 flex items-center gap-2 text-xs text-muted">
        <span class="font-mono text-fg">{octal(mode)}</span>
        <span class="font-mono">{symbolic(mode)}</span>
      </p>
      {#if special !== 0}
        <p class="mt-1 text-[0.65rem] text-faint">
          The setuid/setgid/sticky bits are kept as they are.
        </p>
      {/if}
    {/if}
  </div>

  {#if saveError}
    <p class="shrink-0 border-t border-default px-5 py-2 text-xs text-status-crit">
      {$maskText(saveError)}
    </p>
  {/if}

  <footer class="flex shrink-0 justify-end gap-2 border-t border-default px-5 py-3">
    <Button variant="ghost" onclick={onClose}>Cancel</Button>
    <Button variant="primary" disabled={!meta || loading || saving} onclick={() => void save()}>
      {saving ? 'Saving…' : 'Save'}
    </Button>
  </footer>
</Modal>
