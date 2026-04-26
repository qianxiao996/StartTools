import { invoke } from "@tauri-apps/api/core";

async function loadData(store, actionName, invokeName) {
  try {
    const data = await invoke(invokeName, {});
    const list = Array.isArray(data) ? data : [];

    if (actionName === 'updateMenus') {
      list.sort((a, b) => Number(a.sort || 0) - Number(b.sort || 0));
    }

    store?.commit(actionName, list);
    return list;
  } catch (error) {
    console.error(`Failed to load ${invokeName}:`, error);
    return [];
  }
}

export async function loadMenus(store) {
  return loadData(store, 'updateMenus', 'load_menus');
}

export async function loadTags(store) {
  return loadData(store, 'updateTags', 'load_tags');
}

export async function loadTools(store) {
  return loadData(store, 'updateTools', 'load_tools');
}
