<template>
  <div
    class="toolsEdit-form-div"
    :class="{ 'is-dragging-file': isDraggingFile }"
  >
    <el-form :model="tool" label-width="auto" class="toolsEdit-form">
      <el-form-item label="图标">
        <div class="toolsEdit-icon">
          <img v-if="tool.icon" :src="tool.icon" @click="beforeAvatarUpload" />
          <el-icon v-else><Plus /></el-icon>
          <el-button-group class="ml-4">
            <el-button :icon="RefreshLeft" :loading="isLoadingIcon" @click="RefreshIcon" />
          </el-button-group>
        </div>
      </el-form-item>

      <el-form-item label="名称">
        <el-input v-model="tool.name">
          <template #append>
            <el-button :icon="Close" @click="clearName" />
          </template>
        </el-input>
      </el-form-item>

      <el-form-item label="目标">
        <el-input
          v-model="tool.target"
          placeholder="选择文件，或直接拖入文件"
        >
          <template #append>
            <el-button :icon="FolderOpened" @click="SelectFile" />
          </template>
        </el-input>
      </el-form-item>

      <el-form-item label="参数">
        <el-input v-model="tool.parameters" type="textarea" />
      </el-form-item>

      <el-form-item label="管理员">
        <el-segmented v-model="tool.isadmin" :options="adminOptions" />
      </el-form-item>

      <el-form-item>
        <div class="toolsEdit-button dialog-actions">
          <el-button :icon="Close" @click="closeWindow()">取消</el-button>
          <el-button type="primary" :icon="Check" @click="submitForm()">保存</el-button>
        </div>
      </el-form-item>
    </el-form>
    <div v-if="isDraggingFile" class="drag-file-tip">松开添加目标文件</div>
  </div>
</template>

<script setup>
import { open } from '@tauri-apps/plugin-dialog';
import { nextTick, onMounted, onUnmounted, ref } from 'vue';
import { listen } from '@tauri-apps/api/event';
import '../css/white.css';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { LogicalSize } from '@tauri-apps/api/window';
import { Plus, Check, Close, FolderOpened, RefreshLeft } from '@element-plus/icons-vue';
import { invoke } from "@tauri-apps/api/core";
import { message } from '@tauri-apps/plugin-dialog';
import { emit } from '@tauri-apps/api/event';
import { basename } from '@tauri-apps/api/path';

let unlisten;
let unlistenDragDrop;
const currentWindow = getCurrentWebviewWindow();
const isDraggingFile = ref(false);
const isLoadingIcon = ref(false);
const adminOptions = [
  { label: '普通', value: false },
  { label: '管理员', value: true },
];
const tool = ref({
  name: '',
  icon: '',
  target: '',
  parameters: '',
  isadmin: false,
  number: 0,
  menu_id: '',
  tags_id: '',
});

async function resizeWindowToContent() {
  await nextTick();
  requestAnimationFrame(async () => {
    const root = document.querySelector('.toolsEdit-form-div');
    const form = document.querySelector('.toolsEdit-form');
    const contentHeight = Math.ceil(
      root?.getBoundingClientRect().height ||
      root?.scrollHeight ||
      form?.getBoundingClientRect().height ||
      form?.scrollHeight ||
      0
    );
    const height = Math.max(240, contentHeight + 2);
    await currentWindow.setSize(new LogicalSize(600, height));
    await currentWindow.center();
  });
}

async function applyTargetFile(file) {
  const path = firstFilePath(file);
  if (!path) {
    return;
  }

  const fileName = await basename(path);
  tool.value.target = path;
  tool.value.name = fileName;
  await loadIcon(path);
}

function firstFilePath(value) {
  if (!value) {
    return '';
  }
  if (Array.isArray(value)) {
    return firstFilePath(value[0]);
  }
  if (typeof value === 'string') {
    return value;
  }
  return value.path || '';
}

async function loadIcon(filename) {
  if (!filename) {
    return false;
  }

  isLoadingIcon.value = true;
  const data = await invoke("get_exe_icon", { filename });
  isLoadingIcon.value = false;

  if (data && !data.startsWith('Error')) {
    tool.value.icon = data;
    return true;
  }

  await message('图标获取失败!\n' + data, { title: 'Error', type: 'error' });
  return false;
}

onMounted(async () => {
  try {
    unlisten = await listen('receiveToolData', (event) => {
      if (!event.payload?.tool) {
        return;
      }
      tool.value = event.payload.tool;
      resizeWindowToContent();
    });
    await emit('toolsEditReady', { label: currentWindow.label });

    unlistenDragDrop = await currentWindow.onDragDropEvent(async (event) => {
      if (event.payload.type === 'enter' || event.payload.type === 'over') {
        isDraggingFile.value = true;
        return;
      }

      if (event.payload.type === 'leave') {
        isDraggingFile.value = false;
        return;
      }

      if (event.payload.type === 'drop') {
        isDraggingFile.value = false;
        await applyTargetFile(event.payload.paths);
      }
    });
  } catch (error) {
    console.error('监听事件失败:', error);
  }
});

onUnmounted(() => {
  unlisten?.();
  unlistenDragDrop?.();
});

function clearName() {
  tool.value.name = '';
}

async function submitForm() {
  tool.value.isadmin = tool.value.isadmin === true || tool.value.isadmin === 1 || tool.value.isadmin === '1';
  if (tool.value.target == null || tool.value.target === "") {
    await message('请输入目标路径!', { title: 'Error', type: 'error' });
    return;
  }
  if (tool.value.name == null || tool.value.name === "") {
    await message('请输入目标名称!', { title: 'Error', type: 'error' });
    return;
  }

  const data = await invoke("update_tool", { tool: tool.value });
  if (data === "ok") {
    await emit('toolUpdated', { action: tool.value.id > 0 ? 'edit' : 'add', tool: tool.value });
    currentWindow.close();
  } else {
    await message('保存失败!\n' + data, { title: 'Error', type: 'error' });
  }
}

function closeWindow() {
  currentWindow.close();
}

const beforeAvatarUpload = async () => {
  const file = await open({
    multiple: false,
    directory: false,
  });
  await loadIcon(firstFilePath(file));
};

async function SelectFile() {
  const file = await open({
    multiple: false,
    directory: false,
  });
  await applyTargetFile(file);
}

async function RefreshIcon() {
  await loadIcon(tool.value.target);
}
</script>

<style scoped>
.toolsEdit-form-div {
  transition: background-color 0.16s ease, outline-color 0.16s ease;
  outline: 1px dashed transparent;
  outline-offset: -8px;
}

.toolsEdit-form-div.is-dragging-file {
  background-color: var(--el-color-primary-light-9);
  outline-color: var(--el-color-primary);
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 8px;
  width: 100%;
  margin-top: 4px;
  padding-top: 12px;
  border-top: 1px solid var(--el-border-color-lighter);
}

:deep(.el-form-item:last-child) {
  margin-bottom: 0;
}

.drag-file-tip {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
  color: var(--el-color-primary);
  font-size: 14px;
  font-weight: 600;
  background: color-mix(in srgb, var(--el-color-primary) 10%, transparent);
}
</style>
