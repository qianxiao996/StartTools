<template>
  <main class="config-page">
    <section class="config-section">
      <div class="section-title">
        <Monitor class="section-icon" />
        <span>{{ labels.window }}</span>
      </div>
      <div class="setting-grid">
        <div class="setting-item">
          <span>{{ labels.windowWidth }}</span>
          <el-input-number v-model="form.Width" :min="260" :max="2400" :step="10" controls-position="right" />
        </div>
        <div class="setting-item">
          <span>{{ labels.windowHeight }}</span>
          <el-input-number v-model="form.Height" :min="240" :max="1800" :step="10" controls-position="right" />
        </div>
        <div class="setting-item">
          <span>{{ labels.leftWidth }}</span>
          <el-input-number v-model="form.LeftWidth" :min="56" :max="320" :step="4" controls-position="right" />
        </div>
        <div class="setting-item">
          <span>{{ labels.theme }}</span>
          <el-segmented v-model="form.Skin" :options="skinOptions" />
        </div>
        <div class="setting-item">
          <span>{{ labels.searchHotkey }}</span>
          <el-input
            v-model="form.HotKey_Search"
            readonly
            spellcheck="false"
            @keydown.prevent="recordHotkey($event, 'HotKey_Search')"
          />
        </div>
        <div class="setting-item">
          <span>{{ labels.mainWindowHotkey }}</span>
          <el-input
            v-model="form.HotKey_Show_Hidden"
            readonly
            spellcheck="false"
            @keydown.prevent="recordHotkey($event, 'HotKey_Show_Hidden')"
          />
        </div>
      </div>
      <div class="switch-row">
        <el-checkbox v-model="form.Top_Window">{{ labels.topWindow }}</el-checkbox>
        <el-checkbox v-model="form.Locked_Position">{{ labels.lockPosition }}</el-checkbox>
        <el-checkbox v-model="form.Locked_Size">{{ labels.lockSize }}</el-checkbox>
      </div>
    </section>

    <section class="config-section">
      <div class="section-title">
        <VideoPlay class="section-icon" />
        <span>{{ labels.startup }}</span>
      </div>
      <div class="switch-row">
        <el-checkbox v-model="form.Startup">{{ labels.startWithSystem }}</el-checkbox>
        <el-checkbox v-model="form.Startup_Background">{{ labels.startInBackground }}</el-checkbox>
      </div>
    </section>

    <section class="config-section">
      <div class="section-title">
        <FolderOpened class="section-icon" />
        <span>{{ labels.terminal }}</span>
      </div>
      <div class="setting-stack">
        <div class="setting-item wide">
          <span>{{ labels.terminalPath }}</span>
          <el-input v-model="form.Terminal" spellcheck="false" />
        </div>
        <div class="setting-item wide">
          <span>{{ labels.adminArgs }}</span>
          <el-input v-model="form.Terminal_Runas_Arguments" spellcheck="false" />
        </div>
      </div>
    </section>

    <section class="config-section">
      <div class="section-title">
        <Sort class="section-icon" />
        <span>{{ labels.toolSort }}</span>
      </div>
      <div class="setting-grid">
        <div class="setting-item">
          <span>{{ labels.sortField }}</span>
          <el-segmented v-model="form.Tools_Order_Field" :options="sortFieldOptions" />
        </div>
        <div class="setting-item">
          <span>{{ labels.sortDirection }}</span>
          <el-segmented v-model="form.Tools_Order_Type" :options="sortTypeOptions" />
        </div>
      </div>
    </section>

    <footer class="config-actions">
      <el-button :icon="RefreshLeft" @click="resetForm">{{ labels.reset }}</el-button>
      <el-button type="primary" :icon="Check" @click="saveConfig">{{ labels.save }}</el-button>
    </footer>
  </main>
</template>

<script setup>
import '../css/white.css';
import { nextTick, onMounted, onUnmounted, reactive, watch } from 'vue';
import { useStore } from 'vuex';
import { Check, FolderOpened, Monitor, RefreshLeft, Sort, VideoPlay } from '@element-plus/icons-vue';
import { invoke } from '@tauri-apps/api/core';
import { emit } from '@tauri-apps/api/event';
import { message } from '@tauri-apps/plugin-dialog';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { currentMonitor, LogicalSize } from '@tauri-apps/api/window';

const labels = {
  window: '\u7a97\u53e3',
  windowWidth: '\u7a97\u53e3\u5bbd\u5ea6',
  windowHeight: '\u7a97\u53e3\u9ad8\u5ea6',
  leftWidth: '\u5de6\u4fa7\u680f\u5bbd\u5ea6',
  theme: '\u4e3b\u9898',
  topWindow: '\u7a97\u53e3\u7f6e\u9876',
  lockPosition: '\u9501\u5b9a\u4f4d\u7f6e',
  lockSize: '\u9501\u5b9a\u5927\u5c0f',
  startup: '\u542f\u52a8',
  startWithSystem: '\u5f00\u673a\u542f\u52a8',
  startInBackground: '\u540e\u53f0\u542f\u52a8',
  terminal: '\u547d\u4ee4\u884c',
  terminalPath: '\u7ec8\u7aef\u8def\u5f84',
  adminArgs: '\u7ba1\u7406\u5458\u53c2\u6570',
  toolSort: '\u5de5\u5177\u6392\u5e8f',
  searchHotkey: '\u641c\u7d22\u70ed\u952e',
  mainWindowHotkey: '\u4e3b\u7a97\u53e3\u663e\u793a/\u9690\u85cf',
  sortField: '\u6392\u5e8f\u5b57\u6bb5',
  sortDirection: '\u6392\u5e8f\u65b9\u5411',
  reset: '\u91cd\u7f6e',
  save: '\u4fdd\u5b58',
  light: '\u6d45\u8272',
  dark: '\u6df1\u8272',
  usageCount: '\u4f7f\u7528\u6b21\u6570',
  name: '\u540d\u79f0',
  desc: '\u964d\u5e8f',
  asc: '\u5347\u5e8f',
  saved: '\u8bbe\u7f6e\u5df2\u4fdd\u5b58',
  saveFailed: '\u8bbe\u7f6e\u4fdd\u5b58\u5931\u8d25!',
  settings: '\u8bbe\u7f6e',
};

const store = useStore();
const currentWindow = getCurrentWebviewWindow();
let resizeObserver;

const skinOptions = [
  { label: labels.light, value: 'light' },
  { label: labels.dark, value: 'dark' },
];
const sortFieldOptions = [
  { label: labels.usageCount, value: 'number' },
  { label: labels.name, value: 'name' },
];
const sortTypeOptions = [
  { label: labels.desc, value: 'desc' },
  { label: labels.asc, value: 'asc' },
];

const booleanKeys = ['Startup', 'Startup_Background', 'Locked_Position', 'Locked_Size', 'Top_Window'];
const numberKeys = ['X', 'Y', 'Width', 'Height', 'Current_Menu_Id', 'Current_Tags_Id', 'LeftWidth'];
const form = reactive({});

const toBoolean = (value) => value === true || value === 'true' || value === 1 || value === '1';

function applySkin(skin) {
  if (!skin) {
    return;
  }
  const isDark = skin === 'dark';
  localStorage.setItem('useDarkKEY', isDark ? 'dark' : 'light');
  document.documentElement.classList.toggle('dark', isDark);
  document.documentElement.classList.toggle('light', !isDark);
}

function normalizeConfig(config) {
  const normalized = { ...config };
  for (const key of booleanKeys) {
    normalized[key] = toBoolean(normalized[key]);
  }
  for (const key of numberKeys) {
    const parsed = Number(normalized[key]);
    if (!Number.isNaN(parsed)) {
      normalized[key] = parsed;
    }
  }
  return normalized;
}

function formatHotkeyKey(event) {
  const key = event.key;
  if (!key || ['Control', 'Alt', 'Shift', 'Meta'].includes(key)) {
    return '';
  }
  if (key === ' ') {
    return 'Space';
  }
  if (key === 'Escape') {
    return 'Esc';
  }
  if (/^F\d{1,2}$/.test(key)) {
    return key;
  }
  if (key.length === 1) {
    return key.toUpperCase();
  }
  return key[0].toUpperCase() + key.slice(1);
}

function recordHotkey(event, configKey) {
  const parts = [];
  if (event.ctrlKey) parts.push('Ctrl');
  if (event.altKey) parts.push('Alt');
  if (event.shiftKey) parts.push('Shift');
  if (event.metaKey) parts.push('Win');

  const key = formatHotkeyKey(event);
  if (!key) {
    return;
  }

  parts.push(key);
  form[configKey] = parts.join('+');
}

function resetForm() {
  Object.assign(form, normalizeConfig(store.state.Config));
}

async function fitWindowToContent() {
  await nextTick();
  const scaleFactor = await currentWindow.scaleFactor();
  const size = await currentWindow.innerSize();
  const monitor = await currentMonitor();
  const logicalWidth = Math.round(size.width / scaleFactor);
  const contentHeight = document.querySelector('.config-page')?.scrollHeight || document.body.scrollHeight;
  const maxHeight = monitor ? Math.floor(monitor.size.height / scaleFactor - 80) : 900;
  const nextHeight = Math.min(Math.max(contentHeight, 420), maxHeight);
  await currentWindow.setSize(new LogicalSize(logicalWidth, nextHeight));
}

async function saveConfig() {
  const nextConfig = {
    ...form,
    Width: Number(form.Width),
    Height: Number(form.Height),
    LeftWidth: Number(form.LeftWidth),
  };
  const result = await invoke('update_config', { config: nextConfig });
  if (result === 'ok') {
    store.commit('updateConfig', nextConfig);
    await emit('configUpdated', nextConfig);
    await message(labels.saved, { title: labels.settings, type: 'info' });
  } else {
    await message(labels.saveFailed + '\n' + result, { title: 'Error', type: 'error' });
  }
}

watch(
  () => store.state.Config,
  () => resetForm(),
  { deep: true, immediate: true }
);

watch(
  () => form.Skin,
  (skin) => applySkin(skin),
  { immediate: true }
);

onMounted(async () => {
  await fitWindowToContent();
  resizeObserver = new ResizeObserver(() => {
    fitWindowToContent();
  });
  const page = document.querySelector('.config-page');
  if (page) {
    resizeObserver.observe(page);
  }
});

onUnmounted(() => {
  resizeObserver?.disconnect?.();
});
</script>

<style scoped>
.config-page {
  min-height: 0;
  overflow: visible;
  padding: 14px 16px 18px;
  box-sizing: border-box;
  background: var(--el-bg-color);
}

.config-section {
  border-bottom: 1px solid var(--el-border-color-lighter);
  padding: 12px 0 16px;
}

.config-section:first-child {
  padding-top: 0;
}

.section-title {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 28px;
  font-size: 14px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.section-icon {
  width: 17px;
  height: 17px;
  color: var(--el-color-primary);
}

.setting-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 12px 16px;
  margin-top: 10px;
}

.setting-stack {
  display: grid;
  gap: 12px;
  margin-top: 10px;
}

.setting-item {
  display: grid;
  grid-template-columns: 86px minmax(0, 1fr);
  align-items: center;
  gap: 10px;
  min-width: 0;
  font-size: 13px;
  color: var(--el-text-color-regular);
}

.setting-item.wide {
  grid-template-columns: 86px minmax(0, 1fr);
}

.switch-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 22px;
  margin-top: 12px;
}

.config-actions {
  position: sticky;
  bottom: 0;
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 0 0;
  background: var(--el-bg-color);
}

:deep(.el-segmented),
:deep(.el-input-number),
:deep(.el-input) {
  width: 100%;
}
</style>
