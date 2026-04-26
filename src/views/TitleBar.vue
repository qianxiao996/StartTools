<template>
  <div :data-tauri-drag-region="isDraggable ? '' : null" class="titlebar" @dblclick="toggleMaximizeFromTitlebar">
    <div class="titlebar-left">
      <img :src="appIconSrc" alt="App Icon" class="titlebar-icon">
      <span class="titlebar-title">{{ windowTitle }}</span>
    </div>

    <el-button-group class="titlebar-actions">
      <el-tooltip effect="light" :content="labels.home" placement="bottom">
        <el-button @click="routerPush('Home')" class="titlebar-button" :icon="House" />
      </el-tooltip>

      <el-tooltip effect="light" :content="labels.search" placement="bottom">
        <el-button @click="routerPush('Search')" class="titlebar-button" :icon="Search" />
      </el-tooltip>

      <el-tooltip effect="light" :content="labels.skin" placement="bottom">
        <el-dropdown trigger="click" @command="changeSkin">
          <el-button class="titlebar-button" :icon="User" />
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item :icon="Sunny" command="light">{{ labels.lightStyle }}</el-dropdown-item>
              <el-dropdown-item :icon="Moon" command="dark">{{ labels.darkStyle }}</el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </el-tooltip>

      <el-tooltip effect="light" :content="labels.settings" placement="bottom">
        <el-dropdown trigger="click" @command="changeSet">
          <el-button class="titlebar-button" :icon="Setting" />
          <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item :icon="Setting" command="set">{{ labels.settings }}</el-dropdown-item>
                <el-dropdown-item :icon="DocumentAdd" command="toolManage">{{ labels.toolManage }}</el-dropdown-item>
                <el-dropdown-item :icon="Download" command="exportData">{{ labels.exportData }}</el-dropdown-item>
                <el-dropdown-item :icon="Upload" command="importData">{{ labels.importData }}</el-dropdown-item>
                <el-dropdown-item :icon="FolderChecked" command="path1">{{ labels.toPortablePath }}</el-dropdown-item>
                <el-dropdown-item :icon="FolderOpened" command="path2">{{ labels.toAbsolutePath }}</el-dropdown-item>
              <el-dropdown-item :icon="SwitchButton" command="exit">{{ labels.exit }}</el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </el-tooltip>

      <el-button @click="minimize" class="titlebar-button" :icon="Minus" />
      <el-button @click="maximize" class="titlebar-button" :icon="FullScreen" />
      <el-button @click="close" class="titlebar-button" :icon="Close" />
    </el-button-group>
  </div>
</template>

<script setup>
import { computed, getCurrentInstance, onMounted, ref } from 'vue';
import { useStore } from 'vuex';
import { WebviewWindow, getAllWebviewWindows } from '@tauri-apps/api/webviewWindow';
import { Window } from '@tauri-apps/api/window';
import {
  Close,
  DocumentAdd,
  Download,
  FolderChecked,
  FolderOpened,
  FullScreen,
  House,
  Minus,
  Moon,
  Search,
  Setting,
  Sunny,
  SwitchButton,
  Upload,
  User,
} from '@element-plus/icons-vue';
import { loadMenus, loadTags, loadTools } from '../js/data';
import { invoke } from "@tauri-apps/api/core";
import { emit } from '@tauri-apps/api/event';
import { ask, message, open, save } from '@tauri-apps/plugin-dialog';
import appIconSrc from '../js/appIcon.js';

const labels = {
  home: '\u4e3b\u9875',
  search: '\u641c\u7d22',
  skin: '\u76ae\u80a4',
  lightStyle: '\u6d45\u8272\u98ce\u683c',
  darkStyle: '\u6df1\u8272\u98ce\u683c',
  settings: '\u8bbe\u7f6e',
  toolManage: '\u5de5\u5177\u7ba1\u7406',
  exportData: '导出数据',
  importData: '导入数据',
  toPortablePath: '\u8f6c\u6362\u5230\u4fbf\u643a\u8def\u5f84',
  toAbsolutePath: '\u8f6c\u6362\u5230\u7edd\u5bf9\u8def\u5f84',
  exit: '\u9000\u51fa',
  createSettingsFailed: '\u521b\u5efa\u8bbe\u7f6e\u7a97\u53e3\u5931\u8d25!',
  createToolManageFailed: '\u521b\u5efa\u5de5\u5177\u7ba1\u7406\u7a97\u53e3\u5931\u8d25!',
  convertFailed: '\u8f6c\u6362\u5931\u8d25!',
  exportDataSuccess: '数据已导出',
  exportDataFailed: '导出数据失败!',
  importDataSuccess: '数据已导入',
  importDataFailed: '导入数据失败!',
  importDataConfirm: '导入会覆盖当前数据，是否继续？',
};

const { proxy } = getCurrentInstance();
const store = useStore();
const appWindow = new Window('main');
const windowTitle = ref('');
const toBoolean = (value) => value === true || value === 'true' || value === 1 || value === '1';

const isDraggable = computed(() => !toBoolean(store.state.Config.Locked_Position));

onMounted(async () => {
  windowTitle.value = await appWindow.title();
});

async function changeSkin(command) {
  const nextConfig = { ...store.state.Config, Skin: command };
  store.commit('updateSkin', command);
  await emit('configUpdated', nextConfig);
}

function changeSet(command) {
  switch (command) {
    case 'set':
      openConfigWindow();
      break;
    case 'toolManage':
      openToolsManageWindow();
      break;
    case 'exportData':
      exportData();
      break;
    case 'importData':
      importData();
      break;
    case 'path1':
      changeBianxiePath(true);
      break;
    case 'path2':
      changeBianxiePath(false);
      break;
    case 'exit':
      appWindow.close();
      break;
    default:
      break;
  }
}

function firstPath(value) {
  if (!value) {
    return '';
  }
  return Array.isArray(value) ? value[0] || '' : value;
}

async function exportData() {
  const path = await save({
    title: labels.exportData,
    defaultPath: 'starttools-backup.stbak',
    filters: [{ name: 'StartTools 备份', extensions: ['stbak'] }],
  });
  if (!path) {
    return;
  }

  try {
    await invoke('export_data', { path });
    await message(labels.exportDataSuccess, { title: labels.exportData, type: 'info' });
  } catch (error) {
    await message(labels.exportDataFailed + '\n' + error, { title: 'Error', type: 'error' });
  }
}

async function importData() {
  const confirmed = await ask(labels.importDataConfirm, {
    title: labels.importData,
    kind: 'warning',
  });
  if (!confirmed) {
    return;
  }

  const selected = await open({
    title: labels.importData,
    multiple: false,
    directory: false,
    filters: [{ name: 'StartTools 备份', extensions: ['stbak'] }],
  });
  const path = firstPath(selected);
  if (!path) {
    return;
  }

  try {
    await invoke('import_data', { path });
    const nextConfig = await invoke('load_config');
    store.commit('updateConfig', nextConfig);
    await Promise.all([loadMenus(store), loadTags(store), loadTools(store)]);
    await emit('configUpdated', nextConfig);
    await emit('builtinToolsUpdated');
    await emit('startToolsPluginsUpdated');
    await message(labels.importDataSuccess, { title: labels.importData, type: 'info' });
  } catch (error) {
    await message(labels.importDataFailed + '\n' + error, { title: 'Error', type: 'error' });
  }
}

async function openToolsManageWindow() {
  const windows = await getAllWebviewWindows();
  const toolsManageWindow = windows.find((window) => window.label === 'tools_manage');
  if (toolsManageWindow) {
    if (await toolsManageWindow.isMinimized()) {
      await toolsManageWindow.unminimize();
    }
    await toolsManageWindow.show();
    await toolsManageWindow.center();
    await toolsManageWindow.setFocus();
    return;
  }

  const toolsManageWebview = new WebviewWindow('tools_manage', {
    url: '/toolsmanage',
    title: labels.toolManage,
    width: 1100,
    height: 760,
    minWidth: 900,
    minHeight: 560,
    center: true,
    resizable: true,
    alwaysOnTop: false,
    focus: true,
  });

  toolsManageWebview.once('tauri://created', async () => {
    await toolsManageWebview.show();
    await toolsManageWebview.setFocus();
  });

  toolsManageWebview.once('tauri://error', async (error) => {
    await message(labels.createToolManageFailed + '\n' + error, { title: 'Error', type: 'error' });
  });
}

async function openConfigWindow() {
  const windows = await getAllWebviewWindows();
  const configWindow = windows.find((window) => window.label === 'config');
  if (configWindow) {
    await configWindow.show();
    await configWindow.center();
    await configWindow.setFocus();
    return;
  }

  const configWebview = new WebviewWindow('config', {
    url: '/config',
    title: labels.settings,
    width: 620,
    minWidth: 520,
    minHeight: 420,
    center: true,
    resizable: true,
    alwaysOnTop: false,
    focus: true,
  });

  configWebview.once('tauri://created', async () => {
    await configWebview.show();
    await configWebview.setFocus();
  });

  configWebview.once('tauri://error', async (error) => {
    await message(labels.createSettingsFailed + '\n' + error, { title: 'Error', type: 'error' });
  });
}

async function changeBianxiePath(isBianxie) {
  const tooldata = store.state.Tools;
  const result = await invoke("change_bianxie_path", { allTool: tooldata, isBianxie });
  if (result === "ok") {
    loadTools(store);
  } else {
    await message(labels.convertFailed + '\n' + result, { title: 'Error', type: 'error' });
  }
}

function minimize() {
  appWindow.minimize();
}

function maximize() {
  appWindow.toggleMaximize();
}

function toggleMaximizeFromTitlebar(event) {
  const interactive = event.target.closest?.('.titlebar-actions, .titlebar-button, .el-dropdown, button');
  if (interactive) {
    return;
  }
  maximize();
}

function close() {
  appWindow.hide();
  store.commit('updateClose', true);
}

function routerPush(path) {
  proxy.$router.push({ name: path });
}
</script>

<style scoped>
.titlebar-actions {
  display: flex;
}
</style>
