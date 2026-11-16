<script>
  import { tick } from 'svelte';
  import { api } from './api.js';
  import { className, cssReference, displayValue, groupLabel, isColorLike, readUtilPrefs, utilFor } from './tokens.js';

  let { projects, settings, scanning, indexing, onopen, onpin, onrescan, onsettings, notify } = $props();

  // Cross-project token search (custom tokens of every indexed config).
  let hits = $state([]);
  let searchTimer;
  $effect(() => {
    const q = query.trim();
    indexing.done; // search again as the index fills up
    clearTimeout(searchTimer);
    if (q.length < 2) {
      hits = [];
      return;
    }
    searchTimer = setTimeout(async () => {
      const result = await api.searchTokens(q);
      if (query.trim() === q) hits = result;
    }, 120);
  });

  async function copyHit(hit, e) {
    const theme = { kind: hit.kind, prefix: hit.prefix };
    let text;
    if (e.altKey) text = displayValue(hit.token);
    else if (e.shiftKey) text = cssReference(theme, hit.group, hit.token);
    else text = className(theme, hit.group, utilFor(readUtilPrefs(), hit.group), hit.token) ?? cssReference(theme, hit.group, hit.token);
    await api.copy(text);
    notify(`Copié · ${text}`);
  }

  function openHitProject(hit) {
    const project = projects.find((p) => p.path === hit.projectPath);
    if (project) onopen(project);
  }

  let query = $state('');
  let selected = $state(0);
  let input;
  let listEl;

  export async function focus() {
    await tick();
    input?.focus();
    input?.select();
  }

  function location(p) {
    const rel = p.path.startsWith(p.root) ? p.path.slice(p.root.length + 1) : p.path;
    const parent = rel.split('/').slice(0, -1).join('/');
    const rootName = p.root.split('/').filter(Boolean).pop();
    return parent ? `${rootName}/${parent}` : rootName;
  }

  function score(p, q) {
    const name = p.name.toLowerCase();
    if (name.startsWith(q)) return 3;
    if (name.includes(q)) return 2;
    // Loose subsequence match ("lrq" → louis-roque).
    let i = 0;
    for (const ch of name) if (ch === q[i]) i++;
    if (i === q.length) return 1;
    return p.path.toLowerCase().includes(q) ? 0.5 : 0;
  }

  const visible = $derived(projects.filter((p) => !settings.hidden.includes(p.path)));

  const sections = $derived.by(() => {
    let i = 0;
    return rawSections().map((sec) => ({ ...sec, items: sec.items.map((project) => ({ project, index: i++ })) }));
  });

  function rawSections() {
    const q = query.trim().toLowerCase();
    if (q) {
      const hits = visible
        .map((p) => ({ p, s: score(p, q) }))
        .filter((x) => x.s > 0)
        .sort((a, b) => b.s - a.s || a.p.name.localeCompare(b.p.name))
        .map((x) => x.p);
      return [{ title: `${hits.length} résultat${hits.length > 1 ? 's' : ''}`, items: hits }];
    }
    const byPath = new Map(visible.map((p) => [p.path, p]));
    const pinned = settings.pinned.map((path) => byPath.get(path)).filter(Boolean);
    const recent = settings.recent
      .filter((path) => !settings.pinned.includes(path))
      .map((path) => byPath.get(path))
      .filter(Boolean)
      .slice(0, 4);
    const out = [];
    if (pinned.length) out.push({ title: 'Épinglés', items: pinned });
    if (recent.length) out.push({ title: 'Récents', items: recent });
    out.push({ title: `Tous les projets · ${visible.length}`, items: visible });
    return out;
  }

  const flat = $derived(sections.flatMap((s) => s.items.map((it) => it.project)));

  $effect(() => {
    query;
    selected = 0;
  });

  async function move(delta) {
    if (!flat.length) return;
    selected = (selected + delta + flat.length) % flat.length;
    await tick();
    listEl?.querySelector('[data-selected="true"]')?.scrollIntoView({ block: 'nearest' });
  }

  function onkeydown(e) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      move(1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      move(-1);
    } else if (e.key === 'Enter' && flat[selected]) {
      e.preventDefault();
      onopen(flat[selected]);
    }
  }

  $effect(() => {
    focus();
  });
</script>

<header class="flex items-center gap-2 border-b border-line bg-head px-3 py-2.5">
  <div class="relative flex-1">
    <svg class="absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted" viewBox="0 0 20 20" fill="currentColor"
      ><path
        fill-rule="evenodd"
        d="M9 3.5a5.5 5.5 0 1 0 0 11 5.5 5.5 0 0 0 0-11ZM2 9a7 7 0 1 1 12.45 4.39l3.08 3.08a.75.75 0 1 1-1.06 1.06l-3.08-3.08A7 7 0 0 1 2 9Z"
        clip-rule="evenodd"
      /></svg
    >
    <input
      bind:this={input}
      bind:value={query}
      {onkeydown}
      placeholder="Projet ou token (couleur, nom, valeur…)"
      spellcheck="false"
      class="w-full rounded-lg bg-panel-2 py-1.5 pr-2 pl-8 outline-none placeholder:text-muted focus:ring-2 focus:ring-accent/40"
    />
  </div>
  <button
    title="Rescanner"
    onclick={onrescan}
    class="grid size-7 place-items-center rounded-md text-muted hover:bg-panel-2 hover:text-ink"
  >
    <svg class="size-4 {scanning ? 'animate-spin' : ''}" viewBox="0 0 20 20" fill="currentColor"
      ><path
        fill-rule="evenodd"
        d="M15.31 11.42a5.5 5.5 0 0 1-9.2 2.47l-.82-.82h2.43a.75.75 0 0 0 0-1.5H3.53a.75.75 0 0 0-.75.75v4.19a.75.75 0 0 0 1.5 0v-2.43l.82.82a7 7 0 0 0 11.71-3.14.75.75 0 1 0-1.45-.39Zm-10.62-2.84a5.5 5.5 0 0 1 9.2-2.47l.82.82h-2.43a.75.75 0 0 0 0 1.5h4.19a.75.75 0 0 0 .75-.75V3.49a.75.75 0 0 0-1.5 0v2.43l-.82-.82A7 7 0 0 0 3.24 8.24a.75.75 0 1 0 1.45.39Z"
        clip-rule="evenodd"
      /></svg
    >
  </button>
  <button
    title="Réglages"
    onclick={onsettings}
    class="grid size-7 place-items-center rounded-md text-muted hover:bg-panel-2 hover:text-ink"
  >
    <svg class="size-4" viewBox="0 0 20 20" fill="currentColor"
      ><path
        fill-rule="evenodd"
        d="M7.84 1.8A1 1 0 0 1 8.82 1h2.36a1 1 0 0 1 .98.8l.33 1.67c.47.2.91.45 1.32.75l1.6-.55a1 1 0 0 1 1.19.45l1.18 2.04a1 1 0 0 1-.21 1.25l-1.27 1.12a6.1 6.1 0 0 1 0 1.52l1.27 1.12a1 1 0 0 1 .2 1.25l-1.17 2.04a1 1 0 0 1-1.19.45l-1.6-.55c-.41.3-.85.55-1.32.75l-.33 1.67a1 1 0 0 1-.98.8H8.82a1 1 0 0 1-.98-.8l-.33-1.67a6.5 6.5 0 0 1-1.32-.75l-1.6.55a1 1 0 0 1-1.19-.45L2.22 13.4a1 1 0 0 1 .21-1.25l1.27-1.12a6.1 6.1 0 0 1 0-1.52L2.43 8.39a1 1 0 0 1-.2-1.25l1.17-2.04a1 1 0 0 1 1.19-.45l1.6.55c.41-.3.85-.55 1.32-.75l.33-1.67ZM10 13a3 3 0 1 0 0-6 3 3 0 0 0 0 6Z"
        clip-rule="evenodd"
      /></svg
    >
  </button>
</header>

<div bind:this={listEl} class="scroll-thin flex-1 overflow-y-auto px-1.5 pb-2">
  {#if !projects.length}
    <div class="px-4 py-10 text-center text-muted">
      {scanning ? 'Scan des projets…' : 'Aucun projet Tailwind trouvé. Ajoute des dossiers dans les réglages.'}
    </div>
  {/if}

  {#each sections as section (section.title)}
    <div class="sticky top-0 z-10 bg-panel px-2.5 pt-3 pb-1 text-[11px] font-medium tracking-wide text-muted uppercase">
      {section.title}
    </div>
    {#each section.items as { project, index } (section.title + project.path)}
      {@const isSelected = index === selected}
      <div
        role="button"
        tabindex="-1"
        data-selected={isSelected}
        onclick={() => onopen(project)}
        onkeydown={() => {}}
        onmouseenter={() => (selected = index)}
        class="group flex items-center gap-2.5 rounded-lg px-2.5 py-1.5 {isSelected ? 'bg-accent-soft' : ''}"
      >
        <div class="min-w-0 flex-1">
          <div class="truncate font-medium">{project.name}</div>
          <div class="truncate text-[11px] text-muted">{location(project)}</div>
        </div>
        {#each [...new Set(project.configs.map((c) => c.kind))] as kind (kind)}
          <span
            class="rounded px-1.5 py-px font-mono text-[10px] {kind === 'v4'
              ? 'bg-sky-500/15 text-sky-600 dark:text-sky-400'
              : 'bg-violet-500/15 text-violet-600 dark:text-violet-400'}">{kind}</span
          >
        {/each}
        <button
          title={settings.pinned.includes(project.path) ? 'Désépingler' : 'Épingler'}
          onclick={(e) => {
            e.stopPropagation();
            onpin(project);
          }}
          class="grid size-6 place-items-center rounded text-muted hover:text-ink {settings.pinned.includes(project.path)
            ? 'text-amber-500!'
            : 'opacity-0 group-hover:opacity-100'}"
        >
          <svg class="size-3.5" viewBox="0 0 20 20" fill="currentColor"
            ><path
              fill-rule="evenodd"
              d="M10.87 2.6a1 1 0 0 0-1.74 0L7.2 6.47l-4.27.62a1 1 0 0 0-.55 1.7l3.09 3.02-.73 4.25a1 1 0 0 0 1.45 1.05L10 15.1l3.81 2.01a1 1 0 0 0 1.45-1.05l-.73-4.25 3.09-3.01a1 1 0 0 0-.55-1.71l-4.27-.62-1.93-3.87Z"
              clip-rule="evenodd"
            /></svg
          >
        </button>
      </div>
    {/each}
  {/each}

  {#if hits.length}
    <div class="sticky top-0 z-10 bg-panel px-2.5 pt-3 pb-1 text-[11px] font-medium tracking-wide text-muted uppercase">
      Tokens · {hits.length}{hits.length === 80 ? '+' : ''}
    </div>
    {#each hits as hit (hit.configPath + hit.group + hit.token.name)}
      {@const value = displayValue(hit.token)}
      {@const cls = className({ kind: hit.kind, prefix: hit.prefix }, hit.group, utilFor(readUtilPrefs(), hit.group), hit.token)}
      <div
        role="button"
        tabindex="-1"
        onclick={(e) => copyHit(hit, e)}
        onkeydown={() => {}}
        title="Clic : copier · ⌥ valeur · ⇧ {hit.kind === 'v4' ? 'var()' : 'theme()'}"
        class="group flex items-center gap-2.5 rounded-lg px-2.5 py-1.5 hover:bg-panel-2"
      >
        {#if isColorLike(value)}
          <span class="checker size-6 shrink-0 overflow-hidden rounded-md ring-1 ring-line"
            ><span class="block size-full" style:background={value}></span></span
          >
        {:else}
          <span class="grid size-6 shrink-0 place-items-center rounded-md bg-panel-2 text-[9px] text-muted uppercase"
            >{groupLabel(hit.group).slice(0, 2)}</span
          >
        {/if}
        <div class="min-w-0 flex-1">
          <div class="truncate font-mono text-[12px]">{cls ?? hit.token.varName ?? hit.token.name}</div>
          <div class="truncate text-[11px] text-muted"><span class="font-mono">{value}</span> · {hit.project}</div>
        </div>
        <button
          title="Ouvrir le projet"
          onclick={(e) => {
            e.stopPropagation();
            openHitProject(hit);
          }}
          class="grid size-6 shrink-0 place-items-center rounded text-muted opacity-0 group-hover:opacity-100 hover:bg-panel hover:text-ink"
        >
          <svg class="size-3.5" viewBox="0 0 20 20" fill="currentColor"
            ><path
              fill-rule="evenodd"
              d="M8.22 5.22a.75.75 0 0 1 1.06 0l4.25 4.25a.75.75 0 0 1 0 1.06l-4.25 4.25a.75.75 0 0 1-1.06-1.06L11.94 10 8.22 6.28a.75.75 0 0 1 0-1.06Z"
              clip-rule="evenodd"
            /></svg
          >
        </button>
      </div>
    {/each}
  {/if}
</div>

{#if indexing.total && indexing.done < indexing.total}
  <footer class="border-t border-line px-3 py-1.5 text-[10.5px] text-muted">
    Indexation des tokens… {indexing.done}/{indexing.total}
  </footer>
{/if}
