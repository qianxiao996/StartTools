import { invoke } from '@tauri-apps/api/core';

let registeredShortcut = '';
let registeredMainWindowShortcut = '';

export async function setupSearchHotkey(shortcut) {
  const nextShortcut = String(shortcut || 'Alt+3').trim();
  if (!nextShortcut || nextShortcut === registeredShortcut) {
    return;
  }

  try {
    await invoke('register_search_hotkey', { shortcut: nextShortcut });
    registeredShortcut = nextShortcut;
  } catch (error) {
    console.warn('Failed to register search shortcut:', nextShortcut, error);
  }
}

export async function setupMainWindowHotkey(shortcut) {
  const nextShortcut = String(shortcut || 'Alt+2').trim();
  if (!nextShortcut || nextShortcut === registeredMainWindowShortcut) {
    return;
  }

  try {
    await invoke('register_main_window_hotkey', { shortcut: nextShortcut });
    registeredMainWindowShortcut = nextShortcut;
  } catch (error) {
    console.warn('Failed to register main window shortcut:', nextShortcut, error);
  }
}
