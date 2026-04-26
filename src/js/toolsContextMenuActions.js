import { invoke } from "@tauri-apps/api/core";
import { message, ask } from '@tauri-apps/plugin-dialog';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { WebviewWindow, getAllWebviewWindows } from '@tauri-apps/api/webviewWindow';
import { emit, listen } from '@tauri-apps/api/event';
import appIconSrc from './appIcon.js';
import { loadTools } from '../js/data.js';
import { markToolUsed } from './toolUsage.js';

export async function toolsHandleContextMenuAction(store, action, tool, selectedMenuId, selectedTags) {
  console.log(`${action}: ${JSON.stringify(tool)}`);
  const currentMenuId = selectedMenuId?.value ?? store?.state?.Config?.Current_Menu_Id ?? -1;
  const currentTagsId = selectedTags?.value ?? store?.state?.Config?.Current_Tags_Id ?? -1;

  switch (action) {
    case "run":
      await runTool(store, tool, false);
      break;
    case "runAsAdmin": {
      const runTool = JSON.parse(JSON.stringify(tool));
      runTool.isadmin = 1;
      await runTool(store, runTool, false);
      break;
    }
    case "copyPath": {
      const result = await invoke("copy_path", { tools: tool });
      await writeText(result);
      break;
    }
    case "openDir":
      await Exec_Invoke("open_path", tool, false);
      break;
    case "openCmd":
      await Open_Cmd({
        isAdmin: 0,
        Path: tool.target,
        Terminal: store.state.Config.Terminal,
        Terminal_Runas_Arguments: store.state.Config.Terminal_Runas_Arguments,
      });
      break;
    case "openCmdAsAdmin":
      await Open_Cmd({
        isAdmin: 1,
        Path: tool.target,
        Terminal: store.state.Config.Terminal,
        Terminal_Runas_Arguments: store.state.Config.Terminal_Runas_Arguments,
      });
      break;
    case "add":
      openToolsDialog({
        id: -9999,
        name: '',
        icon: appIconSrc,
        target: '',
        parameters: '',
        isadmin: false,
        number: 0,
        menu_id: parseInt(currentMenuId),
        tags_id: parseInt(currentTagsId),
      }, action);
      break;
    case "edit": {
      const editTool = JSON.parse(JSON.stringify(tool));
      if (parseInt(editTool.isadmin) > 0) {
        editTool.isadmin = true;
      }
      openToolsDialog(editTool, action);
      break;
    }
    case "delete": {
      const answer = await ask('确认删除《' + tool.name + '》吗?', {
        title: '工具删除',
        kind: 'warning',
      });
      if (answer) {
        console.log(`删除工具: ${tool.name}`);
        await DeleteTools(tool, store);
      }
      break;
    }
    case "deleteNo": {
      const deleteNoanswer = await ask('确认删除《' + tool.name + '》分类吗?', {
        title: '分类删除',
        kind: 'warning',
      });
      if (deleteNoanswer) {
        console.log(`删除分类: ${tool.name}`);
      }
      break;
    }
    case "clear": {
      const clearanswer = await ask('确认清空吗?', {
        title: '清空',
        kind: 'warning',
      });
      if (clearanswer) {
        console.log(`清空分类: ${tool.name} 标签: ${tool.name}`);
      }
      break;
    }
    default:
      break;
  }
}

async function runTool(store, tool, isreturn) {
  const result = await Exec_Invoke("open_tools", tool, isreturn);
  if (result === 'ok') {
    markToolUsed(tool);
    await loadTools(store);
  }
  return result;
}

async function DeleteTools(tool, store) {
  if (tool.id < 0) {
    await message('删除失败, 工具ID不正确!', { title: '工具删除', type: 'error' });
    return;
  }

  const result = await invoke('delete_tool', { toolId: parseInt(tool.id) });
  console.log(result);
  if (result === 'ok') {
    await loadTools(store);
    await emit('toolUpdated', { action: 'delete', toolId: parseInt(tool.id) });
  } else {
    await message(result, { title: '失败!', type: 'error' });
  }
  return result;
}

async function openToolsDialog(tool, type) {
  const windowsLabel = !tool || tool.id === undefined || tool.id < 0
    ? 'tools_' + Math.floor(100000 + Math.random() * 900000)
    : 'tools_' + tool.id;
  const title = type === "edit" && tool?.name ? "编辑工具：" + tool.name : "新增工具";

  const sendToolData = async () => {
    try {
      await emit('receiveToolData', { tool });
    } catch (error) {
      console.error('发送工具数据失败:', error);
    }
  };

  const windows = await getAllWebviewWindows();
  for (const window of windows) {
    if (windowsLabel === window.label) {
      await window.show();
      await window.center();
      try {
        window.setFocus();
        window.setTitle(title);
        await sendToolData();
        setTimeout(sendToolData, 120);
        setTimeout(sendToolData, 360);
      } catch (error) {
        console.error('发送数据到子窗口时发生错误:', error);
      }
      return;
    }
  }

  let ready = false;
  let unlistenReady = () => {};
  unlistenReady = await listen('toolsEditReady', async (event) => {
    if (event.payload?.label !== windowsLabel) {
      return;
    }
    ready = true;
    await sendToolData();
    unlistenReady();
  });

  const toolsWebview = new WebviewWindow(windowsLabel, {
    label: "工具编辑",
    center: true,
    url: "/toolsedit",
    title,
    width: 600,
    height: 320,
    minHeight: 240,
    visible: false,
    alwaysOnTop: true,
    focus: true,
  });

  toolsWebview.once('tauri://created', async function () {
    toolsWebview.show();
    toolsWebview.setFocus();
    toolsWebview.center();
    [500, 1000, 1800].forEach((delay) => {
      setTimeout(async () => {
        if (!ready) {
          await sendToolData();
        }
      }, delay);
    });
    setTimeout(() => {
      if (!ready) {
        unlistenReady();
      }
    }, 3000);
  });

  toolsWebview.once('tauri://error', async (error) => {
    unlistenReady();
    await message('创建 Webview 窗口时发生错误: ' + error, { title: 'Error', type: 'error' });
    console.error('创建 Webview 窗口时发生错误:', error);
  });

  toolsWebview.once('tauri://close-requested', () => {
    unlistenReady();
    toolsWebview.close();
  });
}

async function Exec_Invoke(name, tools, isreturn) {
  if (!tools) {
    return;
  }
  const result = await invoke(name, { tools });
  if (isreturn) {
    return result.toString();
  }
  if (result === 'ok') {
    console.log(result);
  } else {
    await message(result, { title: '失败!', type: 'error' });
  }
  return result;
}

async function Open_Cmd(data) {
  const result = await invoke('open_cmd', { data });
  if (result === 'ok') {
    console.log(result);
  } else {
    await message(result, { title: '失败!', type: 'error' });
  }
  return result;
}
