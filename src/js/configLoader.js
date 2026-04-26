import { invoke } from "@tauri-apps/api/core";
import { message } from '@tauri-apps/plugin-dialog';
import { watch } from 'vue';
import { Window, LogicalPosition, LogicalSize, currentMonitor } from '@tauri-apps/api/window';
import { getAllWebviewWindows } from '@tauri-apps/api/webviewWindow';
import { listen } from '@tauri-apps/api/event';

let configValue = {};
let timer = null;
let pendingUpdates = {};

const appWindow = new Window('main');
const edgeThreshold = 4;
const dragHideGuardMs = 800;

let isUpdatingConfig = false;
let isWindowHidden = false;
let isDragging = false;
let hiddenEdge = null;
let lastWindowMoveAt = 0;
let watchersReady = false;

const toBoolean = (value) => value === true || value === 'true' || value === 1 || value === '1';
const toNumber = (value, fallback = 0) => {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
};

async function hasVisibleSettingsWindow() {
  const windows = await getAllWebviewWindows();
  for (const webviewWindow of windows) {
    if (webviewWindow.label !== 'config') {
      continue;
    }
    try {
      if (await webviewWindow.isVisible()) {
        return true;
      }
    } catch (error) {
      console.warn('检查子窗口状态失败:', error);
    }
  }
  return false;
}

export async function loadConfig(store) {
  const Config = await invoke("load_config", {});
  const isEmpty = Object.keys(Config).length === 0 && Config.constructor === Object;
  if (isEmpty) {
    await message('配置加载失败!', { title: 'Error', type: 'error' });
    return;
  }

  configValue = { ...Config };
  if (store) {
    store.commit('updateConfig', Config);
    console.log(`加载配置成功: ${JSON.stringify(Config)}`);
    updatePosition(Config.X, Config.Y);
    updateSize(Config.Width, Config.Height);
  }
}

async function updatePosition(newx, newy) {
  if (newx === null || newx === undefined || newy === null || newy === undefined) {
    console.error('无效的窗口位置:', newx, newy);
    return;
  }

  const x = parseFloat(newx);
  const y = parseFloat(newy);
  if (Number.isNaN(x) || Number.isNaN(y)) {
    console.error('无效的窗口位置:', newx, newy);
    return;
  }

  const position = new LogicalPosition(x, y);
  appWindow.setPosition(position).then(() => {
    console.log(`窗口位置: ${x},${y}`);
  }).catch((error) => {
    console.error('设置窗口位置失败:', error);
    message('设置窗口位置失败: ' + error, { title: 'Error', type: 'error' });
  });
}

async function getCurrentMonitorBounds() {
  const monitor = await currentMonitor();
  if (!monitor) {
    return null;
  }

  const scaleFactor = monitor.scaleFactor || await appWindow.scaleFactor();
  return {
    x: Math.round(monitor.position.x / scaleFactor),
    y: Math.round(monitor.position.y / scaleFactor),
    width: Math.round(monitor.size.width / scaleFactor),
    height: Math.round(monitor.size.height / scaleFactor),
    scaleFactor,
  };
}

async function getLogicalWindowRect() {
  const [position, size, scaleFactor] = await Promise.all([
    appWindow.outerPosition(),
    appWindow.outerSize(),
    appWindow.scaleFactor(),
  ]);

  return {
    x: Math.round(position.x / scaleFactor),
    y: Math.round(position.y / scaleFactor),
    width: Math.round(size.width / scaleFactor),
    height: Math.round(size.height / scaleFactor),
    scaleFactor,
  };
}

function clampWindowPosition(rect, monitorBounds) {
  if (!monitorBounds) {
    return { x: rect.x, y: rect.y };
  }

  return {
    x: Math.min(
      Math.max(rect.x, monitorBounds.x),
      monitorBounds.x + monitorBounds.width - rect.width
    ),
    y: Math.min(
      Math.max(rect.y, monitorBounds.y),
      monitorBounds.y + monitorBounds.height - rect.height
    ),
  };
}

function commitWindowPosition(store, x, y) {
  const currentConfig = store.state.Config;
  if (
    toNumber(currentConfig.X) === x &&
    toNumber(currentConfig.Y) === y
  ) {
    return;
  }

  isUpdatingConfig = true;
  store.commit('updateConfigPosition', { x, y });
  isUpdatingConfig = false;
}

async function updateSize(Width, Height) {
  try {
    const width = parseFloat(Width || 450);
    const height = parseFloat(Height || 800);
    const size = new LogicalSize(width, height);
    appWindow.setSize(size);
    console.log(`窗口大小: ${width}x${height}`);
  } catch (error) {
    console.error('设置窗口大小失败:', error);
  }
}

function updateLocked_Size(value) {
  const isLockedSize = toBoolean(value);
  console.log('锁定窗口大小: ' + isLockedSize);
  appWindow.setResizable(!isLockedSize);
}

function updateTop_Window(value) {
  const isTopWindow = toBoolean(value);
  console.log('窗口置顶: ' + isTopWindow);
  appWindow.setAlwaysOnTop(isTopWindow);
}

function updateSilent_Start(value) {
  const isSilentStart = toBoolean(value);
  console.log('后台启动: ' + isSilentStart);
  if (isSilentStart) {
    appWindow.hide();
  } else {
    appWindow.show();
    appWindow.setFocus();
  }
}

async function updateConfig(updates) {
  const result = await invoke("update_config", { config: updates });
  if (result === 'ok') {
    console.log(result);
    console.log(`配置更新成功: ${JSON.stringify(pendingUpdates)}`);
    pendingUpdates = {};
  } else {
    console.log(`配置更新失败: ${result}`);
    await message('配置更新失败!' + result, { title: 'Error', type: 'error' });
  }
}

function isApproxEqual(a, b, tolerance = 20) {
  return Math.abs(a - b) <= tolerance;
}

export function setupConfigWatchers(store) {
  if (watchersReady) {
    return;
  }
  watchersReady = true;

  if (store) {
    watch(() => store.state.Config.Locked_Size, (newValue) => {
      updateLocked_Size(newValue);
    });

    watch(() => store.state.Config.Top_Window, (newValue) => {
      updateTop_Window(newValue);
    });

    async function setupResizeListener() {
      let resizeTimeout;
      await listen('tauri://resize', async () => {
        if (await appWindow.isMinimized() || await appWindow.isMaximized()) {
          return;
        }
        if (isUpdatingConfig) {
          return;
        }

        if (resizeTimeout) clearTimeout(resizeTimeout);
        resizeTimeout = setTimeout(async () => {
          const size = await appWindow.outerSize();
          const scaleFactor = await appWindow.scaleFactor();
          const logicalWidth = Math.round(size.width / scaleFactor);
          const logicalHeight = Math.round(size.height / scaleFactor);

          console.log(`新的窗口大小: 宽度=${logicalWidth}, 高度=${logicalHeight}`);

          const currentConfig = store.state.Config;
          if (
            !isApproxEqual(parseFloat(currentConfig.Width), logicalWidth) ||
            !isApproxEqual(parseFloat(currentConfig.Height), logicalHeight)
          ) {
            isUpdatingConfig = true;
            store.commit('updateConfigSize', { width: logicalWidth, height: logicalHeight });
            isUpdatingConfig = false;
          }
        }, 300);
      });
    }

    async function setupMoveListener() {
      let moveTimeout;
      await listen('tauri://move', async () => {
        isDragging = true;
        lastWindowMoveAt = Date.now();

        if (isUpdatingConfig) {
          return;
        }
        if (moveTimeout) clearTimeout(moveTimeout);
        if (await appWindow.isMinimized() || await appWindow.isMaximized()) {
          return;
        }

        moveTimeout = setTimeout(async () => {
          const rect = await getLogicalWindowRect();
          commitWindowPosition(store, rect.x, rect.y);
        }, 250);
      });
    }

    async function setupBlurListener() {
      await listen('tauri://blur', async () => {
        if (await appWindow.isMinimized() || await appWindow.isMaximized()) {
          return;
        }

        setTimeout(async () => {
          if (await hasVisibleSettingsWindow()) {
            return;
          }

          const monitorBounds = await getCurrentMonitorBounds();
          if (!monitorBounds) {
            return;
          }

          const rect = await getLogicalWindowRect();

          if (isDragging) {
            const correctedPosition = clampWindowPosition(rect, monitorBounds);
            const correctedOutOfBounds = correctedPosition.x !== rect.x || correctedPosition.y !== rect.y;

            if (correctedOutOfBounds) {
              await updatePosition(correctedPosition.x, correctedPosition.y);
            }

            commitWindowPosition(store, correctedPosition.x, correctedPosition.y);
            isDragging = false;

            if (correctedOutOfBounds) {
              return;
            }
            if (Date.now() - lastWindowMoveAt < dragHideGuardMs) {
              return;
            }
          }

          const logicalX = rect.x;
          const logicalY = rect.y;
          const logicalWidth = rect.width;
          const logicalHeight = rect.height;
          const logicalEdgeThreshold = edgeThreshold;

          const isNearLeftEdge = logicalX <= monitorBounds.x + logicalEdgeThreshold;
          const isNearRightEdge = logicalX + logicalWidth >= monitorBounds.x + monitorBounds.width - logicalEdgeThreshold;
          const isNearTopEdge = logicalY <= monitorBounds.y + logicalEdgeThreshold;
          const isNearBottomEdge = logicalY + logicalHeight >= monitorBounds.y + monitorBounds.height - logicalEdgeThreshold;

          if ((isNearLeftEdge || isNearRightEdge || isNearTopEdge || isNearBottomEdge) && !isWindowHidden) {
            let snappedPosition = null;

            if (isNearLeftEdge) {
              hiddenEdge = 'left';
              snappedPosition = { x: monitorBounds.x, y: logicalY };
            } else if (isNearRightEdge) {
              hiddenEdge = 'right';
              snappedPosition = { x: monitorBounds.x + monitorBounds.width - logicalWidth, y: logicalY };
            } else if (isNearTopEdge) {
              hiddenEdge = 'top';
              snappedPosition = { x: logicalX, y: monitorBounds.y };
            } else if (isNearBottomEdge) {
              hiddenEdge = 'bottom';
              snappedPosition = { x: logicalX, y: monitorBounds.y + monitorBounds.height - logicalHeight };
            }

            if (snappedPosition) {
              await updatePosition(snappedPosition.x, snappedPosition.y);
              commitWindowPosition(store, snappedPosition.x, snappedPosition.y);
            }

            console.log(`窗口靠近 ${hiddenEdge} 边缘，隐藏窗口`);
            await appWindow.hide();
            isWindowHidden = true;
          }
        }, 200);
      });
    }

    updateSilent_Start(store.state.Config.Startup_Background);
    setupBlurListener();
    setupResizeListener();
    setupMoveListener();

    watch(
      () => store.state.Config,
      (newValue) => {
        if (JSON.stringify(configValue) === JSON.stringify(newValue)) {
          return;
        }

        if (timer) {
          clearTimeout(timer);
        }

        let isUpdate = false;
        for (const key in newValue) {
          if (newValue[key] !== configValue[key]) {
            isUpdate = true;

            pendingUpdates[key] = newValue[key];
          }
        }

        configValue = { ...newValue };
        if (isUpdate && Object.keys(pendingUpdates).length > 0) {
          timer = setTimeout(() => {
            console.log(`3 秒内没有新的变化，执行配置更新: ${JSON.stringify(pendingUpdates)}`);
            updateConfig(pendingUpdates);
          }, 3000);
        }
      },
      { deep: true, immediate: false }
    );
  }

  async function setupGlobalMouseEventListener(store) {
    const unlisten = await listen('mouse-near-edge', async (event) => {
      if (!isWindowHidden) {
        return;
      }
      if (store.state.isClose) {
        return;
      }
      if (await appWindow.isMinimized() || await appWindow.isMaximized()) {
        return;
      }

      const scaleFactor = await appWindow.scaleFactor();
      const windowBounds = await appWindow.outerPosition();
      const size = await appWindow.outerSize();
      const windowX = Math.round(windowBounds.x / scaleFactor);
      const windowY = Math.round(windowBounds.y / scaleFactor);
      const windowWidth = Math.round(size.width / scaleFactor);
      const windowHeight = Math.round(size.height / scaleFactor);

      const { x: mouseX, y: mouseY } = event.payload;
      const logicalMouseX = Math.round(mouseX / scaleFactor);
      const logicalMouseY = Math.round(mouseY / scaleFactor);
      const screenWidth = window.screen.width;
      const screenHeight = window.screen.height;

      const isWindowNearLeftEdge = windowX <= edgeThreshold;
      const isWindowNearRightEdge = windowX + windowWidth >= screenWidth - edgeThreshold;
      const isWindowNearTopEdge = windowY <= edgeThreshold;
      const isWindowNearBottomEdge = windowY + windowHeight >= screenHeight - edgeThreshold;

      const isMouseNearLeftEdge = logicalMouseX <= edgeThreshold;
      const isMouseNearRightEdge = logicalMouseX >= screenWidth - edgeThreshold;
      const isMouseNearTopEdge = logicalMouseY <= edgeThreshold;
      const isMouseNearBottomEdge = logicalMouseY >= screenHeight - edgeThreshold;

      let currentEdge = null;
      if (isMouseNearLeftEdge && isWindowNearLeftEdge) {
        currentEdge = 'left';
      } else if (isMouseNearRightEdge && isWindowNearRightEdge) {
        currentEdge = 'right';
      } else if (isMouseNearTopEdge && isWindowNearTopEdge) {
        currentEdge = 'top';
      } else if (isMouseNearBottomEdge && isWindowNearBottomEdge) {
        currentEdge = 'bottom';
      }

      if (currentEdge && currentEdge === hiddenEdge) {
        await appWindow.show();
        await appWindow.setFocus();
        isWindowHidden = false;
        hiddenEdge = null;
      }
    });

    await listen('isclose-edge', async (event) => {
      isWindowHidden = event.payload;
      store.commit('updateClose', false);
      if (event.payload) {
        appWindow.setFocus();
      }
    });

    return unlisten;
  }

  setupGlobalMouseEventListener(store);

  // 禁用右键和开发者工具的逻辑先保留为注释，后续需要时再打开。
  // document.onkeydown = function (event) {
  //   const e = event || window.event;
  //   if (e.keyCode === 123) {
  //     e.preventDefault();
  //     return false;
  //   }
  // };
}
