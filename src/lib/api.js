import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { open as openDialog } from '@tauri-apps/plugin-dialog';

export const api = {
  getSettings: () => invoke('get_settings'),
  saveSettings: (settings) => invoke('save_settings', { settings }),
  listProjects: () => invoke('list_projects'),
  rescan: () => invoke('rescan'),
  loadTheme: (path, kind) => invoke('load_theme', { path, kind }),
  openInEditor: (path, line = null) => invoke('open_in_editor', { path, line }),
  revealInFinder: (path) => invoke('reveal_in_finder', { path }),
  hideWindow: () => invoke('hide_window'),
  detectedNode: () => invoke('detected_node'),
  quit: () => invoke('quit'),
  copy: (text) => writeText(text),
  listen,
  async pickFolder() {
    await invoke('set_keep_open', { keep: true });
    try {
      return await openDialog({ directory: true, multiple: false });
    } finally {
      await invoke('set_keep_open', { keep: false });
    }
  },
};
