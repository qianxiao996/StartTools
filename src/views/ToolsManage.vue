<template>
  <main class="tools-manage-page">
    <header class="manage-tabbar">
      <el-segmented v-model="activeTab" :options="manageTabOptions" />
    </header>

    <section class="tools-manage-content">
      <template v-if="activeTab === 'builtin'">
        <header class="tools-manage-toolbar">
          <el-input
            v-model="keyword"
            class="tools-manage-search"
            clearable
            placeholder="搜索内置工具"
          />
          <el-button :icon="Refresh" @click="loadTools">刷新</el-button>
          <el-button :icon="Download" @click="exportBuiltinTools">导出</el-button>
          <el-button :icon="Upload" @click="importBuiltinTools">导入</el-button>
          <el-button type="primary" :icon="DocumentAdd" @click="openEditor()">新增工具</el-button>
          <div class="builtin-os-config">
            <span class="builtin-os-label">系统</span>
            <el-segmented
              v-model="builtinToolConfig.Builtin_Tools_OS"
              :options="osOptions"
              @change="saveBuiltinToolConfig"
            />
          </div>
        </header>

        <section class="tools-manage-table-wrap">
          <el-table class="tools-manage-table" :data="filteredTools" stripe border>
            <el-table-column prop="name" label="名称" min-width="170" show-overflow-tooltip />
            <el-table-column label="系统" width="92">
              <template #default="{ row }">{{ osLabel(row.os) }}</template>
            </el-table-column>
            <el-table-column label="解释器" width="120">
              <template #default="{ row }">{{ shellLabel(row.shell) }}</template>
            </el-table-column>
            <el-table-column prop="content" label="脚本" min-width="380" show-overflow-tooltip />
            <el-table-column label="启用" width="76" align="center">
              <template #default="{ row }">
                <el-switch v-model="row.enabled" @change="saveTool(row)" />
              </template>
            </el-table-column>
            <el-table-column prop="sort" label="排序" width="82" align="center" />
            <el-table-column label="操作" width="220" fixed="right">
              <template #default="{ row }">
                <el-button-group>
                  <el-button size="small" :icon="ArrowRight" @click="runTool(row)">运行</el-button>
                  <el-button size="small" :icon="Edit" @click="openEditor(row)">编辑</el-button>
                  <el-button size="small" type="danger" :icon="Delete" @click="deleteTool(row)">删除</el-button>
                </el-button-group>
              </template>
            </el-table-column>
          </el-table>
        </section>
      </template>

      <template v-else>
        <section class="starttools-plugin-page">
          <header class="tools-manage-toolbar">
            <el-input
              v-model="pluginKeyword"
              class="tools-manage-search"
              clearable
              placeholder="搜索 StartTools 插件"
            />
            <el-button :icon="Refresh" @click="loadStartToolsPlugins">刷新</el-button>
            <el-button :icon="Download" @click="exportStartToolsPlugins">导出</el-button>
            <el-button :icon="Upload" @click="importStartToolsPlugins">导入</el-button>
            <el-button type="primary" :icon="DocumentAdd" @click="addStartToolsPlugin">新增插件</el-button>
            <div class="plugin-http-config">
              <span class="plugin-http-label">IP</span>
              <el-input
                v-model="pluginHttpConfig.Plugin_Http_Host"
                class="plugin-http-host"
                spellcheck="false"
              />
              <span class="plugin-http-label">Port</span>
              <el-input-number
                v-model="pluginHttpConfig.Plugin_Http_Port"
                class="plugin-http-port"
                :min="1"
                :max="65535"
                :step="1"
                controls-position="right"
              />
              <el-button :icon="Check" @click="savePluginHttpConfig">保存</el-button>
            </div>
          </header>

          <section class="tools-manage-table-wrap">
            <el-table class="tools-manage-table" :data="filteredStartToolsPlugins" stripe border>
              <el-table-column prop="plugin_id" label="ID" min-width="190" show-overflow-tooltip />
              <el-table-column prop="name" label="名称" min-width="150" show-overflow-tooltip />
              <el-table-column prop="main" label="入口" min-width="150" show-overflow-tooltip />
              <el-table-column label="Logo" min-width="160" show-overflow-tooltip>
                <template #default="{ row }">
                  <span class="plugin-logo-cell">
                    <img
                      v-if="row.logo"
                      class="plugin-logo"
                      :src="row.logo_url || row.logo"
                      alt=""
                      @load="showLoadedImage"
                      @error="hideBrokenImage"
                    />
                    <span class="plugin-logo-text">{{ row.logo }}</span>
                  </span>
                </template>
              </el-table-column>
              <el-table-column prop="preload" label="Preload" min-width="150" show-overflow-tooltip />
              <el-table-column label="窗口大小" width="150">
                <template #default="{ row }">
                  {{ pluginWindowSizeLabel(row) }}
                </template>
              </el-table-column>
              <el-table-column prop="sort" label="排序" width="82" align="center" />
              <el-table-column label="操作" width="306" fixed="right">
                <template #default="{ row }">
                  <el-button-group class="row-actions">
                    <el-button size="small" :icon="ArrowRight" @click="runStartToolsPlugin(row)">运行</el-button>
                    <el-button size="small" :icon="FolderOpened" @click="openStartToolsPluginDir(row)">目录</el-button>
                    <el-button size="small" :icon="Edit" @click="openPluginEditor(row)">编辑</el-button>
                    <el-button size="small" type="danger" :icon="Delete" @click="deleteStartToolsPlugin(row)">删除</el-button>
                  </el-button-group>
                </template>
              </el-table-column>
            </el-table>
          </section>
        </section>
      </template>
    </section>

    <el-dialog
      v-model="editorVisible"
      :title="editingTool.id > 0 ? '编辑内置工具' : '新增内置工具'"
      width="680px"
      destroy-on-close
    >
      <el-form :model="editingTool" label-width="76px">
        <el-form-item label="名称">
          <el-input v-model="editingTool.name" />
        </el-form-item>
        <el-form-item label="系统">
          <el-segmented
            v-model="editingTool.os"
            :options="osOptions"
            @change="handleToolOsChange"
          />
        </el-form-item>
        <el-form-item label="解释器">
          <el-segmented v-model="editingTool.shell" :options="shellOptions" />
        </el-form-item>
        <el-form-item label="运行目录">
          <el-input
            v-model="editingTool.working_dir"
            clearable
            placeholder="留空时自动推断运行目录"
          />
        </el-form-item>
        <el-form-item label="脚本">
          <el-input v-model="editingTool.content" type="textarea" :rows="9" spellcheck="false" />
        </el-form-item>
        <el-form-item label="排序">
          <el-input-number v-model="editingTool.sort" :min="0" controls-position="right" />
        </el-form-item>
        <el-form-item label="启用">
          <el-switch v-model="editingTool.enabled" />
        </el-form-item>
      </el-form>
      <template #footer>
        <div class="dialog-actions">
          <el-button :icon="Close" @click="editorVisible = false">取消</el-button>
          <el-button type="primary" :icon="Check" @click="saveEditingTool">保存</el-button>
        </div>
      </template>
    </el-dialog>

    <el-dialog
      v-model="pluginEditorVisible"
      :title="editingPlugin.id > 0 ? '编辑 StartTools 插件' : '新增 StartTools 插件'"
      width="620px"
      destroy-on-close
    >
      <el-form :model="editingPlugin" label-width="84px">
        <el-form-item label="ID">
          <el-input v-model="editingPlugin.plugin_id" readonly placeholder="保存时自动生成" />
        </el-form-item>
        <el-form-item label="名称">
          <el-input v-model="editingPlugin.name" />
        </el-form-item>
        <el-form-item label="入口">
          <el-input v-model="editingPlugin.main" />
        </el-form-item>
        <el-form-item label="Logo">
          <el-input v-model="editingPlugin.logo" />
        </el-form-item>
        <el-form-item label="Preload">
          <el-input v-model="editingPlugin.preload" />
        </el-form-item>
        <el-form-item label="窗口大小">
          <el-segmented
            v-model="editingPlugin.window_mode"
            :options="pluginWindowModeOptions"
            @change="handlePluginWindowModeChange"
          />
        </el-form-item>
        <el-form-item v-if="editingPlugin.window_mode !== 'maximized'" label="宽高">
          <div class="plugin-window-size-inputs">
            <el-input-number
              v-model="editingPlugin.window_width"
              :min="360"
              :max="7680"
              :step="20"
              controls-position="right"
            />
            <el-input-number
              v-model="editingPlugin.window_height"
              :min="240"
              :max="4320"
              :step="20"
              controls-position="right"
            />
          </div>
        </el-form-item>
        <el-form-item label="排序">
          <el-input-number v-model="editingPlugin.sort" :min="0" controls-position="right" />
        </el-form-item>
      </el-form>
      <template #footer>
        <div class="dialog-actions">
          <el-button :icon="Close" @click="pluginEditorVisible = false">取消</el-button>
          <el-button type="primary" :icon="Check" @click="saveEditingPlugin">保存</el-button>
        </div>
      </template>
    </el-dialog>
  </main>
</template>

<script setup>
import '../css/white.css';
import { computed, onMounted, reactive, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { emit } from '@tauri-apps/api/event';
import { ask, message, open, save } from '@tauri-apps/plugin-dialog';
import {
  ArrowRight,
  Check,
  Close,
  Delete,
  DocumentAdd,
  Download,
  Edit,
  FolderOpened,
  Refresh,
  Upload,
} from '@element-plus/icons-vue';

const activeTab = ref('builtin');
const manageTabOptions = [
  { label: '内置工具', value: 'builtin' },
  { label: 'StartTools 插件', value: 'starttools' },
];
const keyword = ref('');
const pluginKeyword = ref('');
const tools = ref([]);
const editorVisible = ref(false);
const starttoolsPlugins = ref([]);
const pluginEditorVisible = ref(false);
const pluginHttpConfig = reactive({
  Plugin_Http_Host: '127.0.0.1',
  Plugin_Http_Port: 13678,
});
const builtinToolConfig = reactive({
  Builtin_Tools_OS: currentClientOs(),
});
const pluginWindowModeOptions = [
  { label: '默认', value: 'default' },
  { label: '最大化', value: 'maximized' },
];
const osOptions = [
  { label: 'Windows', value: 'windows' },
  { label: 'Linux', value: 'linux' },
  { label: 'macOS', value: 'macos' },
];
const shellOptions = [
  { label: 'CMD', value: 'cmd' },
  { label: 'PowerShell', value: 'powershell' },
  { label: 'Bash', value: 'bash' },
  { label: 'Sh', value: 'sh' },
  { label: 'Zsh', value: 'zsh' },
];

const editingTool = reactive(createEmptyTool());
const editingPlugin = reactive(createEmptyPlugin());

const filteredTools = computed(() => {
  const query = keyword.value.trim().toLowerCase();
  return tools.value.filter((tool) => {
    if (!query) {
      return true;
    }
    return [tool.name, tool.content].some((value) =>
      String(value || '').toLowerCase().includes(query)
    );
  });
});
const filteredStartToolsPlugins = computed(() => {
  const query = pluginKeyword.value.trim().toLowerCase();
  if (!query) {
    return starttoolsPlugins.value;
  }
  return starttoolsPlugins.value.filter((plugin) => {
    const values = [
      plugin.plugin_id,
      plugin.name,
      plugin.main,
      plugin.logo,
      plugin.preload,
    ];
    return values.some((value) => String(value || '').toLowerCase().includes(query));
  });
});

onMounted(async () => {
  await Promise.all([loadTools(), loadStartToolsPlugins(), loadPluginHttpConfig(), loadBuiltinToolConfig()]);
});

function createEmptyTool() {
  return {
    id: -1,
    name: '',
    run_type: 'script',
    os: currentClientOs(),
    shell: defaultShellForOs(currentClientOs()),
    content: '',
    working_dir: '',
    enabled: true,
    sort: 0,
  };
}

function createEmptyPlugin() {
  return {
    id: -1,
    plugin_id: '',
    name: 'StartTools',
    main: '',
    logo: '',
    preload: '',
    window_mode: 'default',
    window_width: 1080,
    window_height: 680,
    sort: starttoolsPlugins.value.length,
  };
}

async function loadTools() {
  tools.value = await invoke('load_all_builtin_tools');
}

function firstPath(value) {
  if (!value) {
    return '';
  }
  return Array.isArray(value) ? value[0] || '' : value;
}

async function exportBuiltinTools() {
  const path = await save({
    title: '导出内置工具',
    defaultPath: 'starttools-builtin-tools.json',
    filters: [{ name: '内置工具', extensions: ['json'] }],
  });
  if (!path) {
    return;
  }
  try {
    await invoke('export_builtin_tools', { path });
    await message('内置工具已导出', { title: '工具管理', type: 'info' });
  } catch (error) {
    await message('导出内置工具失败: ' + error, { title: '工具管理', type: 'error' });
  }
}

async function importBuiltinTools() {
  const ok = await ask('导入会覆盖当前内置工具，是否继续？', {
    title: '导入内置工具',
    kind: 'warning',
  });
  if (!ok) {
    return;
  }
  const selected = await open({
    title: '导入内置工具',
    multiple: false,
    directory: false,
    filters: [{ name: '内置工具', extensions: ['json'] }],
  });
  const path = firstPath(selected);
  if (!path) {
    return;
  }
  try {
    await invoke('import_builtin_tools', { path });
    await loadTools();
    await emit('builtinToolsUpdated');
    await message('内置工具已导入', { title: '工具管理', type: 'info' });
  } catch (error) {
    await message('导入内置工具失败: ' + error, { title: '工具管理', type: 'error' });
  }
}

async function loadBuiltinToolConfig() {
  const config = await invoke('load_config');
  builtinToolConfig.Builtin_Tools_OS = normalizeToolOs(config.Builtin_Tools_OS || currentClientOs());
}

async function saveBuiltinToolConfig() {
  const nextConfig = {
    Builtin_Tools_OS: normalizeToolOs(builtinToolConfig.Builtin_Tools_OS),
  };
  const result = await invoke('update_config', { config: nextConfig });
  if (result === 'ok') {
    Object.assign(builtinToolConfig, nextConfig);
    await emit('configUpdated', nextConfig);
    await emit('builtinToolsUpdated');
  } else {
    await message('内置工具系统设置保存失败: ' + result, { title: '工具管理', type: 'error' });
  }
}

function openEditor(tool = null) {
  Object.assign(editingTool, tool ? JSON.parse(JSON.stringify(tool)) : createEmptyTool());
  normalizeEditingToolSystem();
  editorVisible.value = true;
}

async function saveEditingTool() {
  editingTool.run_type = 'script';
  normalizeEditingToolSystem();
  if (!editingTool.name.trim()) {
    await message('请输入工具名称', { title: '工具管理', type: 'error' });
    return;
  }
  if (!editingTool.content.trim()) {
    await message('请输入脚本内容', { title: '工具管理', type: 'error' });
    return;
  }
  await saveTool(editingTool);
  editorVisible.value = false;
}

async function saveTool(tool) {
  await invoke('update_builtin_tool', { tool });
  await loadTools();
  await emit('builtinToolsUpdated');
}

async function deleteTool(tool) {
  const ok = await ask(`确认删除内置工具《${tool.name}》吗？`, {
    title: '工具管理',
    kind: 'warning',
  });
  if (!ok) {
    return;
  }
  await invoke('delete_builtin_tool', { toolId: Number(tool.id) });
  await loadTools();
  await emit('builtinToolsUpdated');
}

async function runTool(tool) {
  try {
    await invoke('run_builtin_tool', { tool });
  } catch (error) {
    await message(String(error), { title: '运行失败', type: 'error' });
  }
}

function shellLabel(shell) {
  return shellOptions.find((option) => option.value === shell)?.label || shell || '-';
}

function osLabel(os) {
  return osOptions.find((option) => option.value === os)?.label || os || '-';
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

function defaultShellForOs(os) {
  if (os === 'windows') {
    return 'cmd';
  }
  return 'sh';
}

function normalizeToolOs(os) {
  if (os === 'linux' || os === 'macos') {
    return os;
  }
  return 'windows';
}

function normalizeEditingToolSystem() {
  editingTool.os = normalizeToolOs(editingTool.os);
  if (!shellOptions.some((option) => option.value === editingTool.shell)) {
    editingTool.shell = defaultShellForOs(editingTool.os);
  }
}

function handleToolOsChange(os) {
  editingTool.os = normalizeToolOs(os);
  editingTool.shell = defaultShellForOs(editingTool.os);
}

function hideBrokenImage(event) {
  event.currentTarget.style.display = 'none';
}

function showLoadedImage(event) {
  event.currentTarget.style.display = '';
}

async function loadStartToolsPlugins() {
  starttoolsPlugins.value = await invoke('load_starttools_plugins');
}

async function exportStartToolsPlugins() {
  const path = await save({
    title: '导出 StartTools 插件',
    defaultPath: 'starttools-plugins.stplugins',
    filters: [{ name: 'StartTools 插件', extensions: ['stplugins'] }],
  });
  if (!path) {
    return;
  }
  try {
    await invoke('export_starttools_plugins', { path });
    await message('StartTools 插件已导出', { title: 'StartTools 插件', type: 'info' });
  } catch (error) {
    await message('导出 StartTools 插件失败: ' + error, { title: 'StartTools 插件', type: 'error' });
  }
}

async function importStartToolsPlugins() {
  const ok = await ask('导入会覆盖当前 StartTools 插件记录和插件目录，是否继续？', {
    title: '导入 StartTools 插件',
    kind: 'warning',
  });
  if (!ok) {
    return;
  }
  const selected = await open({
    title: '导入 StartTools 插件',
    multiple: false,
    directory: false,
    filters: [{ name: 'StartTools 插件', extensions: ['stplugins'] }],
  });
  const path = firstPath(selected);
  if (!path) {
    return;
  }
  try {
    await invoke('import_starttools_plugins', { path });
    await loadStartToolsPlugins();
    await emit('startToolsPluginsUpdated');
    await message('StartTools 插件已导入', { title: 'StartTools 插件', type: 'info' });
  } catch (error) {
    await message('导入 StartTools 插件失败: ' + error, { title: 'StartTools 插件', type: 'error' });
  }
}

async function loadPluginHttpConfig() {
  const config = await invoke('load_config');
  pluginHttpConfig.Plugin_Http_Host = config.Plugin_Http_Host || '127.0.0.1';
  const port = Number(config.Plugin_Http_Port || 13678);
  pluginHttpConfig.Plugin_Http_Port = Number.isNaN(port) ? 13678 : port;
}

async function savePluginHttpConfig() {
  const nextConfig = {
    Plugin_Http_Host: String(pluginHttpConfig.Plugin_Http_Host || '127.0.0.1').trim() || '127.0.0.1',
    Plugin_Http_Port: Number(pluginHttpConfig.Plugin_Http_Port || 13678),
  };
  const result = await invoke('update_config', { config: nextConfig });
  if (result === 'ok') {
    Object.assign(pluginHttpConfig, nextConfig);
    const activeServer = await invoke('restart_starttools_plugin_http_server');
    await emit('configUpdated', nextConfig);
    await message(`HTTP server active: ${activeServer.host}:${activeServer.port}`, { title: 'StartTools', type: 'info' });
  } else {
    await message('插件 HTTP 设置保存失败: ' + result, { title: 'StartTools 插件', type: 'error' });
  }
}

async function runStartToolsPlugin(plugin) {
  try {
    await invoke('open_starttools_plugin', { pluginId: plugin.plugin_id });
  } catch (error) {
    await message('打开 StartTools 插件失败: ' + error, { title: '运行失败', type: 'error' });
  }
}
async function openStartToolsPluginDir(plugin) {
  try {
    await invoke('open_starttools_plugin_dir', { pluginId: plugin.plugin_id });
  } catch (error) {
    await message('打开插件目录失败: ' + error, { title: 'StartTools 插件', type: 'error' });
  }
}
function addStartToolsPlugin() {
  openPluginEditor();
}

function openPluginEditor(plugin = null) {
  Object.assign(editingPlugin, plugin ? JSON.parse(JSON.stringify(plugin)) : createEmptyPlugin());
  normalizeEditingPluginWindow();
  pluginEditorVisible.value = true;
}

async function saveEditingPlugin() {
  if (!editingPlugin.name.trim()) {
    await message('请输入插件名称', { title: 'StartTools 插件', type: 'error' });
    return;
  }
  normalizeEditingPluginWindow();
  await invoke('update_starttools_plugin', { plugin: editingPlugin });
  pluginEditorVisible.value = false;
  await loadStartToolsPlugins();
  await emit('startToolsPluginsUpdated');
}

async function deleteStartToolsPlugin(plugin) {
  const ok = await ask(`确认删除插件《${plugin.name}》吗？`, {
    title: 'StartTools 插件',
    kind: 'warning',
  });
  if (!ok) {
    return;
  }
  await invoke('delete_starttools_plugin', { pluginId: Number(plugin.id) });
  await loadStartToolsPlugins();
  await emit('startToolsPluginsUpdated');
}

function normalizeEditingPluginWindow() {
  editingPlugin.window_mode = editingPlugin.window_mode === 'maximized' ? 'maximized' : 'default';
  editingPlugin.window_width = clampNumber(editingPlugin.window_width, 1080, 360, 7680);
  editingPlugin.window_height = clampNumber(editingPlugin.window_height, 680, 240, 4320);
}

function handlePluginWindowModeChange(mode) {
  if (mode !== 'default') {
    return;
  }
  editingPlugin.window_width = validSizeOrDefault(editingPlugin.window_width, 1080, 360, 7680);
  editingPlugin.window_height = validSizeOrDefault(editingPlugin.window_height, 680, 240, 4320);
}

function clampNumber(value, fallback, min, max) {
  const number = Number(value);
  if (!Number.isFinite(number)) {
    return fallback;
  }
  return Math.min(Math.max(Math.round(number), min), max);
}

function validSizeOrDefault(value, fallback, min, max) {
  const number = Number(value);
  if (!Number.isFinite(number) || number < min || number > max) {
    return fallback;
  }
  return Math.round(number);
}

function pluginWindowSizeLabel(plugin) {
  if (plugin.window_mode === 'maximized') {
    return '最大化';
  }
  const width = plugin.window_width || 1080;
  const height = plugin.window_height || 680;
  return `${width} x ${height}`;
}
</script>

<style scoped>
.tools-manage-page {
  display: flex;
  flex-direction: column;
  height: 100vh;
  min-height: 0;
  overflow: hidden;
  padding: 10px;
  box-sizing: border-box;
  background: var(--st-content-bg);
}

.manage-tabbar {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  height: 36px;
  margin-bottom: 10px;
}

.tools-manage-content {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  min-height: 0;
}

.tools-manage-toolbar {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 8px;
  margin-bottom: 10px;
}

.tools-manage-search {
  width: 260px;
}

.plugin-http-config,
.builtin-os-config {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin-left: auto;
  min-width: 0;
}

.plugin-http-label,
.builtin-os-label {
  color: var(--el-text-color-secondary);
  font-size: 12px;
  white-space: nowrap;
}

.plugin-http-host {
  width: 132px;
}

.plugin-http-port {
  width: 118px;
}

.tools-manage-table-wrap {
  height: calc(100% - 44px);
  min-height: 0;
  overflow: hidden;
}

.tools-manage-table {
  height: 100%;
}

.tools-manage-table :deep(.el-table__inner-wrapper),
.tools-manage-table :deep(.el-scrollbar),
.tools-manage-table :deep(.el-scrollbar__wrap) {
  height: 100%;
}

.row-actions {
  display: inline-flex;
  flex-wrap: nowrap;
  white-space: nowrap;
}

.plugin-logo-cell {
  display: inline-flex;
  align-items: center;
  max-width: 100%;
  gap: 6px;
  min-width: 0;
}

.plugin-logo {
  width: 22px;
  height: 22px;
  flex: 0 0 auto;
  object-fit: contain;
}

.plugin-logo-text {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.plugin-window-size-inputs {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
  width: 100%;
}

.starttools-plugin-page {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

:deep(.el-dialog__body) {
  padding-bottom: 8px;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding-top: 12px;
  border-top: 1px solid var(--el-border-color-lighter);
}
</style>
