<script>
  import { tick } from 'svelte';
  import { api } from './api.js';
  import {
    UTILITIES,
    className,
    cssReference,
    displayValue,
    groupLabel,
    isColorLike,
    sortGroups,
    toPx,
    remToPx,
    annotateRem,
    readUtilPrefs,
    writeUtilPrefs,
    utilFor as pickUtil,
  } from './tokens.js';

  let { project, pinned, onback, onpin, onhide, notify } = $props();

  let configIndex = $state(0);
  let theme = $state(null);
  let loading = $state(false);
  let query = $state('');
  let customOnly = $state(true);
  let input;
  let scroller;

  let chosenUtil = $state(readUtilPrefs());

  // Typography utilities from the project's typography.css (if any).
  let tab = $state('tokens');
  let typo = $state([]);

  async function loadTypo() {
    const p = project.path;
    try {
      const files = await api.loadTypography(p);
      if (project.path === p) typo = files;
    } catch {
      if (project.path === p) typo = [];
    }
  }

  $effect(() => {
    project;
    tab = 'tokens';
    typo = [];
    loadTypo();
  });

  // Typography files are not watched: refresh when the popover is shown again.
  $effect(() => {
    let unlisten;
    let disposed = false;
    api.listen('popover-shown', () => loadTypo()).then((u) => (disposed ? u() : (unlisten = u)));
    return () => {
      disposed = true;
      unlisten?.();
    };
  });

  const typoCount = $derived(typo.reduce((n, f) => n + f.utilities.length, 0));

  const typoFiles = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return typo
      .map((f) => ({
        ...f,
        utilities: f.utilities.filter((u) => !q || u.name.toLowerCase().includes(q) || u.body.toLowerCase().includes(q)),
      }))
      .filter((f) => f.utilities.length || f.error);
  });

  /** v4 prefixes every utility (`tw:u-title`); v3 leaves custom classes alone. */
  function typoClass(name) {
    return theme?.kind === 'v4' && theme.prefix ? `${theme.prefix}:${name}` : name;
  }

  async function copyTypo(name) {
    const text = typoClass(name);
    await api.copy(text);
    notify(`Copié · ${text}`);
  }

  function relPath(path) {
    return path.startsWith(project.path + '/') ? path.slice(project.path.length + 1) : fileName(path);
  }

  function chooseUtil(group, util) {
    chosenUtil = { ...chosenUtil, [group]: util };
    writeUtilPrefs(chosenUtil);
  }

  export async function focus() {
    await tick();
    input?.focus();
  }

  const config = $derived(project.configs[configIndex] ?? project.configs[0]);

  async function load({ silent = false } = {}) {
    const c = config;
    if (!silent) {
      loading = true;
      theme = null;
    }
    try {
      const t = await api.loadTheme(c.path, c.kind);
      if (config === c) theme = t;
    } catch (e) {
      theme = { kind: c.kind, error: String(e), groups: [], prefix: '' };
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    config;
    load();
  });

  // Auto-refresh when the config (or a file it imports) changes on disk.
  $effect(() => {
    const path = config.path;
    let unlisten;
    let disposed = false;
    api
      .listen('config-changed', async (event) => {
        if (event.payload.includes(path)) {
          await load({ silent: true });
          notify('Config rechargée');
        }
      })
      .then((u) => (disposed ? u() : (unlisten = u)));
    return () => {
      disposed = true;
      unlisten?.();
    };
  });

  $effect(() => {
    project;
    configIndex = 0;
    query = '';
    focus();
  });

  const hasDefaults = $derived(theme?.groups.some((g) => g.tokens.some((t) => !t.custom)) ?? false);

  const groups = $derived.by(() => {
    if (!theme) return [];
    const q = query.trim().toLowerCase();
    return sortGroups(theme.groups)
      .map((g) => ({
        ...g,
        tokens: g.tokens.filter(
          (t) =>
            (!customOnly || t.custom || !hasDefaults) &&
            (!q || t.name.toLowerCase().includes(q) || displayValue(t).toLowerCase().includes(q) || g.key.toLowerCase().includes(q)),
        ),
      }))
      .filter((g) => g.tokens.length);
  });

  const tokenCount = $derived(groups.reduce((n, g) => n + g.tokens.length, 0));

  const utilFor = (groupKey) => pickUtil(chosenUtil, groupKey);

  async function copy(group, token, e) {
    let text;
    if (e.altKey) text = displayValue(token);
    else if (e.shiftKey) text = cssReference(theme, group.key, token);
    else text = className(theme, group.key, utilFor(group.key), token) ?? cssReference(theme, group.key, token);
    await api.copy(text);
    notify(`Copié · ${text}`);
  }

  function openToken(token) {
    api.openInEditor(token.file ?? config.path, token.line ?? null);
  }

  function jump(key) {
    scroller?.querySelector(`[data-group="${key}"]`)?.scrollIntoView({ block: 'start', behavior: 'smooth' });
  }

  function fileName(path) {
    const parts = path.split('/');
    return parts.slice(-2).join('/');
  }
</script>

<header class="border-b border-line bg-head px-3 pt-2.5 pb-2">
  <div class="flex items-center gap-1.5">
    <button
      title="Retour (Échap)"
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
    <div class="min-w-0 flex-1">
      <div class="truncate font-semibold">{project.name}</div>
      <div class="truncate text-[11px] text-muted">
        Tailwind {theme?.version ?? (config.kind === 'v4' ? '4' : '3')}
        {#if theme?.prefix}· préfixe <code class="font-mono">{theme.kind === 'v4' ? `${theme.prefix}:` : theme.prefix}</code>{/if}
        {#if theme?.source === 'bundled'}· résolu avec Tailwind embarqué{/if}
      </div>
    </div>
    <button
      title="Ouvrir la config dans Cursor"
      onclick={() => api.openInEditor(config.path)}
      class="grid size-7 place-items-center rounded-md text-muted hover:bg-panel-2 hover:text-ink"
    >
      <svg class="size-4" viewBox="0 0 20 20" fill="currentColor"
        ><path
          fill-rule="evenodd"
          d="M6.28 5.22a.75.75 0 0 1 0 1.06L2.56 10l3.72 3.72a.75.75 0 0 1-1.06 1.06L.97 10.53a.75.75 0 0 1 0-1.06l4.25-4.25a.75.75 0 0 1 1.06 0Zm7.44 0a.75.75 0 0 1 1.06 0l4.25 4.25a.75.75 0 0 1 0 1.06l-4.25 4.25a.75.75 0 0 1-1.06-1.06L17.44 10l-3.72-3.72a.75.75 0 0 1 0-1.06ZM11.38 2.27a.75.75 0 0 1 .35 1l-5 13.5a.75.75 0 1 1-1.4-.52l5-13.5a.75.75 0 0 1 1.05-.48Z"
          clip-rule="evenodd"
        /></svg
      >
    </button>
    <button
      title="Afficher dans le Finder"
      onclick={() => api.revealInFinder(config.path)}
      class="grid size-7 place-items-center rounded-md text-muted hover:bg-panel-2 hover:text-ink"
    >
      <svg class="size-4" viewBox="0 0 20 20" fill="currentColor"
        ><path
          d="M3.75 3A1.75 1.75 0 0 0 2 4.75v3.26a3.24 3.24 0 0 1 1.75-.51h12.5c.64 0 1.24.19 1.75.51V6.75A1.75 1.75 0 0 0 16.25 5h-4.84a.25.25 0 0 1-.18-.07L9.82 3.51A1.75 1.75 0 0 0 8.59 3H3.75ZM3.75 9A1.75 1.75 0 0 0 2 10.75v4.5c0 .97.78 1.75 1.75 1.75h12.5c.97 0 1.75-.78 1.75-1.75v-4.5A1.75 1.75 0 0 0 16.25 9H3.75Z"
        /></svg
      >
    </button>
    <button
      title={pinned ? 'Désépingler' : 'Épingler'}
      onclick={onpin}
      class="grid size-7 place-items-center rounded-md hover:bg-panel-2 {pinned ? 'text-amber-500' : 'text-muted hover:text-ink'}"
    >
      <svg class="size-4" viewBox="0 0 20 20" fill="currentColor"
        ><path
          fill-rule="evenodd"
          d="M10.87 2.6a1 1 0 0 0-1.74 0L7.2 6.47l-4.27.62a1 1 0 0 0-.55 1.7l3.09 3.02-.73 4.25a1 1 0 0 0 1.45 1.05L10 15.1l3.81 2.01a1 1 0 0 0 1.45-1.05l-.73-4.25 3.09-3.01a1 1 0 0 0-.55-1.71l-4.27-.62-1.93-3.87Z"
          clip-rule="evenodd"
        /></svg
      >
    </button>
    <button
      title="Masquer ce projet"
      onclick={onhide}
      class="grid size-7 place-items-center rounded-md text-muted hover:bg-panel-2 hover:text-ink"
    >
      <svg class="size-4" viewBox="0 0 20 20" fill="currentColor"
        ><path
          fill-rule="evenodd"
          d="M3.28 2.22a.75.75 0 0 0-1.06 1.06l14.5 14.5a.75.75 0 1 0 1.06-1.06l-1.75-1.75a10.05 10.05 0 0 0 3.3-4.38 1.65 1.65 0 0 0 0-1.18A10 10 0 0 0 10 3c-1.6 0-3.12.38-4.47 1.05L3.28 2.22ZM7.75 6.7l1.1 1.1a2.5 2.5 0 0 1 3.35 3.35l1.1 1.1A4 4 0 0 0 7.75 6.7Zm2.37 7.3-2.75-2.75A2.5 2.5 0 0 1 7.5 10c0-.13 0-.25.03-.38L4.6 6.7A10 10 0 0 0 .66 9.41a1.65 1.65 0 0 0 0 1.18A10 10 0 0 0 10 17c1.3 0 2.55-.25 3.69-.7L12.12 14.7A4 4 0 0 1 10.12 14Z"
          clip-rule="evenodd"
        /></svg
      >
    </button>
  </div>

  {#if project.configs.length > 1}
    <select
      bind:value={configIndex}
      class="mt-2 w-full truncate rounded-md border border-line bg-panel-2 px-2 py-1 font-mono text-[11px] outline-none"
    >
      {#each project.configs as c, i (c.path)}
        <option value={i}>{c.kind} · {fileName(c.path)}</option>
      {/each}
    </select>
  {/if}

  {#if typoCount}
    <div class="mt-2 flex rounded-lg bg-panel-2 p-0.5 text-[11px]">
      <button
        onclick={() => (tab = 'tokens')}
        class="flex-1 rounded-md px-2 py-1 font-medium {tab === 'tokens' ? 'bg-sky-500/15 text-sky-700 shadow-sm ring-1 ring-sky-500/30 dark:text-sky-300' : 'text-muted hover:text-sky-600 dark:hover:text-sky-400'}">Tokens</button
      >
      <button
        onclick={() => {
          tab = 'typo';
          loadTypo();
        }}
        class="flex-1 rounded-md px-2 py-1 font-medium {tab === 'typo' ? 'bg-emerald-500/15 text-emerald-700 shadow-sm ring-1 ring-emerald-500/30 dark:text-emerald-300' : 'text-muted hover:text-emerald-600 dark:hover:text-emerald-400'}"
        >Typographie <span class="opacity-60">{typoCount}</span></button
      >
    </div>
  {/if}

  <div class="mt-2 flex items-center gap-2">
    <input
      bind:this={input}
      bind:value={query}
      placeholder={tab === 'typo' ? 'Filtrer les utilitaires…' : 'Filtrer les tokens…'}
      spellcheck="false"
      class="min-w-0 flex-1 rounded-lg bg-panel-2 px-2.5 py-1.5 outline-none placeholder:text-muted focus:ring-2 focus:ring-accent/40"
    />
    {#if hasDefaults && tab === 'tokens'}
      <div class="flex shrink-0 rounded-lg bg-panel-2 p-0.5 text-[11px]">
        <button
          onclick={() => (customOnly = true)}
          class="rounded-md px-2 py-1 {customOnly ? 'bg-head shadow-sm' : 'text-muted'}">Perso</button
        >
        <button
          onclick={() => (customOnly = false)}
          class="rounded-md px-2 py-1 {!customOnly ? 'bg-head shadow-sm' : 'text-muted'}">Tout</button
        >
      </div>
    {/if}
  </div>

  {#if groups.length > 1 && tab === 'tokens'}
    <nav class="mt-2 flex flex-wrap gap-1">
      {#each groups as g (g.key)}
        <button
          onclick={() => jump(g.key)}
          class="shrink-0 rounded-full border border-line px-2 py-0.5 text-[11px] text-muted hover:border-accent hover:text-ink"
          >{groupLabel(g.key)} <span class="opacity-60">{g.tokens.length}</span></button
        >
      {/each}
    </nav>
  {/if}
</header>

<div bind:this={scroller} class="scroll-thin flex-1 overflow-y-auto pb-12">
  {#if tab === 'typo'}
    {#if !typoFiles.length}
      <div class="px-4 py-10 text-center text-muted">Aucun utilitaire ne correspond.</div>
    {/if}
    {#each typoFiles as file (file.path)}
      <section class="px-3 pt-3">
        <div class="sticky top-0 z-10 -mx-3 flex items-center gap-2 bg-panel/95 px-3 py-1 backdrop-blur">
          <h3 class="min-w-0 truncate font-mono text-[11px] text-muted" title={file.path}>{relPath(file.path)}</h3>
          <button
            title="Ouvrir dans Cursor"
            class="ml-auto shrink-0 text-[11px] text-accent hover:underline"
            onclick={() => api.openInEditor(file.path)}>Ouvrir</button
          >
        </div>
        {#if file.error}
          <div class="mt-1 font-mono text-[11px] break-words text-red-600 dark:text-red-400">{file.error}</div>
        {/if}
        <div class="divide-y divide-line/60">
          {#each file.utilities as u (u.name + u.line)}
            {@const cls = typoClass(u.name)}
            <div class="group -mx-1.5 rounded-md px-1.5 py-2 hover:bg-panel-2">
              <div class="flex items-center gap-1.5">
                <code class="min-w-0 flex-1 truncate font-mono text-[12px] font-medium">{cls}</code>
                <button
                  title="Ouvrir dans Cursor (ligne {u.line})"
                  class="grid size-6 shrink-0 place-items-center rounded text-muted opacity-0 group-hover:opacity-100 hover:bg-panel hover:text-ink"
                  onclick={() => api.openInEditor(file.path, u.line)}
                >
                  <svg class="size-3.5" viewBox="0 0 20 20" fill="currentColor"
                    ><path
                      fill-rule="evenodd"
                      d="M4.25 5.5a.75.75 0 0 0-.75.75v8.5c0 .41.34.75.75.75h8.5a.75.75 0 0 0 .75-.75v-4a.75.75 0 0 1 1.5 0v4A2.25 2.25 0 0 1 12.75 17h-8.5A2.25 2.25 0 0 1 2 14.75v-8.5A2.25 2.25 0 0 1 4.25 4h5a.75.75 0 0 1 0 1.5h-5Zm7.25-.75a.75.75 0 0 1 .75-.75h3.5a.75.75 0 0 1 .75.75v3.5a.75.75 0 0 1-1.5 0V6.56l-5.22 5.22a.75.75 0 1 1-1.06-1.06l5.22-5.22h-1.69a.75.75 0 0 1-.75-.75Z"
                      clip-rule="evenodd"
                    /></svg
                  >
                </button>
                <button
                  title="Copier {cls}"
                  class="flex shrink-0 items-center gap-1 rounded-md border border-line px-1.5 py-0.5 text-[11px] text-muted hover:border-accent hover:text-ink"
                  onclick={() => copyTypo(u.name)}
                >
                  <svg class="size-3" viewBox="0 0 20 20" fill="currentColor"
                    ><path d="M7 3.5A1.5 1.5 0 0 1 8.5 2h3.88a1.5 1.5 0 0 1 1.06.44l3.12 3.12A1.5 1.5 0 0 1 17 6.62V12.5a1.5 1.5 0 0 1-1.5 1.5h-1v-3.38a3 3 0 0 0-.88-2.12L10.5 5.38A3 3 0 0 0 8.38 4.5H7v-1Z" /><path
                      d="M4.5 6A1.5 1.5 0 0 0 3 7.5v9A1.5 1.5 0 0 0 4.5 18h7a1.5 1.5 0 0 0 1.5-1.5v-5.88a1.5 1.5 0 0 0-.44-1.06L9.44 6.44A1.5 1.5 0 0 0 8.38 6H4.5Z"
                    /></svg
                  >Copier
                </button>
              </div>
              {#if u.body}
                <pre class="mt-1 overflow-x-auto font-mono text-[10.5px] leading-snug whitespace-pre-wrap text-muted select-text">{annotateRem(u.body)}</pre>
              {/if}
            </div>
          {/each}
        </div>
      </section>
    {/each}
  {:else if loading}
    <div class="px-4 py-10 text-center text-muted">Lecture de la config…</div>
  {:else if theme?.error}
    <div class="m-3 rounded-lg border border-red-500/30 bg-red-500/10 p-3 text-[12px]">
      <div class="font-medium text-red-600 dark:text-red-400">Impossible de lire la config</div>
      <div class="mt-1 font-mono break-words text-muted select-text">{theme.error}</div>
      <button class="mt-2 text-accent hover:underline" onclick={() => api.openInEditor(config.path)}
        >Ouvrir le fichier dans Cursor</button
      >
    </div>
  {:else if theme && !tokenCount}
    <div class="px-4 py-10 text-center text-muted">
      {query ? 'Aucun token ne correspond.' : 'Aucun token personnalisé dans cette config.'}
    </div>
  {:else if theme}
    {#each groups as group (group.key)}
      {@const options = UTILITIES[group.key]}
      {@const util = utilFor(group.key)}
      <section data-group={group.key} class="px-3 pt-3">
        <div class="sticky top-0 z-10 -mx-3 flex items-center gap-2 bg-panel/95 px-3 py-1 backdrop-blur">
          <h3 class="text-[11px] font-medium tracking-wide text-muted uppercase">{groupLabel(group.key)}</h3>
          {#if options && options.length > 1}
            <select
              value={util}
              onchange={(e) => chooseUtil(group.key, e.currentTarget.value)}
              title="Utilitaire copié au clic"
              class="ml-auto rounded border border-line bg-panel-2 px-1 py-px font-mono text-[11px] outline-none"
            >
              {#each options as o (o)}
                <option value={o}>{o === '@' ? '@container' : o}</option>
              {/each}
            </select>
          {/if}
        </div>

        {#if group.key === 'colors'}
          <div class="grid grid-cols-4 gap-2 pt-1">
            {#each group.tokens as token (token.name)}
              {@const value = displayValue(token)}
              <button
                class="group min-w-0 text-left"
                title="{className(theme, group.key, util, token)}  ·  {value}"
                onclick={(e) => copy(group, token, e)}
                oncontextmenu={(e) => {
                  e.preventDefault();
                  openToken(token);
                }}
              >
                <div class="checker h-10 overflow-hidden rounded-lg ring-1 ring-black/10 ring-inset group-hover:ring-2 group-hover:ring-accent dark:ring-white/10">
                  <div class="size-full" style:background={value}></div>
                </div>
                <div class="mt-1 truncate text-[11px] font-medium">{token.name}</div>
                <div class="truncate font-mono text-[10px] text-muted">{value}</div>
              </button>
            {/each}
          </div>
        {:else}
          <div class="divide-y divide-line/60">
            {#each group.tokens as token (token.name)}
              {@const value = displayValue(token)}
              {@const cls = className(theme, group.key, util, token)}
              {@const px = toPx(value)}
              {@const remPx = remToPx(value)}
              <div
                role="button"
                tabindex="-1"
                class="group -mx-1.5 flex items-center gap-2.5 rounded-md px-1.5 py-1.5 hover:bg-panel-2"
                onclick={(e) => copy(group, token, e)}
                onkeydown={() => {}}
              >
                {#if group.key === 'fontFamily'}
                  <span class="w-7 shrink-0 text-[17px] leading-none" style:font-family={value}>Aa</span>
                {:else if group.key === 'borderRadius'}
                  <span class="size-5 shrink-0 border-2 border-accent/70 bg-accent-soft" style:border-radius={value}></span>
                {:else if group.key === 'boxShadow' || group.key === 'dropShadow'}
                  <span class="size-5 shrink-0 rounded bg-panel ring-1 ring-line" style:box-shadow={value}></span>
                {:else if isColorLike(value)}
                  <span class="checker size-5 shrink-0 overflow-hidden rounded ring-1 ring-line"><span class="block size-full" style:background={value}></span></span>
                {/if}
                <div class="min-w-0 flex-1">
                  <div class="truncate font-mono text-[12px]">{cls ?? token.varName ?? token.name}</div>
                  <div class="truncate font-mono text-[11px] text-muted" title={value}>{value}{#if remPx}<span class="opacity-70"> · {remPx}</span>{/if}</div>
                  {#if px != null && ['spacing', 'padding', 'margin', 'gap', 'width', 'maxWidth', 'height', 'fontSize', 'lineHeight', 'screens'].includes(group.key)}
                    <div class="mt-1 h-1 rounded-full bg-accent/60" style:width="{Math.min(px / (group.key === 'screens' ? 10 : 2), 100)}%"></div>
                  {/if}
                </div>
                {#if token.line}
                  <button
                    title="Ouvrir dans Cursor (ligne {token.line})"
                    class="grid size-6 shrink-0 place-items-center rounded text-muted opacity-0 group-hover:opacity-100 hover:bg-panel hover:text-ink"
                    onclick={(e) => {
                      e.stopPropagation();
                      openToken(token);
                    }}
                  >
                    <svg class="size-3.5" viewBox="0 0 20 20" fill="currentColor"
                      ><path
                        fill-rule="evenodd"
                        d="M4.25 5.5a.75.75 0 0 0-.75.75v8.5c0 .41.34.75.75.75h8.5a.75.75 0 0 0 .75-.75v-4a.75.75 0 0 1 1.5 0v4A2.25 2.25 0 0 1 12.75 17h-8.5A2.25 2.25 0 0 1 2 14.75v-8.5A2.25 2.25 0 0 1 4.25 4h5a.75.75 0 0 1 0 1.5h-5Zm7.25-.75a.75.75 0 0 1 .75-.75h3.5a.75.75 0 0 1 .75.75v3.5a.75.75 0 0 1-1.5 0V6.56l-5.22 5.22a.75.75 0 1 1-1.06-1.06l5.22-5.22h-1.69a.75.75 0 0 1-.75-.75Z"
                        clip-rule="evenodd"
                      /></svg
                    >
                  </button>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </section>
    {/each}
  {/if}
</div>

<footer class="absolute inset-x-0 bottom-0 border-t border-line bg-panel/95 px-3 py-1.5 text-[10.5px] text-muted backdrop-blur">
  {#if tab === 'typo'}
  Copier : nom de la classe · ↗ : ouvrir la ligne dans Cursor
  {:else}
  Clic : classe · <kbd>⌥</kbd> clic : valeur · <kbd>⇧</kbd> clic : {config.kind === 'v4' ? 'var()' : 'theme()'}
  {#if theme?.groups.some((g) => g.key === 'colors')}· clic droit couleur : Cursor{/if}
  {/if}
</footer>
