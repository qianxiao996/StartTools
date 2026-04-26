<template>
  <template v-if="isConfigReady">
    <TitleBar v-if="isShowTitleBar" />
    <router-view></router-view>
  </template>
</template>

<script setup>
import { onMounted, ref, watch } from 'vue';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import { useStore } from 'vuex';
import TitleBar from './views/TitleBar.vue';
import { loadConfig, setupConfigWatchers } from './js/configLoader.js';
import { setupMainWindowHotkey, setupSearchHotkey } from './js/searchHotkey.js';

const store = useStore();
const currentWindow = getCurrentWindow();
const isShowTitleBar = ref(false);
const isConfigReady = ref(false);

function applySkin(skin) {
  const isDark = skin === 'dark';
  localStorage.setItem('useDarkKEY', isDark ? 'dark' : 'light');
  document.documentElement.classList.toggle('dark', isDark);
  document.documentElement.classList.toggle('light', !isDark);
}

watch(
  () => store.state.Config.Skin,
  (skin) => applySkin(skin),
  { immediate: true }
);

onMounted(async () => {
  await loadConfig(store);
  await listen('configUpdated', (event) => {
    store.commit('updateConfig', event.payload);
    if (currentWindow.label === 'main') {
      setupSearchHotkey(event.payload.HotKey_Search);
      setupMainWindowHotkey(event.payload.HotKey_Show_Hidden);
      const width = Number(event.payload.Width);
      const height = Number(event.payload.Height);
      if (!Number.isNaN(width) && !Number.isNaN(height)) {
        currentWindow.setSize(new LogicalSize(width, height));
      }
    }
  });
  if (currentWindow.label === 'main') {
    await setupSearchHotkey(store.state.Config.HotKey_Search);
    await setupMainWindowHotkey(store.state.Config.HotKey_Show_Hidden);
    setupConfigWatchers(store);
  }
  isShowTitleBar.value = currentWindow.label === 'main' && !await currentWindow.isDecorated();
  isConfigReady.value = true;
});
</script>

<style scoped>
</style>
