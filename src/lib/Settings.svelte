<script>
  import { onMount } from 'svelte';
  import { api } from './api.js';

  let { settings, projects, onsave, onback, onrescan, notify } = $props();

  let detectedNode = $state('');
  // svelte-ignore state_referenced_locally
  let nodePath = $state(settings.nodePath ?? '');
  // svelte-ignore state_referenced_locally
  let shortcut = $state(settings.shortcut);

  onMount(async () => {
    detectedNode = (await api.detectedNode()) ?? '';
  });

  const hiddenProjects = $derived(
    settings.hidden.map((path) => projects.find((p) => p.path === path) ?? { name: path.split('/').pop(), path }),
  );

  async function addRoot() {
    const dir = await api.pickFolder();
    if (dir && !settings.roots.includes(dir)) {
      await onsave({ ...settings, roots: [...settings.roots, dir] });
      notify('Dossier ajouté — pense à rescanner');
    }
  }

  function removeRoot(root) {
    onsave({ ...settings, roots: settings.roots.filter((r) => r !== root) });
  }

  function unhide(path) {
    onsave({ ...settings, hidden: settings.hidden.filter((p) => p !== path) });
  }

  async function saveAdvanced() {
    await onsave({ ...settings, nodePath: nodePath.trim() || null, shortcut: shortcut.trim() });
    notify('Réglages enregistrés');
  }
</script>

<header class="flex items-center gap-1.5 border-b border-line px-3 py-2.5">
  <button
    title="Retour"
    onclick={onback}
    class="grid size-7 place-items-center rounded-md text-muted hover:bg-panel-2 hover:text-ink"
  >
    <svg class="size-4" viewBox="0 0 20 20" fill="currentColor"
      ><path
        fill-rule="evenodd"
        d="M11.78 5.22a.75.75 0 0 1 0 1.06L8.06 10l3.72 3.72a.75.75 0 1 1-1.06 1.06l-4.25-4.25a.75.75 0 0 1 0-1.06l4.25-4.25a.75.75 0 0 1 1.06 0Z"
        clip-rule="evenodd"
      /></svg
    >
  </button>
  <div class="font-semibold">Réglages</div>
</header>

<div class="scroll-thin flex-1 space-y-5 overflow-y-auto p-3">
  <section>
    <h3 class="mb-1.5 text-[11px] font-medium tracking-wide text-muted uppercase">Dossiers scannés</h3>
    <div class="divide-y divide-line rounded-lg border border-line">
      {#each settings.roots as root (root)}
        <div class="flex items-center gap-2 px-2.5 py-1.5">
          <span class="min-w-0 flex-1 truncate font-mono text-[12px]">{root}</span>
          <button class="text-[11px] text-muted hover:text-red-500" onclick={() => removeRoot(root)}>Retirer</button>
        </div>
      {:else}
        <div class="px-2.5 py-2 text-muted">Aucun dossier.</div>
      {/each}
    </div>
    <div class="mt-2 flex gap-2">
      <button class="rounded-md bg-panel-2 px-2.5 py-1 hover:bg-accent-soft" onclick={addRoot}>Ajouter un dossier…</button>
      <button class="rounded-md bg-panel-2 px-2.5 py-1 hover:bg-accent-soft" onclick={onrescan}>Rescanner</button>
    </div>
  </section>

  <section class="space-y-3">
    <h3 class="text-[11px] font-medium tracking-wide text-muted uppercase">Avancé</h3>
    <label class="block">
      <span class="mb-1 block text-muted">Raccourci global</span>
      <input
        bind:value={shortcut}
        spellcheck="false"
        placeholder="Alt+Super+T"
        class="w-full rounded-md border border-line bg-panel-2 px-2 py-1 font-mono text-[12px] outline-none focus:ring-2 focus:ring-accent/40"
      />
      <span class="mt-1 block text-[11px] text-muted">Format : Alt / Shift / Control / Super (⌘) + touche. Ex. <code>Alt+Super+T</code></span>
    </label>
    <label class="block">
      <span class="mb-1 block text-muted">Chemin de node (configs v3)</span>
      <input
        bind:value={nodePath}
        spellcheck="false"
        placeholder={detectedNode || 'auto'}
        class="w-full rounded-md border border-line bg-panel-2 px-2 py-1 font-mono text-[12px] outline-none focus:ring-2 focus:ring-accent/40"
      />
      <span class="mt-1 block text-[11px] text-muted">
        {detectedNode ? `Détecté : ${detectedNode}` : 'node non détecté automatiquement.'}
      </span>
    </label>
    <button class="rounded-md bg-accent px-3 py-1 font-medium text-white hover:opacity-90" onclick={saveAdvanced}>Enregistrer</button>
  </section>

  {#if hiddenProjects.length}
    <section>
      <h3 class="mb-1.5 text-[11px] font-medium tracking-wide text-muted uppercase">Projets masqués</h3>
      <div class="divide-y divide-line rounded-lg border border-line">
        {#each hiddenProjects as p (p.path)}
          <div class="flex items-center gap-2 px-2.5 py-1.5">
            <span class="min-w-0 flex-1 truncate" title={p.path}>{p.name}</span>
            <button class="text-[11px] text-accent hover:underline" onclick={() => unhide(p.path)}>Réafficher</button>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  <section class="flex items-center justify-between border-t border-line pt-3 text-[11px] text-muted">
    <span>TailTip 0.1.0</span>
    <button class="rounded-md px-2 py-1 hover:bg-panel-2 hover:text-red-500" onclick={() => api.quit()}>Quitter TailTip</button>
  </section>
</div>
