<script setup>
import '../css/white.css';
import { ref, watch, computed, nextTick, onMounted, onUnmounted, useTemplateRef } from "vue";
import 'element-plus/dist/index.css';
import { useStore } from 'vuex';
import { loadMenus,loadTags,loadTools } from '../js/data.js';
import { markToolUsed, sortToolsForView } from '../js/toolUsage.js';
import { invoke } from "@tauri-apps/api/core";
import { message } from '@tauri-apps/plugin-dialog';
import { WebviewWindow,getAllWebviewWindows } from '@tauri-apps/api/webviewWindow'
import { emit } from '@tauri-apps/api/event';
import { listen } from '@tauri-apps/api/event';
import { ask } from '@tauri-apps/plugin-dialog';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { toolsHandleContextMenuAction } from '../js/toolsContextMenuActions.js';
import appIconSrc from '../js/appIcon.js';
import {
  Sort,Refresh,Remove,
  ArrowRight,ArrowRightBold,Link,DocumentAdd,Monitor,Platform,Edit,Delete,Folder,Clock
} from '@element-plus/icons-vue'
//tools鍙抽敭鑿滃崟
// let contextToolMenu= reactive({
//   visible: false, // 鏄惁鏄剧ず鍙抽敭鑿滃崟
//   x: 0,           // 鑿滃崟鐨?X 鍧愭爣
//   y: 0,           // 鑿滃崟鐨?Y 鍧愭爣
//   selectedTool: null, // 褰撳墠閫変腑鐨勫伐鍏?
// })
// let contextToolsMenu= reactive({
//   visible: false, // 鏄惁鏄剧ず鍙抽敭鑿滃崟
//   x: 0,           // 鑿滃崟鐨?X 鍧愭爣
//   y: 0,           // 鑿滃崟鐨?Y 鍧愭爣
//   selectedTool: null, // 褰撳墠閫変腑鐨勫伐鍏?
// })
// let trayIconInstance = null;
//鍔犺浇鎵樼洏鍥炬爣
// async function load_tray() {
//   const menu = await Menu.new({
//     items: [
//       {
//         id: 'quit',
//         text: '閫€鍑?,
//         action: () => {
//           appWindow.close();
//         },
//       },
//     ],
//   });
//   const options = {
//    	// icon 鐨勭浉瀵硅矾寰勫熀浜庯細椤圭洰鏍圭洰褰?src-tauri/
//      icon: await defaultWindowIcon(),
//      // 鎵樼洏鎻愮ず锛屾偓娴湪鎵樼洏鍥炬爣涓婂彲浠ユ樉绀?tauri-app
//      tooltip: 'StartTools',
//      // 鏄惁鍦ㄥ乏閿偣鍑绘椂鏄剧ず鎵樼洏鑿滃崟锛岄粯璁や负 true銆傚綋鐒朵笉鑳戒负 true 鍟︼紝绋嬪簭杩涘叆鍚庡彴涓嶅緱宸﹂敭鐐瑰嚮鍥炬爣鏄剧ず绐楀彛鍟娿€?
//      menuOnLeftClick: false,
//      // 鎵樼洏鑿滃崟锛屽悗缁垱寤?....
//      menu: menu,
//      // 鎵樼洏鍥炬爣涓婁簨浠剁殑澶勭悊绋嬪簭銆傝繖涓笅闈㈤渶瑕佽缁嗚涓€涓?
//      action: async(event) => {
//       if(event.button!=='Left')return; // 鍙鐞嗗乏閿偣鍑讳簨浠?
//       switch (event.type) {
//         case 'DoubleClick':
//         case 'Click':
//           // console.log(
//           //   `mouse ${event.button} button pressed, state: ${event.buttonState}`
//           // );
//           if(event.buttonState==='Up'){
//             if (await appWindow.isMinimized()) {
//               await appWindow.unminimize(); // 杩樺師鏈€灏忓寲鐨勭獥鍙?
//             }
//             await appWindow.show(); // 鏄剧ず绐楀彛
//             await appWindow.setFocus(); // 鑱氱劍绐楀彛
//           }
//           break;
//       }
//     },
//   };
//   trayIconInstance = await TrayIcon.new(options);
// }
// load_tray();
const store = useStore();
loadMenus(store);
loadTags(store);
loadTools(store);
const MIN_MENU_WIDTH = 56;
const MAX_MENU_WIDTH = 320;
const eventUnlisteners = [];
// const menuWidth = ref(100); // 鍒濆瀹藉害
let startX = 0; // 榧犳爣璧峰浣嶇疆
let startWidth = 0; // 鍒濆瀹藉害鍊?
let isResizing = false; // 鏄惁姝ｅ湪璋冩暣澶у皬鐨勭姸鎬佹爣蹇?
const allmenu = computed(() => store.state.Menus);
const alltags = computed(() => store.state.Tags);
const alltools = computed(() => store.state.Tools);
const sortField = computed(() => store.state.Config.Tools_Order_Field);
const sortType = computed(() => store.state.Config.Tools_Order_Type);
const menuWidth =computed(() => store.state.Config.LeftWidth); 
const selectedMenuId = computed(() => store.state.Config.Current_Menu_Id.toString());
const selectedTags = computed(() => store.state.Config.Current_Tags_Id.toString());
// const menuWidth = ref(store.state.Config.LeftWidth);
// 褰撳墠閫変腑鐨勮彍鍗旾D
// const selectedMenuId = ref(store.state.Config.Current_Menu_Id);
// const selectedTags = ref(store.state.Config.Current_Tags_Id);
const filteredTags =ref([]);
const filteredTools =ref([]);
const draggingTool = ref(null);
const dragOverMenuId = ref(null);
const dragOverTagId = ref(null);
const dragTargetMenuId = ref(null);
const dragTargetTagId = ref(null);
const isToolDropActive = ref(false);
const isToolMoveActive = ref(false);
const toolDragPoint = ref({ x: 0, y: 0 });
const toolMoveTargetText = computed(() => {
  const menu = allmenu.value.find(item => Number(item.id) === Number(dragTargetMenuId.value));
  const tag = alltags.value.find(item => Number(item.id) === Number(dragTargetTagId.value));
  const menuName = menu?.name || '当前分类';
  const tagName = tag?.name || '默认标签';
  return `${menuName} / ${tagName}`;
});
let lastToolDragSwitchKey = '';
let lastToolDragSwitchAt = 0;
let clearToolDragTimer = null;
let pointerDragTool = null;
let pointerDragStart = null;
let pointerDragging = false;
let suppressNextToolClick = false;
const clamp = (value, min, max) => Math.min(Math.max(value, min), max);
function startResize(event) {
  startX = event.clientX; // 鑾峰彇榧犳爣璧峰浣嶇疆
  startWidth = menuWidth.value; // 鑾峰彇褰撳墠瀹藉害鍊?
  isResizing = true; // 璁剧疆姝ｅ湪璋冩暣澶у皬鐨勭姸鎬佷负 true 
  document.addEventListener('mousemove', doResize); // 娣诲姞榧犳爣绉诲姩浜嬩欢鐩戝惉鍣?
  document.addEventListener('mouseup', stopResize); // 娣诲姞榧犳爣鏉惧紑浜嬩欢鐩戝惉鍣?
}
// watch(() => store.state.Config.LeftWidth, (newWidth) => {
//   if(newWidth>0){
//     // console.log('menuWidth 鍙戠敓鍙樺寲:', newWidth);
//     menuWidth.value = newWidth; // 鏇存柊 menuWidth 鐨勫€?
//   }
// });
function doResize(event) { 
  if (isResizing) { // 濡傛灉姝ｅ湪璋冩暣澶у皬锛屽垯鏇存柊瀹藉害鍊?
    const diffX = event.clientX - startX; // 璁＄畻榧犳爣绉诲姩鐨勮窛绂诲樊鍊?
    // 纭繚 startWidth 鏄暟瀛楃被鍨?
    const newWidth = clamp(parseFloat(startWidth) + diffX, MIN_MENU_WIDTH, MAX_MENU_WIDTH); 
    store.commit('updateConfigLeftWidth',  newWidth); 
    console.log('menuWidth 鍙戠敓鍙樺寲:', newWidth); 
  } 
}

function toolsHandle(action, tool) {
  toolsHandleContextMenuAction(store,action,tool,selectedMenuId,selectedTags)
}

function startToolDrag(event, tool) {
  if (clearToolDragTimer) {
    clearTimeout(clearToolDragTimer);
    clearToolDragTimer = null;
  }
  draggingTool.value = tool;
  dragTargetMenuId.value = Number(tool.menu_id);
  dragTargetTagId.value = Number(tool.tags_id);
  lastToolDragSwitchKey = '';
  lastToolDragSwitchAt = 0;
  event.dataTransfer.effectAllowed = 'move';
  event.dataTransfer.setData('text/plain', String(tool.id));
}

function startToolPointerDrag(event, tool) {
  if (isToolMoveActive.value && event.button === 2) {
    event.preventDefault();
    cancelToolMove();
    return;
  }
  if (event.button !== 0) {
    return;
  }

  event.preventDefault();
  pointerDragTool = tool;
  pointerDragStart = { x: event.clientX, y: event.clientY };
  pointerDragging = false;
  try {
    event.currentTarget?.setPointerCapture?.(event.pointerId);
  } catch (error) {
    console.warn('setPointerCapture failed:', error);
  }
  window.addEventListener('pointermove', handleToolPointerMove, { capture: true });
  window.addEventListener('pointerdown', handleToolPointerCancelButton, { capture: true });
  window.addEventListener('pointerup', handleToolPointerUp, { capture: true, once: true });
  window.addEventListener('pointercancel', handleToolPointerCancel, { capture: true, once: true });
  window.addEventListener('mousedown', handleToolMouseCancelButton, { capture: true });
  window.addEventListener('mouseup', handleToolMouseUpFallback, { capture: true, once: true });
  window.addEventListener('blur', handleToolPointerCancel, { once: true });
  document.addEventListener('dragstart', preventNativeToolDrag, { capture: true });
  document.addEventListener('contextmenu', cancelToolMoveByContextMenu, { capture: true });
}

function beginPointerToolDrag() {
  if (!pointerDragTool || draggingTool.value) {
    return;
  }

  draggingTool.value = pointerDragTool;
  isToolMoveActive.value = true;
  dragTargetMenuId.value = Number(pointerDragTool.menu_id);
  dragTargetTagId.value = Number(pointerDragTool.tags_id);
  lastToolDragSwitchKey = '';
  lastToolDragSwitchAt = 0;
}

function handlePointerHoverSwitch(x, y) {
  const target = document.elementFromPoint(x, y);
  const menuItem = target?.closest?.('[data-menu-id]');
  const tagItem = target?.closest?.('[data-tag-id]');

  if (menuItem) {
    setDragOverMenu(menuItem.dataset.menuId);
    return;
  }

  if (tagItem) {
    setDragOverTag(tagItem.dataset.tagId);
    return;
  }

  if (target?.closest?.('.tools-content')) {
    dragOverMenuId.value = null;
    dragOverTagId.value = null;
    isToolDropActive.value = true;
  } else {
    isToolDropActive.value = false;
  }
}

function handleToolPointerMove(event) {
  if (!pointerDragTool || !pointerDragStart) {
    return;
  }

  if (pointerDragging && (event.buttons & 2) === 2) {
    cancelToolMoveByPointerEvent(event);
    return;
  }

  if (pointerDragging && event.buttons === 0) {
    handleToolPointerUp(event);
    return;
  }

  const moved = Math.abs(event.clientX - pointerDragStart.x) + Math.abs(event.clientY - pointerDragStart.y);
  if (!pointerDragging && moved < 6) {
    return;
  }

  pointerDragging = true;
  suppressNextToolClick = true;
  toolDragPoint.value = { x: event.clientX, y: event.clientY };
  beginPointerToolDrag();
  handlePointerHoverSwitch(event.clientX, event.clientY);
}

async function handleToolPointerUp(event) {
  removeToolPointerListeners();

  if (pointerDragging && draggingTool.value) {
    const target = document.elementFromPoint(event.clientX, event.clientY);
    if (isToolDropActive.value && target?.closest?.('.tools-content')) {
      const menuId = dragTargetMenuId.value ?? Number(selectedMenuId.value);
      const tagId = dragTargetTagId.value ?? Number(selectedTags.value);
      await moveToolTo(menuId, tagId);
    } else {
      clearToolDragState();
    }
  }

  pointerDragTool = null;
  pointerDragStart = null;
  pointerDragging = false;
}

function handleToolPointerCancelButton(event) {
  if (event.button === 2) {
    cancelToolMoveByPointerEvent(event);
  }
}

function handleToolMouseCancelButton(event) {
  if (event.button === 2) {
    cancelToolMoveByPointerEvent(event);
  }
}

function cancelToolMoveByPointerEvent(event) {
  if (!isToolMoveActive.value && !pointerDragTool && !draggingTool.value) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  cancelToolMove();
}

function handleToolMouseUpFallback(event) {
  if (!pointerDragTool && !draggingTool.value) {
    removeToolPointerListeners();
    return;
  }
  handleToolPointerUp(event);
}

function handleToolPointerCancel() {
  removeToolPointerListeners();
  pointerDragTool = null;
  pointerDragStart = null;
  pointerDragging = false;
  clearToolDragState();
}

function removeToolPointerListeners() {
  window.removeEventListener('pointermove', handleToolPointerMove, { capture: true });
  window.removeEventListener('pointerdown', handleToolPointerCancelButton, { capture: true });
  window.removeEventListener('pointerup', handleToolPointerUp, { capture: true });
  window.removeEventListener('pointercancel', handleToolPointerCancel, { capture: true });
  window.removeEventListener('mousedown', handleToolMouseCancelButton, { capture: true });
  window.removeEventListener('mouseup', handleToolMouseUpFallback, { capture: true });
  window.removeEventListener('blur', handleToolPointerCancel);
  document.removeEventListener('dragstart', preventNativeToolDrag, { capture: true });
}

function preventNativeToolDrag(event) {
  if (!pointerDragTool && !draggingTool.value) {
    return;
  }
  event.preventDefault();
}

function handleToolClick(tool) {
  if (suppressNextToolClick) {
    suppressNextToolClick = false;
    return;
  }
  Exec_Invoke('open_tools', tool, false);
}

function cancelToolMove() {
  removeToolPointerListeners();
  document.removeEventListener('contextmenu', cancelToolMoveByContextMenu, { capture: true });
  pointerDragTool = null;
  pointerDragStart = null;
  pointerDragging = false;
  suppressNextToolClick = false;
  clearToolDragState();
}

function cancelToolMoveByContextMenu(event) {
  if (!isToolMoveActive.value && !pointerDragTool) {
    return;
  }
  cancelToolMoveByPointerEvent(event);
}

function endToolDrag() {
  if (clearToolDragTimer) {
    clearTimeout(clearToolDragTimer);
  }
  clearToolDragTimer = setTimeout(() => {
    clearToolDragState();
  }, 30000);
}

function clearToolDragState() {
  document.removeEventListener('contextmenu', cancelToolMoveByContextMenu, { capture: true });
  if (clearToolDragTimer) {
    clearTimeout(clearToolDragTimer);
    clearToolDragTimer = null;
  }
  const needsToolListRefresh = Boolean(draggingTool.value);
  draggingTool.value = null;
  dragOverMenuId.value = null;
  dragOverTagId.value = null;
  dragTargetMenuId.value = null;
  dragTargetTagId.value = null;
  isToolDropActive.value = false;
  isToolMoveActive.value = false;
  lastToolDragSwitchKey = '';
  if (needsToolListRefresh) {
    Change_Tag(parseInt(selectedMenuId.value, 10), parseInt(selectedTags.value, 10));
  }
}

function getFirstTagIdByMenu(menuId) {
  const targetTag = alltags.value
    .filter(tag => Number(tag.menu_id) === Number(menuId))
    .sort((a, b) => Number(a.sort || 0) - Number(b.sort || 0))[0];
  return targetTag ? Number(targetTag.id) : -1;
}

function setDragOverMenu(menuId) {
  if (draggingTool.value) {
    const nextMenuId = Number(menuId);
    const key = 'menu:' + nextMenuId;
    const now = Date.now();
    dragOverMenuId.value = nextMenuId;
    dragOverTagId.value = null;
    dragTargetMenuId.value = nextMenuId;
    dragTargetTagId.value = getFirstTagIdByMenu(nextMenuId);
    if (key !== lastToolDragSwitchKey && now - lastToolDragSwitchAt > 180) {
      lastToolDragSwitchKey = key;
      lastToolDragSwitchAt = now;
      selectMenu(nextMenuId);
    }
  }
}

function setDragOverTag(tagId) {
  if (draggingTool.value) {
    const nextTagId = Number(tagId);
    const key = 'tag:' + nextTagId;
    const now = Date.now();
    dragOverTagId.value = nextTagId;
    dragOverMenuId.value = null;
    const tag = alltags.value.find(item => Number(item.id) === nextTagId);
    dragTargetMenuId.value = tag ? Number(tag.menu_id) : Number(selectedMenuId.value);
    dragTargetTagId.value = nextTagId;
    if (key !== lastToolDragSwitchKey && now - lastToolDragSwitchAt > 180) {
      lastToolDragSwitchKey = key;
      lastToolDragSwitchAt = now;
      selectTags(nextTagId);
    }
  }
}

async function moveToolTo(menuId, tagId) {
  const tool = draggingTool.value;
  if (!tool) {
    return;
  }

  const nextTool = {
    ...tool,
    menu_id: Number(menuId),
    tags_id: Number(tagId),
  };
  const result = await invoke("update_tool", { tool: nextTool });
  if (result === "ok") {
    await loadTools(store);
    await emit('toolUpdated', { action: 'move', toolId: Number(tool.id) });
    selectMenu(Number(menuId));
    selectTags(Number(tagId));
  } else {
    await message('移动失败!\n' + result, { title: 'Error', type: 'error' });
  }
  clearToolDragState();
}

function handleToolsDragOver(event) {
  if (!draggingTool.value) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  event.dataTransfer.dropEffect = 'move';
}

async function dropToolToCurrentSelection(event) {
  if (!draggingTool.value) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  const menuId = dragTargetMenuId.value ?? Number(selectedMenuId.value);
  const tagId = dragTargetTagId.value ?? Number(selectedTags.value);
  await moveToolTo(menuId, tagId);
}

function stopResize() {
  isResizing = false; // 璁剧疆姝ｅ湪璋冩暣澶у皬鐨勭姸鎬佷负 false锛屽苟绉婚櫎浜嬩欢鐩戝惉鍣?
  document.removeEventListener('mousemove', doResize);
  document.removeEventListener('mouseup', stopResize);
}

// // 鐩戝惉 selectedMenuId 鍙樺寲
// Change_Menu(selectedMenuId.value);
// 鐩戝惉 alltags 鍙樺寲
watch(alltags, (newAllTags) => {
  if (newAllTags.length > 0 && selectedMenuId.value) {
    Change_Menu(parseInt(selectedMenuId.value, 10));
  }
}, { immediate: true });

watch(alltools, () => {
  if (selectedTags.value) {
    Change_Tag(parseInt(selectedMenuId.value, 10), parseInt(selectedTags.value, 10));
  }
}, { immediate: true });

watch(selectedMenuId, (menuId) => {
  if (allmenu.value.length > 0) {
    Change_Menu(parseInt(menuId, 10));
  }
});

watch([selectedTags, sortField, sortType], () => {
  Change_Tag(parseInt(selectedMenuId.value, 10), parseInt(selectedTags.value, 10));
});

function Change_Menu(menu_id) {
  const foundTag = allmenu.value.find(menu => menu.id.toString() == menu_id);
  if(!foundTag){
    if(allmenu.value.length>0){
      menu_id = allmenu.value[0].id.toString();
      store.commit('updateCurrent_Menu_Id',menu_id);
    }else{
      return;
    }

  }
  // console.log("Change_Menu:"+menu_id);
  filteredTags.value = alltags.value.filter(tag => tag.menu_id == menu_id);
  filteredTags.value.sort((a, b) => a.sort - b.sort); // 鍗囧簭鎺掑簭
  if (filteredTags.value.length > 0) {
    // 妫€鏌?selectedTags 鏄惁鍦?filteredTags 涓?
    const foundTag = filteredTags.value.find(tag => tag.id.toString() == selectedTags.value.toString());
    if (!foundTag) {
      // 濡傛灉 selectedTags 涓嶅湪 filteredTags 涓紝鍒欒缃负 filteredTags 鐨勭涓€涓厓绱?
      // selectedTags.value = filteredTags.value[0].id.toString();
      store.commit('updateCurrent_Tags_Id',filteredTags.value[0].id.toString());
    }
    if( selectedTags.value<0){
      store.commit('updateCurrent_Tags_Id',filteredTags.value[0].id.toString());
      // selectedTags.value = filteredTags.value[0].id.toString();
    }
  }else{
    filteredTags.value.push({ id: -1, name: "默认标签", menu_id: menu_id });
    store.commit('updateCurrent_Tags_Id', -1);
    // selectedTags.value = "-1"; // 榛樿鍊硷紝闃叉filteredTags涓虹┖鐨勬儏鍐?
  }
  Change_Tag(parseInt(menu_id, 10), parseInt(selectedTags.value, 10));
}

function selectMenu(id) {
  Change_Menu(id);
}

function selectTags(id) {
  Change_Tag(parseInt(selectedMenuId.value, 10), parseInt(id, 10));
}

function Change_Tag(menu_id, tag_id) {
  // selectedMenuId.value = menu_id.toString();
  store.commit('updateCurrent_Menu_Id',menu_id.toString());
  store.commit('updateCurrent_Tags_Id',tag_id.toString());
  // selectedTags.value = tag_id.toString();
  if (tag_id == -1 || tag_id == "-1") {
    // store.commit('updateCurrent_Tags_Id',-1);
    filteredTools.value = alltools.value.filter(tools => tools.menu_id === menu_id);
  } else {
    filteredTools.value = alltools.value.filter(tools => tools.tags_id === tag_id && tools.menu_id === menu_id);
  }
  if (sortField.value === "number") {
    filteredTools.value = sortToolsForView(filteredTools.value, "number", sortType.value);
  } else if (sortField.value === "recent") {
    filteredTools.value = sortToolsForView(filteredTools.value, "recent", sortType.value);
  } else if (sortField.value === "name") {
    filteredTools.value = sortToolsForView(filteredTools.value, "name", sortType.value);
  }
  if (draggingTool.value && !filteredTools.value.some(tool => Number(tool.id) === Number(draggingTool.value.id))) {
    filteredTools.value = [draggingTool.value, ...filteredTools.value];
  }
}


async function Exec_Invoke(name,tools,isreturn) {
  let result = await invoke(name,  { tools });
  if(isreturn){
    return result.toString();
  }
  if (result== 'ok') {
    console.log(result);
    if (name === 'open_tools') {
      markToolUsed(tools);
      await loadTools(store);
    }
  }else{
    await message(result, { title: '失败!', type: 'error' });
  }
  return result;
}

// 鍒濆鍖栬彍鍗曢€夋嫨
selectMenu(parseInt(selectedMenuId.value, 10));
// 瀹氫箟涓€涓?ref 鏉ュ紩鐢ㄨ彍鍗曞厓绱?
// const contextToolMenuRef = ref(null);
// const contextToolsMenuRef = ref(null);
// 鏄剧ず鍙抽敭鑿滃崟
// function showToolContextMenu(event, tool,menu_type) {
//   event.stopPropagation();
//   event.preventDefault();
//   let contextMenu_obj = contextToolMenu;
//   let contextMenuRef_obj  =contextToolMenuRef;
//   if(menu_type=="tool"){
//     contextToolMenu.visible = true;
//     contextToolsMenu.visible = false;
//     contextMenu_obj = contextToolMenu
//     contextMenuRef_obj = contextToolMenuRef
//   }else{
//     contextToolMenu.visible = false;
//     contextToolsMenu.visible = true;
//     contextMenu_obj = contextToolsMenu;
//     contextMenuRef_obj = contextToolsMenuRef

//   }
//   contextMenu_obj.x = event.clientX;
//   contextMenu_obj.y = event.clientY;
//   contextMenu_obj.selectedTool = tool; // 鍔ㄦ€佷紶閫掑伐鍏峰唴瀹?
//   setTimeout(() => {
//     if (contextMenuRef_obj.value) {
//       // 鑾峰彇绐楀彛灏哄
//       const windowWidth = window.innerWidth;
//       const windowHeight = window.innerHeight-30;
//         // 鑾峰彇鑿滃崟鐨勫搴﹀拰楂樺害
//       const menuWidth = contextMenuRef_obj.value.offsetWidth;
//       const menuHeight = contextMenuRef_obj.value.offsetHeight;
//       // 妫€鏌ヨ彍鍗曟槸鍚︿細瓒呭嚭绐楀彛鍙充晶
//       if (contextMenu_obj.x + menuWidth > windowWidth) {
//         contextMenu_obj.x = windowWidth - menuWidth;
//       }
//       // 妫€鏌ヨ彍鍗曟槸鍚︿細瓒呭嚭绐楀彛搴曢儴
//       if (contextMenu_obj.y + menuHeight > (windowHeight)) {
//         contextMenu_obj.y = windowHeight - menuHeight;
//       }
//     }
//   });

// }
function GetLastSort(arr){
  var sort = -9999;
  arr.forEach(function(item) {
    if (item.sort > sort) {
      sort = item.sort;
    }
  });
  return sort;
}
async function tagsHandleContextMenuAction(action,tag) {
  console.log(`${action}: ${JSON.stringify(tag)}`);
  switch (action) {
    case "tagAdd":
      var lastTagSort = GetLastSort(alltags.value)+1;
      openEditDialog('tag',{id:-9999, name: '', menu_id: parseInt(selectedMenuId.value),sort:lastTagSort});      
      break;
    case "tagEdit":  
      if(tag==null){
        await message('未选中标签!', { title: '失败!', type: 'error' });
        return;
      }   
      openEditDialog('tag',{id:tag.id, name: tag.name, menu_id: parseInt(tag.menu_id) ,sort:tag.sort});      
      break;
    case "tagDelete":
      const tagDeleteanswer = await ask('确认删除《' + tag.name + '》标签吗? 对应的工具将一并删除!', {
        title: '标签删除',
        kind: 'warning',
      });
      if(tagDeleteanswer){
        if(tag.id==-1){
          await message('默认标签不能删除!', { title: '标签删除', type: 'error' });
          return;
        }
        if(tag.id<0){
          await message('删除失败, 标签ID不正确', { title: '标签删除', type: 'error' });
          return;
        }else{
          let result = await invoke('delete_tag',  { menuId:parseInt(selectedMenuId.value),tagsId:parseInt(tag.id) });
          if (result== 'ok') {
            loadTags(store);
          }else{
            await message(result, { title: '失败!', type: 'error' });
          }
          return result;
        }
      }
      break;
    }

}
async function menuHandleContextMenuAction(action,menu) {
  console.log(`${action}: ${JSON.stringify(menu)}`);
  switch (action) {
    case "menuAdd":  
      var lastSort = GetLastSort(allmenu.value)+1;
      openEditDialog('menu',{id:-9999, name: '',sort:lastSort});      
      break;    
    case "menuEdit":      
      if(menu==null){
        await message('未选中分类!', { title: '失败!', type: 'error' });
        return;
      }   
      openEditDialog('menu',{id:menu.id, name: menu.name,sort:menu.sort});      
      break;
    case "menuDelete":
      const tagDeleteanswer = await ask('确认删除《' + menu.name + '》分类吗? 对应的标签及工具将一并删除!', {
        title: '分类删除',
        kind: 'warning',
      });
      if(tagDeleteanswer){
        if(menu.id<0){
          await message('删除失败, 分类ID不正确', { title: '分类删除', type: 'error' });
          return;
        }else{
          var foundTagId=[];
          //寰楀埌鑿滃崟涓嬬殑鎵€鏈夋爣绛?
          for (const tag of alltags.value) {
            if (parseInt(tag.menu_id) == parseInt(menu.id)) {
              foundTagId.push(parseInt(tag.id));
            }
          }
          let result = await invoke('delete_menu',{ menuId:parseInt(menu.id) , tagsIdList:foundTagId});
          if (result== 'ok') {
            loadMenus(store);
            loadTags(store);
          }else{
            await message(result, { title: '失败!', type: 'error' });
          }
          return result;
        }
      }
      break;
    }

}

// 澶勭悊鍙抽敭鑿滃崟椤圭偣鍑讳簨浠?
async function toolsContainerHandleContextMenuAction(action) {
  // const tool = contextToolMenu.selectedTool;
  // if (!tool) return;
  console.log(`${action}`);
  switch (action) {
    case "nameSort":
      if(sortType.value=="desc"){
        store.commit('updateTools_Order', { field:'name', type:'asc' });
      }else{
        store.commit('updateTools_Order', { field:'name', type:'desc' });
      }
      selectTags(parseInt(selectedTags.value, 10));
      break;
    case "numberSort":    
      if(sortType.value=="desc"){
        store.commit('updateTools_Order', { field:'number', type:'asc' });
      }else{
        store.commit('updateTools_Order', { field:'number', type:'desc' });
      }
      selectTags(parseInt(selectedTags.value, 10));
      break;
    case "recentSort":
      if(sortType.value=="desc"){
        store.commit('updateTools_Order', { field:'recent', type:'asc' });
      }else{
        store.commit('updateTools_Order', { field:'recent', type:'desc' });
      }
      selectTags(parseInt(selectedTags.value, 10));
      break;
    // 清空无效项目
    case "clear_no":  
      const clearNoanswer = await ask('确认清除无效项目吗?', {
          title: '清除无效项目',
          kind: 'warning',
      });
      if(clearNoanswer){
        console.log(`清除无效项目: ${selectedMenuId.value} 标签: ${selectedTags.value}`);
        let result = await invoke('clear_tool_no',  { menuId:parseInt(selectedMenuId.value), tagsId:parseInt(selectedTags.value)});
        if (result== 'ok') {
          console.log(result);
          await loadTools(store);
          await emit('toolUpdated', { action: 'clear_no' });
          selectTags(parseInt(selectedTags.value, 10));
        }else{
          await message(result, { title: '失败!', type: 'error' });
        }
      }       
      break
    case "flushed":      
      await loadTools(store);
      selectTags(parseInt(selectedTags.value, 10));
      break;
    case "clear":  
      const clearanswer = await ask('确认清空吗?', {
          title: '清空',
          kind: 'warning',
      });
      if(clearanswer){
        console.log(`清空分类: ${selectedMenuId.value} 标签: ${selectedTags.value}`);
        let result = await invoke('clear_tool',  { menuId:parseInt(selectedMenuId.value), tagsId:parseInt(selectedTags.value)});
        if (result== 'ok') {
          console.log(result);
          await loadTools(store);
          await emit('toolUpdated', { action: 'clear' });
          selectTags(parseInt(selectedTags.value, 10));
        }else{
          await message(result, { title: '失败!', type: 'error' });
        }
      }      
      break;
    default:
      break;
  }
  // 闅愯棌鍙抽敭鑿滃崟
  // contextToolMenu.visible = false;
}


//鑿滃崟缂栬緫 鍜?鏍囩缂栬緫

async function openEditDialog(type,edit_obj) {
  var windows_label;
  if(edit_obj==null || edit_obj.id ==undefined || edit_obj.id<0){
    windows_label ='edit_'+ type+"_"+Math.floor(100000 + Math.random() * 900000);
  }else{
    windows_label ='edit_'+ type+"_"+edit_obj.id;
  }
  // 妫€鏌ユ槸鍚﹀凡缁忓瓨鍦ㄥ搴旂殑 WebviewWindow 瀹炰緥
  const windows = await  getAllWebviewWindows();
  var wintitle ;
  if(type=="tag"){
    wintitle = edit_obj.name ? '编辑标签：' + edit_obj.name : '新增标签';
  }else{
    wintitle = edit_obj.name ? '编辑分类：' + edit_obj.name : '新增分类';
  }
  const sendEditData = async () => {
    try {
      await emit('receiveEditData', { type:type,edit_obj:edit_obj,allmenu:allmenu.value});
    } catch (error) {
      console.error('发送数据失败:', error);
    }
  };
  for (const window of windows) {
    if(windows_label== window.label){
      await window.show();
      await window.center();
      try {
        window.setFocus();
        window.setTitle(wintitle);
        await sendEditData();
        setTimeout(sendEditData, 120);
        setTimeout(sendEditData, 360);
      } catch (error) {
        console.error('发送数据到子窗口时发生错误:', error);
      }
      return; 
    }
  }
  // 姣忔閮藉垱寤烘柊鐨?WebviewWindow 瀹炰緥
  let ready = false;
  let unlistenReady = () => {};
  unlistenReady = await listen('editReady', async (event) => {
    if (event.payload?.label !== windows_label) {
      return;
    }
    ready = true;
    await sendEditData();
    unlistenReady();
  });

  const toolsWebview = new WebviewWindow(windows_label, {
    label: wintitle,
    center: true,
    url: "/edit",
    title: wintitle,
    width: 420,
    height: type === 'tag' ? 210 : 168,
    minHeight: type === 'tag' ? 190 : 148,
    visible: false,
    alwaysOnTop: true,
    focus:true,
  });
  toolsWebview.once('tauri://created',async function () {
    toolsWebview.show();
    toolsWebview.setFocus();
    // toolsWebview.center();
      // 纭繚瀛愮獥鍙ｅ姞杞藉畬鎴愬悗鍐嶅彂閫佷簨浠?
    [500, 1000, 1800].forEach((delay) => {
      setTimeout(async () => {
        if (!ready) {
          await sendEditData();
        }
      }, delay);
    });
    setTimeout(() => {
      if (!ready) {
        unlistenReady();
      }
    }, 3000);
  });
  // 鐩戝惉绐楀彛鍒涘缓閿欒浜嬩欢
  toolsWebview.once('tauri://error',async (error) => {
    unlistenReady();
    await message('创建 Webview 窗口时发生错误: '+error, { title: 'Error', type: 'error' });
    console.error('创建 Webview 窗口时发生错误:', error);
  });
  // 鐩戝惉绐楀彛鍏抽棴浜嬩欢锛屽叧闂獥鍙ｆ椂閿€姣佸疄渚?
  toolsWebview.once('tauri://close-requested', () => {
    unlistenReady();
    // console.log('鑷畾涔夊璇濇宸插叧闂?);
    toolsWebview.close();
  });
}

onMounted(async () => {
  try {
    // 鐩戝惉瀛愮獥鍙ｅ彂閫佺殑浜嬩欢
    const toolUnlisten = await listen('toolUpdated', async (event) => {
      if (event.payload?.tool) {
        store.commit("updateOneTools", [event.payload.tool]);
      }
      await loadTools(store)
      // console.log(alltools); // 杈撳嚭鎺ユ敹鍒扮殑鏁版嵁
      // Change_Tag(parseInt(selectedMenuId.value, 10), parseInt(selectedTags.value, 10));
    });
    eventUnlisteners.push(toolUnlisten);
  } catch (error) {
    console.error('监听事件失败:', error);
  }
  try {
    // 鐩戝惉瀛愮獥鍙ｅ彂閫佺殑浜嬩欢
    let menuUnlisten = await listen('menuUpdated', (event) => {
      loadMenus(store);
      loadTags(store);
    });
    let tagUnlisten = await listen('tagUpdated', (event) => {
      loadMenus(store);
      loadTags(store);
    });
    eventUnlisteners.push(menuUnlisten, tagUnlisten);
  } catch (error) {
    console.error('监听事件失败:', error);
  }
});
// function hidecontextMenu() {
//   contextToolMenu.visible = false;
//   contextToolsMenu.visible = false;
// }
// window.addEventListener("click", hidecontextMenu);
function getPathName(file) {
  const normalized = String(file || '').replace(/[\\/]+$/, '');
  const name = normalized.split(/[\\/]/).pop();
  if (!name) {
    return normalized || '未命名';
  }

  const dotIndex = name.lastIndexOf('.');
  if (dotIndex > 0) {
    return name.slice(0, dotIndex);
  }
  return name;
}

//鎷栨嫿澧炲姞宸ュ叿
async function add_tool(file) {
  const fileName = getPathName(file);
  console.log('Selected file name:', fileName);
  let data = '';
  try {
    data = await invoke("get_exe_icon", {filename: file});
  } catch (error) {
    console.warn('图标获取失败，使用默认图标:', error);
  }
  const icon = data && !data.startsWith('Error') ? data : appIconSrc;
  var add_tool = {
    id:-9999,
    name: fileName,
    icon,
    target: file,
    parameters: '',
    isadmin: false,
    number:0,
    menu_id: parseInt(selectedMenuId.value),
    tags_id: parseInt(selectedTags.value),
  }
  let tooldata = await invoke("update_tool", {tool: add_tool});
  console.log(tooldata);
  if(tooldata=="ok"){
    await loadTools(store);
    await emit('toolUpdated', { action: 'add', tool: add_tool });
    selectTags(parseInt(selectedTags.value, 10));
  }else{
    await message('保存失败!\n'+tooldata, { title: 'Error', type: 'error' });
  }
}


const dropRef = useTemplateRef('contentContainer')
const dragenter = ref(false)
let lastDragHoverKey = ''
let lastDragHoverAt = 0

function handleDragHoverSwitch(x, y) {
  const now = Date.now()
  const target = document.elementFromPoint(x, y)
  const menuItem = target?.closest?.('[data-menu-id]')
  const tagItem = target?.closest?.('[data-tag-id]')

  if (menuItem) {
    const menuId = menuItem.dataset.menuId
    const key = 'menu:' + menuId
    if (menuId && key !== lastDragHoverKey && now - lastDragHoverAt > 180) {
      lastDragHoverKey = key
      lastDragHoverAt = now
      selectMenu(Number(menuId))
    }
    return
  }

  if (tagItem) {
    const tagId = tagItem.dataset.tagId
    const key = 'tag:' + tagId
    if (tagId && key !== lastDragHoverKey && now - lastDragHoverAt > 180) {
      lastDragHoverKey = key
      lastDragHoverAt = now
      selectTags(Number(tagId))
    }
  }
}

onMounted(async () => {
  const currentWebview = getCurrentWebviewWindow();
  const dropUnlisten = await currentWebview.onDragDropEvent(async ({ payload }) => {
    const { type } = payload
    if (type === 'over') {
      const { x, y } = payload.position
      if (dropRef.value) {
        const scaleFactor = await currentWebview.scaleFactor();
        const logicalX = x / scaleFactor;
        const logicalY = y / scaleFactor;
        handleDragHoverSwitch(logicalX, logicalY)
        const { left, right, top, bottom } = dropRef.value.getBoundingClientRect()
        const inBoundsX = logicalX >= left && logicalX <= right
        const inBoundsY = logicalY >= top && logicalY <= bottom
        dragenter.value = inBoundsX && inBoundsY
      }
    } else if (type === 'drop') {
      dragenter.value = false
      lastDragHoverKey = ''
      const paths = Array.isArray(payload.paths) ? payload.paths : [];
      //澶氫釜鏂囦欢璺緞 
      for (const filepath of paths) {
        await add_tool(filepath);
      }
    } else {
      dragenter.value = false
      lastDragHoverKey = ''
    }
  })
  eventUnlisteners.push(dropUnlisten)
})

onUnmounted(() => {
  stopResize();
  removeToolPointerListeners();
  eventUnlisteners.splice(0).forEach((unlisten) => unlisten?.());
})

//鍒嗙被绌虹櫧鍖哄煙鑿滃崟
const menuDropdown = ref(null);
const menuContainer = ref(null);
const menuDropdownPoint = ref({ x: 0, y: 0 });
const menuDropdownKey = ref(0);

const globalDropdown = ref(null);
const contentContainer = ref(null);
const globalDropdownPoint = ref({ x: 0, y: 0 });
const globalDropdownKey = ref(0);

// 澶勭悊绌虹櫧鍖哄煙鍙抽敭浜嬩欢
const handleEmptyAreaContextMenu = async (event) => {
  const targetElement = event.target;
  // 鍙湁鐐瑰嚮绌虹櫧鍖哄煙鎵嶈Е鍙戝叏灞€鑿滃崟
  if (!targetElement.closest('.tools-button') && event.currentTarget.contains(targetElement)) {
    event.preventDefault();
    //鍏抽棴宸ュ叿鐨勫彸閿彍鍗?
    close_dropdown_click();
    globalDropdownPoint.value = { x: event.clientX, y: event.clientY };
    globalDropdownKey.value += 1;
    // 寮哄埗鏇存柊缁勪欢鐨?popper 閰嶇疆
    await nextTick();
    requestAnimationFrame(() => {
      if (globalDropdown.value) {
        globalDropdown.value.handleOpen();
      }
    });
  }
};
// 澶勭悊绌虹櫧鍖哄煙鍙抽敭浜嬩欢
const handleMenuContextMenu = async (event) => {
  const targetElement = event.target;
  // 鍙湁鐐瑰嚮绌虹櫧鍖哄煙鎵嶈Е鍙戝叏灞€鑿滃崟2
  if (!targetElement.closest('.el-menu') && event.currentTarget.contains(targetElement)) {
    event.preventDefault();
    //鍏抽棴宸ュ叿鐨勫彸閿彍鍗?
    close_dropdown_click();
    menuDropdownPoint.value = { x: event.clientX, y: event.clientY };
    menuDropdownKey.value += 1;
    // 寮哄埗鏇存柊缁勪欢鐨?popper 閰嶇疆
    await nextTick();
    requestAnimationFrame(() => {
      if (menuDropdown.value) {
        menuDropdown.value.handleOpen();
      }
    });
  }
};
const toolsDropdownMap = new Map();
const tagDropdownMap = new Map();
const menuDropdownMap = new Map();

function setDropdownRef(map, key, dropdown) {
  const mapKey = String(key);
  if (dropdown) {
    map.set(mapKey, dropdown);
  } else {
    map.delete(mapKey);
  }
}

function tag_dropdown_click(visible, id) {
  if(visible ){
    const currentDropdown = tagDropdownMap.get(String(id));
    close_dropdown_click({ keep: currentDropdown });
  }
}
// 澶勭悊鑿滃崟鍛戒护鐐瑰嚮
function menu_dropdown_click(visible, id) {
  if(visible ){
    const currentDropdown = menuDropdownMap.get(String(id));
    close_dropdown_click({ keep: currentDropdown });
  }
}

// 澶勭悊鍛戒护鐐瑰嚮
function tools_dropdown_click(visible, id) {
  if(visible ){
    const currentDropdown = toolsDropdownMap.get(String(id));
    close_dropdown_click({ keep: currentDropdown });
  }
}
function close_dropdown_click(skip = {}) {
  // 鍏抽棴涔嬪墠婵€娲荤殑鎵€鏈夊伐鍏?dropdown
  toolsDropdownMap.forEach((dropdown) => {
    if (dropdown === skip.keep) {
      return;
    }
    if (dropdown?.handleClose) {
      dropdown.handleClose()
    }
  })
  //鍏抽棴鏍囩
  tagDropdownMap.forEach((dropdown) => {
    if (dropdown === skip.keep) {
      return;
    }
    if (dropdown?.handleClose) {
      dropdown.handleClose()
    }
  })
  menuDropdownMap.forEach((dropdown) => {
    if (dropdown === skip.keep) {
      return;
    }
    if (dropdown?.handleClose) {
      dropdown.handleClose()
    }
  })
  globalDropdown.value?.handleClose?.();
  menuDropdown.value?.handleClose?.();
}
</script>

<template>
  <el-container >
    <el-aside :style="{ width: menuWidth + 'px' }" class="el-aside"  @contextmenu.prevent="handleMenuContextMenu" ref="menuContainer"  > 
      <el-menu :default-active="selectedMenuId"  class="el-menu-category" >
        <el-dropdown
          v-for="menu in allmenu"
          :key="menu.id"
          trigger="contextmenu"
          :ref="(dropdown) => setDropdownRef(menuDropdownMap, menu.id, dropdown)"
          style="width: 100%;"
          :teleported="true"
          @visible-change="(visible) => menu_dropdown_click(visible, menu.id)"
        >
          <el-menu-item 
            style="width:100%"
            :class="{ 'drag-over-target': dragOverMenuId === Number(menu.id) }"
            :index="menu.id.toString()"
            :data-menu-id="menu.id"
            @dragover.prevent="setDragOverMenu(menu.id)"
            @dragleave="dragOverMenuId = null"
            @click="selectMenu(menu.id)"
          >
            {{ menu.name }}
          </el-menu-item>
          <template #dropdown>
            <el-dropdown-menu >   
              <el-dropdown-item :icon="DocumentAdd" @click="menuHandleContextMenuAction('menuAdd',null)">新增</el-dropdown-item>
              <el-dropdown-item :icon="Edit" @click="menuHandleContextMenuAction('menuEdit',menu)">编辑</el-dropdown-item>
              <el-dropdown-item :icon="Delete" @click="menuHandleContextMenuAction('menuDelete',menu)">删除</el-dropdown-item>
              <el-dropdown-item :icon="DocumentAdd" @click="tagsHandleContextMenuAction('tagAdd',null)">新增标签</el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </el-menu>
      <el-dropdown
        :key="menuDropdownKey"
        ref="menuDropdown"
        trigger="manual"  
        placement="bottom-end"
        :teleported="true"
      >
        <!-- <span style="position: absolute; top: 0; left: 0; pointer-events: none;"></span> -->
        <span
          :style="{
            position: 'fixed',
            left: menuDropdownPoint.x + 'px',
            top: menuDropdownPoint.y + 'px',
            width: '1px',
            height: '1px',
            pointerEvents: 'none'
          }"
        ></span>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item :icon="DocumentAdd" @click="menuHandleContextMenuAction('menuAdd',null)">新增分类</el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </el-aside>
    <div class="resize-handle" :style="{ left: menuWidth + 'px' }" @mousedown="startResize"></div> 
    <div class="tools-container">
      <el-menu mode="horizontal" :default-active="selectedTags" class="el-menu-tags" :ellipsis="false">
        <el-dropdown
          v-for="tag in filteredTags"
          :key="tag.id"
          trigger="contextmenu"
          :ref="(dropdown) => setDropdownRef(tagDropdownMap, tag.id, dropdown)"
          :teleported="true"
          @visible-change="(visible) => tag_dropdown_click(visible, tag.id)"
        >
          <el-menu-item
            :class="{ 'drag-over-target': dragOverTagId === Number(tag.id) }"
            :index="tag.id.toString()"
            :data-tag-id="tag.id"
            @dragover.prevent="setDragOverTag(tag.id)"
            @dragleave="dragOverTagId = null"
            @click="selectTags(tag.id)"
          >
            {{ tag.name }}   
          </el-menu-item>
          <template #dropdown>
            <el-dropdown-menu >   
              <el-dropdown-item :icon="DocumentAdd" @click="tagsHandleContextMenuAction('tagAdd',tag)">新增</el-dropdown-item>
              <el-dropdown-item :icon="Edit" @click="tagsHandleContextMenuAction('tagEdit',tag)">编辑</el-dropdown-item>
              <el-dropdown-item :icon="Delete" @click="tagsHandleContextMenuAction('tagDelete',tag)">删除</el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </el-menu>
      <el-dropdown
        :key="globalDropdownKey"
        ref="globalDropdown"
        trigger="manual"  
        placement="bottom-start"
        :teleported="true"
      >
        <span
          :style="{
            position: 'fixed',
            left: globalDropdownPoint.x + 'px',
            top: globalDropdownPoint.y + 'px',
            width: '1px',
            height: '1px',
            pointerEvents: 'none'
          }"
        ></span>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item :icon="DocumentAdd" @click="toolsHandle('add',null)">新建项目</el-dropdown-item>
            <el-dropdown-item :icon="Clock" @click="toolsContainerHandleContextMenuAction('recentSort')">最近使用置顶</el-dropdown-item>
            <el-dropdown-item :icon="Sort" @click="toolsContainerHandleContextMenuAction('nameSort')">按名称排序</el-dropdown-item>
            <el-dropdown-item :icon="Sort" @click="toolsContainerHandleContextMenuAction('numberSort')">按使用次数排序</el-dropdown-item>
            <el-dropdown-item :icon="Remove" @click="toolsContainerHandleContextMenuAction('clear_no')">清除无效项目</el-dropdown-item>
            <el-dropdown-item :icon="Refresh" @click="toolsContainerHandleContextMenuAction('flushed')">刷新</el-dropdown-item>
            <el-dropdown-item :icon="Delete" @click="toolsContainerHandleContextMenuAction('clear')">清空</el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
      <div
        class="tools-content"
        :class="{ 'tool-drop-active': isToolDropActive, 'tool-move-active': isToolMoveActive }"
        @contextmenu="handleEmptyAreaContextMenu"
        @dragenter.prevent="handleToolsDragOver"
        @dragover="handleToolsDragOver"
        @drop.stop.prevent="dropToolToCurrentSelection"
        ref="contentContainer"
      >
        <el-dropdown
          v-for="tools in filteredTools"
          :key="tools.id"
          trigger="contextmenu"
          :ref="(dropdown) => setDropdownRef(toolsDropdownMap, tools.id, dropdown)"
          :teleported="true"
          @visible-change="(visible) => tools_dropdown_click(visible, tools.id)"
        >
          <!-- <el-tooltip
            :content="tools.target"
            :effect="skin"
          >
            <el-button 
              class="tools-button" 
              @click="Exec_Invoke('open_tools',tools,false)" 
              :key="tools.id"
            >
              <img 
                class="tools-button-icon"
                :src="tools.icon" 
                alt="工具图标" 
              />
              {{ tools.name }}
            </el-button>
          </el-tooltip> -->
          <div
            class="tools-button el-button"
            :class="{ 'is-moving-tool': draggingTool && Number(draggingTool.id) === Number(tools.id) }"
            draggable="false"
            @pointerdown="startToolPointerDrag($event, tools)"
            @dragstart.prevent
            @click="handleToolClick(tools)"
          >
            <el-tooltip :content="tools.target" effect="light">
              <span class="tools-button-inner">
                  <img class="tools-button-icon" :src="tools.icon" alt="工具图标" draggable="false" />
                  <span class="tools-button-title">{{ tools.name }}</span>
              </span>
            </el-tooltip>
          </div>
          <template #dropdown>
            <el-dropdown-menu>  
              <el-dropdown-item :icon="ArrowRight" @click="toolsHandle('run',tools)">运行</el-dropdown-item>
              <el-dropdown-item :icon="ArrowRightBold" @click="toolsHandle('runAsAdmin',tools)">以管理员运行</el-dropdown-item>
              <el-dropdown-item :icon="Link" @click="toolsHandle('copyPath',tools)">复制完整路径</el-dropdown-item>
              <el-dropdown-item :icon="Folder" @click="toolsHandle('openDir',tools)">打开文件位置</el-dropdown-item>
              <el-dropdown-item :icon="Monitor" @click="toolsHandle('openCmd',tools)">打开命令行</el-dropdown-item>
              <el-dropdown-item :icon="Platform" @click="toolsHandle('openCmdAsAdmin',tools)">以管理员打开命令行</el-dropdown-item>
              <el-dropdown-item :icon="DocumentAdd" @click="toolsHandle('add',tools)">新建项目</el-dropdown-item>
              <el-dropdown-item :icon="Edit" @click="toolsHandle('edit',tools)">编辑项目</el-dropdown-item>
              <el-dropdown-item :icon="Delete" @click="toolsHandle('delete',tools)">删除项目</el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
        <div v-if="isToolMoveActive" class="tool-drop-message">
          {{ isToolDropActive ? '松开移动到 ' + toolMoveTargetText + '，右键取消' : '移到工具区域后松开，右键取消' }}
        </div>
      </div>
      <div
        v-if="isToolMoveActive && draggingTool"
        class="tool-drag-ghost"
        :style="{ transform: `translate(${toolDragPoint.x + 12}px, ${toolDragPoint.y + 12}px)` }"
      >
        <img class="tool-drag-ghost-icon" :src="draggingTool.icon" alt="" draggable="false">
        <span>{{ draggingTool.name }}</span>
        <span class="tool-drag-ghost-tip">右键取消</span>
      </div>
    </div>
  </el-container>
</template>

<style scoped>

.tools-button-inner {
  display: flex;
  align-items: center;
  min-width: 0;
}

.is-moving-tool {
  opacity: 0.45;
}

.drag-over-target {
  outline: 1px solid var(--el-color-primary);
  outline-offset: -2px;
  background: var(--st-active-bg) !important;
}

.tool-move-active {
  position: relative;
}

.tool-drop-active {
  box-shadow: inset 0 0 0 1px var(--el-color-primary);
  background: color-mix(in srgb, var(--el-color-primary) 5%, var(--st-content-bg)) !important;
}

.tool-drop-message {
  position: sticky;
  left: 50%;
  bottom: 10px;
  transform: translateX(-50%);
  z-index: 20;
  padding: 7px 12px;
  border-radius: 6px;
  background: var(--el-bg-color-overlay);
  border: 1px solid var(--st-border-soft);
  box-shadow: var(--st-shadow-menu);
  color: var(--el-text-color-primary);
  font-size: 12px;
  pointer-events: none;
  white-space: nowrap;
}

.tool-drag-ghost {
  position: fixed;
  left: 0;
  top: 0;
  z-index: 3000;
  display: inline-flex;
  align-items: center;
  max-width: 220px;
  padding: 6px 9px;
  border-radius: 6px;
  background: var(--el-bg-color-overlay);
  border: 1px solid var(--st-border-soft);
  box-shadow: var(--st-shadow-menu);
  color: var(--el-text-color-primary);
  font-size: 12px;
  pointer-events: none;
}

.tool-drag-ghost-icon {
  width: 20px;
  height: 20px;
  margin-right: 6px;
  object-fit: contain;
}

.tool-drag-ghost-tip {
  margin-left: 8px;
  color: var(--el-text-color-secondary);
}
</style>
