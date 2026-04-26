import { WebviewWindow, getAllWebviewWindows } from '@tauri-apps/api/webviewWindow';

export async function createWindow(label, options = {}) {
  const existingWindows = await getAllWebviewWindows();
  const existingWindow = existingWindows.find((window) => window.label === label);
  if (existingWindow) {
    await existingWindow.show();
    await existingWindow.center();
    await existingWindow.setFocus();
    return existingWindow;
  }

  const webview = new WebviewWindow(label, {
    center: true,
    visible: false,
    focus: true,
    resizable: true,
    ...options,
  });

  webview.once('tauri://created', async () => {
    await webview.show();
    await webview.setFocus();
  });

  return webview;
}
