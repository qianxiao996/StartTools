<script setup>
import { ref, computed, nextTick, onMounted, onUnmounted, watch } from 'vue';
import { useStore } from 'vuex';
import { useRouter } from 'vue-router';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { message } from '@tauri-apps/plugin-dialog';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { LogicalSize } from '@tauri-apps/api/window';
import {
  ArrowRight,
  ArrowRightBold,
  Link,
  Monitor,
  Platform,
  Folder,
} from '@element-plus/icons-vue';
import { loadMenus, loadTags, loadTools } from '../js/data.js';
import { toolsHandleContextMenuAction } from '../js/toolsContextMenuActions.js';

const props = defineProps({
  mode: {
    type: String,
    default: 'inline',
    validator: (value) => ['global', 'inline'].includes(value),
  },
});

const store = useStore();
const router = useRouter();
const keyword = ref('');
const builtinTools = ref([]);
const startToolsPlugins = ref([]);
const toolsDropdowns = ref([]);
const selectedIndex = ref(-1);
const searchInput = ref(null);
const isSearchInputFocused = ref(false);
const isSearchResultsActive = ref(false);
const currentWindow = getCurrentWebviewWindow();
const isGlobalSearch = computed(() => props.mode === 'global');
const MAX_SEARCH_RESULTS = 80;
let unlistenWindowBlur;
let unlistenBuiltinToolsUpdated;
let unlistenToolsUpdated;
let unlistenStartToolsPluginsUpdated;
let inlineIdleTimer;
let isContextMenuOpen = false;
let contextMenuExtraHeight = 0;
let contextMenuCloseTimer;
let contextMenuToken = 0;
let resizeFrame = 0;
let resizeRequested = false;

const allTools = computed(() => store.state.Tools);
const menusById = computed(() => new Map(store.state.Menus.map((menu) => [Number(menu.id), menu.name])));
const tagsById = computed(() => new Map(store.state.Tags.map((tag) => [Number(tag.id), tag.name])));
const indexedNormalTools = computed(() =>
  allTools.value.map((tool) => {
    const menu = menusById.value.get(Number(tool.menu_id)) || '';
    const tags = tagsById.value.get(Number(tool.tags_id)) || '默认标签';
    return {
      ...tool,
      __searchName: String(tool.name || '').toLowerCase(),
      __searchTarget: String(tool.target || '').toLowerCase(),
      __menuName: menu,
      __tagName: tags,
      __meta: `${menu}/${tags}`,
    };
  })
);
const rawQuery = computed(() => keyword.value.trim());
const isBuiltinPrefix = computed(() => rawQuery.value.startsWith('>'));
const isStartToolsPrefix = computed(() => rawQuery.value.startsWith('/'));
const searchQuery = computed(() => {
  const value = isBuiltinPrefix.value || isStartToolsPrefix.value ? rawQuery.value.slice(1) : rawQuery.value;
  return value.trim().toLowerCase();
});

const normalToolResults = computed(() => {
  const query = searchQuery.value;
  if (!query) {
    return [];
  }

  const results = [];
  for (const tool of indexedNormalTools.value) {
    if (tool.__searchName.includes(query) || tool.__searchTarget.includes(query)) {
      results.push({
        ...tool,
        __searchType: 'normal',
        menu: tool.__menuName,
        tags: tool.__tagName,
        meta: tool.__meta,
      });
      if (results.length >= MAX_SEARCH_RESULTS) {
        break;
      }
    }
  }
  return results;
});

const builtinToolResults = computed(() => {
  const query = searchQuery.value;
  if (!isBuiltinPrefix.value) {
    return [];
  }

  const results = [];
  for (const tool of builtinTools.value) {
      if (!tool.enabled) {
        continue;
      }
    const name = String(tool.name || '').toLowerCase();
    const content = String(tool.content || '').toLowerCase();
    if (!query || name.includes(query) || content.includes(query)) {
      results.push({
        ...tool,
        __searchName: name,
        __searchType: 'builtin',
        icon: '',
        target: tool.content || '',
        meta: `内置工具/${shellLabel(tool.shell)}`,
      });
      if (results.length >= MAX_SEARCH_RESULTS) {
        break;
      }
    }
  }

  return results
    .sort((a, b) => {
      return Number(a.__searchName !== query) - Number(b.__searchName !== query);
    });
});

const startToolsPluginResults = computed(() => {
  const query = searchQuery.value;
  if (!isStartToolsPrefix.value) {
    return [];
  }

  const results = [];
  for (const plugin of startToolsPlugins.value) {
    const name = String(plugin.name || '').toLowerCase();
    const pluginId = String(plugin.plugin_id || '').toLowerCase();
    const main = String(plugin.main || '').toLowerCase();
    if (!query || name.includes(query) || pluginId.includes(query) || main.includes(query)) {
      results.push({
        ...plugin,
        id: plugin.id,
        __searchType: 'starttools-plugin',
        icon: plugin.logo_url || plugin.logo || '',
        target: plugin.main || '',
        meta: 'StartTools 插件',
      });
      if (results.length >= MAX_SEARCH_RESULTS) {
        break;
      }
    }
  }
  return results;
});

const filteredTools = computed(() => {
  if (isBuiltinPrefix.value) {
    return builtinToolResults.value;
  }
  if (isStartToolsPrefix.value) {
    return startToolsPluginResults.value;
  }
  return normalToolResults.value;
});
const shouldShowResults = computed(() =>
  (Boolean(searchQuery.value) || isBuiltinPrefix.value || isStartToolsPrefix.value) && filteredTools.value.length > 0
);

watch(filteredTools, (tools) => {
  selectedIndex.value = tools.length > 0 ? 0 : -1;
  scheduleResizeSearchWindow();
});

watch(keyword, () => {
  scheduleResizeSearchWindow();
  resetInlineIdleTimer();
});

onMounted(async () => {
  await Promise.all([
    loadMenus(store),
    loadTags(store),
    loadTools(store),
    loadBuiltinTools(),
    loadStartToolsPlugins(),
  ]);
  await nextTick();
  searchInput.value?.focus?.();
  await resizeSearchWindow();
  if (isGlobalSearch.value) {
    await currentWindow.show();
    await currentWindow.setFocus();
  }
  window.addEventListener('keydown', handleWindowKeydown);
  unlistenBuiltinToolsUpdated = await listen('builtinToolsUpdated', loadBuiltinTools);
  unlistenToolsUpdated = await listen('toolUpdated', () => loadTools(store));
  unlistenStartToolsPluginsUpdated = await listen('startToolsPluginsUpdated', loadStartToolsPlugins);

  if (isGlobalSearch.value) {
    unlistenWindowBlur = await currentWindow.onFocusChanged(({ payload }) => {
      if (!payload) {
        closeGlobalSearch();
      }
    });
  } else {
    resetInlineIdleTimer();
  }
});

onUnmounted(() => {
  window.removeEventListener('keydown', handleWindowKeydown);
  unlistenWindowBlur?.();
  unlistenBuiltinToolsUpdated?.();
  unlistenToolsUpdated?.();
  unlistenStartToolsPluginsUpdated?.();
  clearInlineIdleTimer();
  if (resizeFrame) {
    cancelAnimationFrame(resizeFrame);
    resizeFrame = 0;
  }
});

function scheduleResizeSearchWindow() {
  if (!isGlobalSearch.value) {
    return;
  }
  resizeRequested = true;
  if (resizeFrame) {
    return;
  }
  resizeFrame = requestAnimationFrame(async () => {
    resizeFrame = 0;
    if (!resizeRequested) {
      return;
    }
    resizeRequested = false;
    await resizeSearchWindow();
    if (resizeRequested) {
      scheduleResizeSearchWindow();
    }
  });
}

async function loadBuiltinTools() {
  try {
    builtinTools.value = await invoke('load_builtin_tools');
  } catch (error) {
    console.warn('Failed to load builtin tools:', error);
    builtinTools.value = [];
  }
}

async function loadStartToolsPlugins() {
  try {
    startToolsPlugins.value = await invoke('load_starttools_plugins');
  } catch (error) {
    console.warn('Failed to load StartTools plugins:', error);
    startToolsPlugins.value = [];
  }
}

async function resizeSearchWindow() {
  if (!isGlobalSearch.value) {
    return;
  }

  await nextTick();
  const inputHeight = searchInput.value?.$el?.offsetHeight || 42;
  const resultHeight = shouldShowResults.value
    ? Math.min(filteredTools.value.length * 42, 360)
    : 0;
  await currentWindow.setSize(new LogicalSize(640, inputHeight + resultHeight + contextMenuExtraHeight));
}

async function scrollSelectedIntoView() {
  await nextTick();
  document
    .querySelector(`[data-search-index="${selectedIndex.value}"]`)
    ?.scrollIntoView?.({ block: 'nearest' });
}

function selectNext() {
  resetInlineIdleTimer();
  if (filteredTools.value.length === 0) {
    selectedIndex.value = -1;
    return;
  }
  selectedIndex.value = (selectedIndex.value + 1) % filteredTools.value.length;
  scrollSelectedIntoView();
}

function selectPrev() {
  resetInlineIdleTimer();
  if (filteredTools.value.length === 0) {
    selectedIndex.value = -1;
    return;
  }
  selectedIndex.value = (selectedIndex.value - 1 + filteredTools.value.length) % filteredTools.value.length;
  scrollSelectedIntoView();
}

async function runSelected() {
  resetInlineIdleTimer();
  if (isBuiltinPrefix.value) {
    const exactBuiltinTool = builtinToolResults.value.find(
      (tool) => String(tool.name || '').toLowerCase() === searchQuery.value
    );
    if (exactBuiltinTool) {
      await toolsHandle('run', exactBuiltinTool);
      return;
    }
  }

  const tool = filteredTools.value[selectedIndex.value];
  if (tool) {
    await toolsHandle('run', tool);
  }
}

function closeGlobalSearch() {
  if (!isGlobalSearch.value) {
    return;
  }
  keyword.value = '';
  currentWindow.hide();
}

function handleWindowKeydown(event) {
  resetInlineIdleTimer();
  if (event.key === 'Escape') {
    event.preventDefault();
    closeGlobalSearch();
  }
}

async function toolsDropdownClick(visible, index) {
  resetInlineIdleTimer();
  if (!visible) {
    const closeToken = contextMenuToken;
    clearContextMenuCloseTimer();
    contextMenuCloseTimer = setTimeout(() => {
      if (closeToken === contextMenuToken && !getVisibleDropdownMenu()) {
        isContextMenuOpen = false;
        contextMenuExtraHeight = 0;
        resizeSearchWindow();
      }
    }, 300);
    return;
  }
  contextMenuToken += 1;
  isContextMenuOpen = true;
  clearContextMenuCloseTimer();
  contextMenuExtraHeight = estimateContextMenuExtraHeight(index);
  if (contextMenuExtraHeight > 0) {
    await resizeSearchWindow();
    await nextTick();
    await nextFrame();
  }
  const currentDropdown = toolsDropdowns.value[index];
  closeDropdowns(index);
  currentDropdown?.handleOpen?.();
}

function closeDropdowns(exceptIndex = -1) {
  toolsDropdowns.value.forEach((dropdown, index) => {
    if (index !== exceptIndex) {
      dropdown?.handleClose?.();
    }
  });
}

function clearContextMenuCloseTimer() {
  if (contextMenuCloseTimer) {
    clearTimeout(contextMenuCloseTimer);
    contextMenuCloseTimer = null;
  }
}

function estimateContextMenuExtraHeight(index) {
  if (!isGlobalSearch.value) {
    return 0;
  }
  const tool = filteredTools.value[index];
  const menuItemCount = tool?.__searchType === 'normal' ? 6 : 1;
  const menuHeight = menuItemCount * 38 + 24;
  const inputHeight = searchInput.value?.$el?.offsetHeight || 42;
  const resultHeight = shouldShowResults.value
    ? Math.min(filteredTools.value.length * 42, 360)
    : 0;
  const results = document.querySelector('.search-tools-content');
  const scrollTop = results?.scrollTop || 0;
  const triggerTop = inputHeight + index * 42 - scrollTop;
  const baseWindowHeight = inputHeight + resultHeight;
  const availableHeight = baseWindowHeight - triggerTop;
  const missingHeight = menuHeight - availableHeight;
  return missingHeight > 0 ? Math.ceil(missingHeight + 32) : 0;
}

async function adjustSearchWindowForContextMenu() {
  if (!isGlobalSearch.value) {
    return;
  }
  await nextTick();
  await nextFrame();
  await nextFrame();

  const menu = getVisibleDropdownMenu();
  if (!menu) {
    return;
  }
  const menuRect = menu.getBoundingClientRect();
  const menuHeight = Math.ceil(Math.max(
    menu.scrollHeight,
    menu.offsetHeight,
    menuRect.height
  ));
  const overflowHeight = menuRect.top + menuHeight - window.innerHeight;
  contextMenuExtraHeight = overflowHeight > 0 ? menuHeight + 16 : 0;
  if (contextMenuExtraHeight > 0) {
    await resizeSearchWindow();
  }
}

function nextFrame() {
  return new Promise((resolve) => requestAnimationFrame(resolve));
}

function handleSearchIconError(tool) {
  tool.icon = '';
}

function getVisibleDropdownMenu() {
  const menus = Array.from(document.querySelectorAll('.el-dropdown__popper .el-dropdown-menu'));
  return menus.find((menu) => {
    const popper = menu.closest('.el-popper');
    const rect = menu.getBoundingClientRect();
    return popper
      && popper.getAttribute('aria-hidden') !== 'true'
      && rect.width > 0
      && Math.max(menu.scrollHeight, menu.offsetHeight, rect.height) > 0;
  });
}

function handleToolMouseEnter(index) {
  selectedIndex.value = index;
  resetInlineIdleTimer();
}

async function toolsHandle(action, tool) {
  resetInlineIdleTimer();
  if (tool?.__searchType === 'starttools-plugin') {
    if (action === 'run') {
      await openStartToolsPlugin(tool);
    }
    return;
  }

  if (tool?.__searchType === 'builtin') {
    if (action === 'run') {
      await runBuiltinTool(tool);
    }
    return;
  }

  await toolsHandleContextMenuAction(store, action, tool, null, null);
  if (action === 'run' && isGlobalSearch.value) {
    currentWindow.hide();
    keyword.value = '';
  }
}

async function openStartToolsPlugin(plugin) {
  try {
    const url = await invoke('open_starttools_plugin', { pluginId: plugin.plugin_id });
    console.info('[StartTools plugin url]', url);
    keyword.value = '';
  } catch (error) {
    await message('打开 StartTools 插件失败: ' + error, { title: '运行失败', type: 'error' });
  }
}
async function runBuiltinTool(tool) {
  try {
    await invoke('run_builtin_tool', { tool: toBuiltinTool(tool) });
    if (isGlobalSearch.value) {
      currentWindow.hide();
      keyword.value = '';
    }
  } catch (error) {
    await message(String(error), { title: '运行失败', type: 'error' });
  }
}

function toBuiltinTool(tool) {
  return {
    id: Number(tool.id),
    name: tool.name || '',
    run_type: tool.run_type || 'script',
    shell: tool.shell || defaultShell(),
    content: tool.content || '',
    working_dir: tool.working_dir || '',
    os: tool.os || currentClientOs(),
    enabled: Boolean(tool.enabled),
    sort: Number(tool.sort || 0),
  };
}

function currentClientOs() {
  const platform = navigator.platform.toLowerCase();
  if (platform.includes('mac')) {
    return 'macos';
  }
  if (platform.includes('linux')) {
    return 'linux';
  }
  return 'windows';
}

function defaultShell() {
  return navigator.platform.toLowerCase().includes('win') ? 'cmd' : 'sh';
}

function shellLabel(shell) {
  const labels = {
    cmd: 'CMD',
    powershell: 'PowerShell',
    bash: 'Bash',
    sh: 'Sh',
    zsh: 'Zsh',
  };
  return labels[shell] || shell || '-';
}

function handlePanelActivity() {
  resetInlineIdleTimer();
}

function handleSearchInputFocus() {
  isSearchInputFocused.value = true;
  clearInlineIdleTimer();
}

function handleSearchInputBlur() {
  isSearchInputFocused.value = false;
  resetInlineIdleTimer();
}

function handleSearchResultsEnter() {
  isSearchResultsActive.value = true;
  clearInlineIdleTimer();
}

function handleSearchResultsLeave() {
  isSearchResultsActive.value = false;
  resetInlineIdleTimer();
}

function clearInlineIdleTimer() {
  if (inlineIdleTimer) {
    clearTimeout(inlineIdleTimer);
    inlineIdleTimer = null;
  }
}

function resetInlineIdleTimer() {
  if (isGlobalSearch.value) {
    return;
  }
  clearInlineIdleTimer();
  if (isSearchInputFocused.value || isSearchResultsActive.value) {
    return;
  }
  inlineIdleTimer = setTimeout(() => {
    router.push({ name: 'Home' });
  }, 5000);
}
</script>

<template>
  <div
    class="search-panel"
    :class="[`search-panel-${mode}`, { 'has-results': shouldShowResults }]"
    @focusin="handlePanelActivity"
    @focusout="handlePanelActivity"
    @click="handlePanelActivity"
    @pointerdown="handlePanelActivity"
    @pointermove="handlePanelActivity"
    @input="handlePanelActivity"
  >
    <el-input
      ref="searchInput"
      v-model="keyword"
      placeholder="搜索"
      clearable
      class="search-input"
      @keydown.down.prevent="selectNext"
      @keydown.up.prevent="selectPrev"
      @keydown.enter.prevent="runSelected"
      @focus="handleSearchInputFocus"
      @blur="handleSearchInputBlur"
    />

    <div
      v-if="shouldShowResults"
      class="search-tools-content"
      @mouseenter="handleSearchResultsEnter"
      @mouseleave="handleSearchResultsLeave"
      @focusin="handleSearchResultsEnter"
      @focusout="handleSearchResultsLeave"
    >
      <el-dropdown
        v-for="(tools, index) in filteredTools"
        :key="`${tools.__searchType}-${tools.id}`"
        ref="toolsDropdowns"
        class="search-tool-dropdown"
        trigger="contextmenu"
        @visible-change="(visible) => toolsDropdownClick(visible, index)"
      >
        <div
          class="tools-button el-button search-tool-button"
          :class="{ 'is-selected': selectedIndex === index }"
          :data-search-index="index"
          @mouseenter="handleToolMouseEnter(index)"
          @click="toolsHandle('run', tools)"
        >
          <el-tooltip :content="tools.target" effect="light">
            <div class="search-tools-content-tool">
              <span class="search-tool-main">
                <img
                  v-if="tools.icon"
                  class="tools-button-icon"
                  :src="tools.icon"
                  alt="工具图标"
                  @error="handleSearchIconError(tools)"
                />
                <el-icon v-else class="tools-button-icon search-tool-fallback-icon"><Monitor /></el-icon>
                <span class="tools-button-title">{{ tools.name }}</span>
              </span>
              <span v-if="tools.meta" class="search-tool-meta">{{ tools.meta }}</span>
            </div>
          </el-tooltip>
        </div>

        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item :icon="ArrowRight" @click="toolsHandle('run', tools)">运行</el-dropdown-item>
            <template v-if="tools.__searchType === 'normal'">
              <el-dropdown-item :icon="ArrowRightBold" @click="toolsHandle('runAsAdmin', tools)">以管理员运行</el-dropdown-item>
              <el-dropdown-item :icon="Link" @click="toolsHandle('copyPath', tools)">复制完整路径</el-dropdown-item>
              <el-dropdown-item :icon="Folder" @click="toolsHandle('openDir', tools)">打开文件位置</el-dropdown-item>
              <el-dropdown-item :icon="Monitor" @click="toolsHandle('openCmd', tools)">打开命令行</el-dropdown-item>
              <el-dropdown-item :icon="Platform" @click="toolsHandle('openCmdAsAdmin', tools)">以管理员打开命令行</el-dropdown-item>
            </template>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>
  </div>
</template>

<style scoped>
.search-panel {
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
  background: var(--st-content-bg);
}

.search-panel-inline {
  height: 100%;
}

.search-panel-global {
  height: auto;
  background: transparent;
  overflow: visible;
}

.search-panel-global.has-results {
  background: var(--st-content-bg);
}

.search-panel-global .search-tools-content {
  max-height: 360px;
}

.search-input {
  flex: 0 0 auto;
}

.search-input :deep(.el-input__wrapper) {
  height: 42px;
  min-height: 42px;
  border-radius: 0;
}

.search-panel-global .search-input :deep(.el-input__wrapper) {
  min-height: 42px;
  box-sizing: border-box;
  border: 1px solid var(--st-border-soft);
  border-radius: 0;
  background: var(--st-content-bg);
  box-shadow: none;
}

.search-panel-global .search-input :deep(.el-input__wrapper:hover),
.search-panel-global .search-input :deep(.el-input__wrapper.is-focus) {
  border-color: var(--el-color-primary-light-5);
  box-shadow: none;
}

.search-input :deep(.el-input__inner) {
  font-size: 16px;
}

.search-tools-content {
  height: auto;
  flex: 0 1 auto;
  min-height: 0;
  max-height: 360px;
  overflow-x: hidden;
  overflow-y: auto;
  border-top: 1px solid var(--st-border-soft);
  background: var(--st-content-bg);
}

.search-panel-global .search-tools-content {
  margin-top: 0;
  border: 1px solid var(--st-border-soft);
  border-top: 0;
  border-radius: 0;
  background: var(--st-content-bg);
}

.search-panel-inline .search-tools-content {
  flex: 1 1 auto;
  height: auto;
  max-height: none;
}

.search-tool-dropdown,
.search-tool-button {
  width: 100%;
}

.search-tool-button {
  justify-content: flex-start;
  background: var(--st-content-bg) !important;
}

.search-tool-button:hover {
  background: var(--st-hover-bg) !important;
}

.search-tool-button.is-selected {
  background: var(--st-active-bg) !important;
  border-radius: 0 !important;
}

.search-tool-main {
  display: flex;
  align-items: center;
  min-width: 0;
}

.search-tool-meta {
  flex: 0 0 auto;
  margin-left: 8px;
  color: var(--el-text-color-secondary);
}

.search-tool-fallback-icon {
  display: inline-flex;
  justify-content: center;
  align-items: center;
  color: var(--el-color-primary);
  font-size: 32px;
}

.search-tool-fallback-icon :deep(svg) {
  width: 32px;
  height: 32px;
}
</style>

