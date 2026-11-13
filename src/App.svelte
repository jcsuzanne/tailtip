<script>
  import { onMount } from 'svelte';
  import { api } from './lib/api.js';
  import ProjectList from './lib/ProjectList.svelte';
  import ProjectView from './lib/ProjectView.svelte';
  import Settings from './lib/Settings.svelte';

  let view = $state('list');
  let projects = $state([]);
  let settings = $state(null);
  let current = $state(null);
  let scanning = $state(false);
  let toast = $state(null);
  let toastTimer;
  let listRef = $state();
  let projectRef = $state();

  function notify(message) {
    toast = message;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = null), 1400);
  }

  async function saveSettings(next) {
    settings = next;
    try {
      await api.saveSettings($state.snapshot(next));
    } catch (e) {
      notify(String(e));
    }
  }

  async function rescan() {
    scanning = true;
    try {
      projects = await api.rescan();
      notify(`${projects.length} projets trouvés`);
    } finally {
      scanning = false;
    }
  }

  function openProject(project) {
    current = project;
    view = 'project';
    const recent = [project.path, ...settings.recent.filter((p) => p !== project.path)].slice(0, 8);
    saveSettings({ ...settings, recent });
  }

  function togglePin(project) {
    const pinned = settings.pinned.includes(project.path)
      ? settings.pinned.filter((p) => p !== project.path)
      : [...settings.pinned, project.path];
    saveSettings({ ...settings, pinned });
  }

  function hideProject(project) {
    saveSettings({ ...settings, hidden: [...settings.hidden, project.path] });
    view = 'list';
    current = null;
  }

  function onKeydown(e) {
    if (e.key === 'Escape') {
      e.preventDefault();
      if (view === 'list') api.hideWindow();
      else view = 'list';
    }
  }

  onMount(() => {
    let unlisten = [];
    (async () => {
      settings = await api.getSettings();
      scanning = true;
      projects = await api.listProjects();
      scanning = false;
      unlisten.push(
        await api.listen('popover-shown', () => {
          if (view === 'list') listRef?.focus();
          else if (view === 'project') projectRef?.focus();
        }),
        await api.listen('rescan-requested', rescan),
      );
    })();
    return () => unlisten.forEach((u) => u());
  });
</script>

<svelte:window onkeydown={onKeydown} />

<main
  class="relative flex h-full flex-col overflow-hidden rounded-xl border border-line bg-panel font-sans text-[13px] text-ink"
>
  {#if !settings}
    <div class="grid flex-1 place-items-center text-muted">Chargement…</div>
  {:else if view === 'list'}
    <ProjectList
      bind:this={listRef}
      {projects}
      {settings}
      {scanning}
      onopen={openProject}
      onpin={togglePin}
      onrescan={rescan}
      onsettings={() => (view = 'settings')}
    />
  {:else if view === 'project' && current}
    <ProjectView
      bind:this={projectRef}
      project={current}
      pinned={settings.pinned.includes(current.path)}
      onback={() => (view = 'list')}
      onpin={() => togglePin(current)}
      onhide={() => hideProject(current)}
      {notify}
    />
  {:else if view === 'settings'}
    <Settings
      {settings}
      {projects}
      onsave={saveSettings}
      onback={() => (view = 'list')}
      onrescan={async () => {
        view = 'list';
        await rescan();
      }}
      {notify}
    />
  {/if}

  {#if toast}
    <div
      class="pointer-events-none absolute inset-x-0 bottom-3 mx-auto w-fit max-w-[90%] truncate rounded-full bg-ink px-3 py-1.5 font-mono text-[12px] text-panel shadow-lg"
    >
      {toast}
    </div>
  {/if}
</main>
