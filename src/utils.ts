import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { load as loadStore } from '@tauri-apps/plugin-store'
import { createStore } from 'zustand'
import type { ActiveWidget, Dictionary } from '@/types'

export const activeWidgetsStore = await loadStore('active-widgets-store.json', { autoSave: false })

type WidgetWindowsStore = Dictionary<WebviewWindow>

export const widgetWindowsStore = createStore<WidgetWindowsStore>(() => ({}))

export function createWidgetWindow({
  meta,
  instanceId = crypto.randomUUID(),
  isLocked = false,
  height = 400,
  width = 400,
  x = 20,
  y = 20,
  data,
}: Partial<ActiveWidget> & Required<Pick<ActiveWidget, 'meta'>>) {
  const key = `${meta.id}/${instanceId}`
  const url = `/widgets/${key}`

  const window = new WebviewWindow(key, {
    url,
    title: meta.name,
    height,
    width,
    x,
    y,
    resizable: isLocked,
    minHeight: 400,
    minWidth: 400,
    shadow: false,
    focus: false,
    visible: false,
    transparent: true,
    decorations: false,
    // skipTaskbar: true,
  })

  window.once('tauri://created', async () => {
    await window.setAlwaysOnBottom(true)
    await window.show()
  })

  window.once('tauri://error', err => console.error(err))

  return { window, key, instanceId, meta, isLocked, height, width, x, y, data }
}
