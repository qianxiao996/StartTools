<template>
  <div class="tools-form">
    <el-form :model="tool" label-width="auto">
      <el-form-item label="图标">
        <div class = "tools-icon">
          <!-- <el-upload
            :show-file-list="false"
            :before-upload="beforeAvatarUpload"
          > -->
            <img v-if="tool.icon" :src="'data:image/png;base64,' + tool.icon"  @click="beforeAvatarUpload"/>
            <!-- <el-icon v-else ><Plus /></el-icon> -->
          <!-- </el-upload> -->
          <el-button-group class="ml-4">
            <el-button :icon="RefreshLeft"  @click="RefreshIcon"/>
          </el-button-group>
        </div>
      </el-form-item>
      <el-form-item label="名称">
        <el-input
          v-model="tool.name"
        >
          <template #append>
            <el-button :icon="Close"  @click = "clearName"/>
          </template>
        </el-input>
      </el-form-item>
      <el-form-item label="目标">
        <el-input
          v-model="tool.target"
        >
          <template #append>
            <el-button :icon="Plus" @click="SelectFile" />
          </template>
        </el-input>
      </el-form-item>
      <el-form-item label="参数">
        <el-input v-model="tool.parameters" type="textarea" />
      </el-form-item>
      <el-form-item label="管理员">
        <el-switch v-model="tool.isadmin" />
      </el-form-item>
      <el-form-item>
        <div class="button_container">
          <el-button type="primary" @click="submitForm()">
            保存
          </el-button>
          <el-button @click="closeWindow()">取消</el-button>
        </div>
      </el-form-item>
    </el-form>
  </div>
</template>
<script setup>
import { open } from '@tauri-apps/plugin-dialog';
import { onMounted, onUnmounted,ref  } from 'vue';
import { listen } from '@tauri-apps/api/event';
import '../css/white.css';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { Plus,Close,RefreshLeft } from '@element-plus/icons-vue'
import { invoke } from "@tauri-apps/api/core";
import { message } from '@tauri-apps/plugin-dialog';
import { emit } from '@tauri-apps/api/event';
import { basename } from '@tauri-apps/api/path';
let unlisten;
const tool = ref({ 
  name: '',
  icon: '',
  target: '',
  parameters: '',
  isadmin: false,
  number:0,
  menu_id: '',
  tags_id: '',
}); // 初始化表单数据
onMounted(async () => {
  try {
    // 监听主窗口发送的事件
    unlisten = await listen('receiveToolData', (event) => {
      console.log('Received:', JSON.stringify(event.payload?.tool)); // 输出接收到的数据
      if(!event.payload?.tool){
        return; 
      }
      tool.value = event.payload?.tool; // 更新表单数据
    });
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
function clearName(){
  tool.value.name = '';
}
async function submitForm() {
  if(tool.value.target==null || tool.value.target==""){
    await message('请输入目标路径!', { title: 'Error', type: 'error' });
    return;
  }
  if(tool.value.name==null || tool.value.name==""){
    await message('请输入目标名称!', { title: 'Error', type: 'error' });
    return;
  }
  let data = await invoke("update_tool", {tool: tool.value});
  console.log(data);
  if(data=="ok"){
    emit('toolUpdated', { tool: tool.value });
    getCurrentWebviewWindow().close();
    // tool.value.icon = data; 
    // return true;
  }else{
    await message('保存失败!\n'+data, { title: 'Error', type: 'error' });
  }
}

function closeWindow() {
  getCurrentWebviewWindow().close();
}
const beforeAvatarUpload = async() => {
  const file = await open({
    multiple: false,
    directory: false,
  });
  if(file==null){return;}
  let data = await invoke("get_exe_icon", {filename: file});
  if(data && !data.startsWith('Error')){
    // console.log(data);
    tool.value.icon = data; 
  }else{
    await message('图标获取失败!\n'+data, { title: 'Error', type: 'error' });
  }
}
async function SelectFile(){
  const file = await open({
    multiple: false,
    directory: false,
  });
  if(file==null){return;}
  // 从路径中提取文件名
  const fileName = await basename(file);
  console.log('Selected file name:', fileName);
  let data = await invoke("get_exe_icon", {filename: file});
  if(data && !data.startsWith('Error')){
    // console.log(data);
    tool.value.icon = data; 
    tool.value.target = file;
    tool.value.name = fileName;
  }else{
    await message('图标获取失败!\n'+data, { title: 'Error', type: 'error' });
  }
}
async function RefreshIcon(){
  var tool_path = tool.value.target;
  if(tool_path==null || tool_path==""){return;}
  let data = await invoke("get_exe_icon", {filename: tool_path});
  if(data && !data.startsWith('Error')){
    // console.log(data);
    tool.value.icon = data; 
    return true;
  }else{
    await message('图标获取失败!\n'+data, { title: 'Error', type: 'error' });
  }
}
</script>

<style scoped>

</style>
