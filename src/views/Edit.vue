<template>
<div class="global-edit">
    <el-form :model="edit_obj" label-width="auto" class="global-edit-form">
        <el-form-item v-if="edit_type=='tag'" label="分类">
            <el-select
                v-model="edit_obj.menu_id"
                placeholder="选择"
                filterable
                style="width: 100%"
                >
                <el-option
                    v-for="item in allmenu"
                    :key="item.id"
                    :label="item.name"
                    :value="item.id"
                />
                </el-select>
        </el-form-item>
        <el-form-item label="名称">
            <el-input
                v-model="edit_obj.name"
                clearable
            >
            </el-input>
        </el-form-item>
        <el-form-item label="排序">
            <!-- <el-input
                v-model="edit_obj.sort"
                clearable
            >
            </el-input> -->
            <el-input-number
                v-model="edit_obj.sort"
                :min="1"
                :max="9999"
                controls-position="right"
                style="width: 100%;"
            />
        </el-form-item>
        <el-form-item>
            <div class="global-edit-button dialog-actions">
                <el-button :icon="Close" @click="closeWindow()">取消</el-button>
                <el-button type="primary" :icon="Check" @click="submitForm()">
                    保存
                </el-button>
            </div>
        </el-form-item>
    </el-form>
</div>
</template>
<script setup>
import { emit } from '@tauri-apps/api/event';
import { message } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import { invoke } from "@tauri-apps/api/core";
import '../css/white.css';
import { nextTick, onMounted, onUnmounted, ref } from 'vue';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { LogicalSize } from '@tauri-apps/api/window';
import { Check, Close } from '@element-plus/icons-vue';
let unlisten;
const currentWindow = getCurrentWebviewWindow();
const allmenu = ref([]);
const edit_type = ref(""); // 初始化表单数据
function closeWindow() {
    currentWindow.close();
}
const edit_obj = ref({
    id:-9999,
    name: '',
    sort: -9999,
    menu_id: '',
}); // 初始化表单数据
async function resizeWindowToContent() {
  await nextTick();
  requestAnimationFrame(async () => {
    const root = document.querySelector('.global-edit');
    const form = document.querySelector('.global-edit-form');
    const contentHeight = Math.ceil(
      root?.getBoundingClientRect().height ||
      root?.scrollHeight ||
      form?.getBoundingClientRect().height ||
      form?.scrollHeight ||
      0
    );
    const minHeight = edit_type.value === 'tag' ? 190 : 150;
    const height = Math.max(minHeight, contentHeight + 2);
    await currentWindow.setSize(new LogicalSize(420, height));
    await currentWindow.center();
  });
}
async function submitForm() {
    if(edit_type.value =='tag'){
        if(edit_obj.value.menu_id==null || edit_obj.value.menu_id<=0){
            message('请选择正确的分类!', { title: 'Error', type: 'error' });
            return;
        }
    }
    if(edit_obj.value.name==null || edit_obj.value.name==""){
        await message('请输入名称!', { title: 'Error', type: 'error' });
        return;
    }
    let data = await invoke("update_"+edit_type.value, {obj: edit_obj.value});
    console.log("update_"+edit_type.value+":",edit_type);
    if(data=="ok" ){
        currentWindow.close();
        emit(edit_type.value+'Updated', {});
    }else{
        await message('保存失败!\n'+data, { title: 'Error', type: 'error' });
    }
}

onMounted(async () => {
  try {
    // 监听主窗口发送的事件
    unlisten = await listen('receiveEditData', (event) => {
      console.log('Received:', JSON.stringify(event.payload?.edit_obj)); // 输出接收到的数据
      console.log('Received:', JSON.stringify(event.payload?.type)); // 输出接收到的数据
      console.log('Received:', JSON.stringify(event.payload?.allmenu)); // 输出接收到的数据
      if(!event.payload?.edit_obj){
        return; 
      }
      edit_obj.value = event.payload?.edit_obj; // 更新表单数据
      edit_type.value = event.payload?.type; // 更新表单数据
      allmenu.value = event.payload?.allmenu; // 更新表单数据
      resizeWindowToContent();
    });
    await emit('editReady', { label: currentWindow.label });
  } catch (error) {
    console.error('监听事件失败:', error);
  }
});

// 清理监听器（窗口关闭时）
onUnmounted(() => {
  if (unlisten) {
    unlisten(); // 取消监听
  }
});
</script>
  
<style scoped>
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
</style>
  
