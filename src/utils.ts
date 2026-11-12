import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { load as loadStore } from '@tauri-apps/plugin-store'
import type { ActiveWidget } from '@/types'

export const activeWidgetsStore = await loadStore('active-widgets-store.json', { autoSave: false })

export function getWidgetKey(id: string, instanceId: string) {
  return `${id}/${instanceId}`
}

export function createWidgetWindow({ meta, instanceId, isLocked, height, width, x, y, data: _ }: ActiveWidget) {
  const key = getWidgetKey(meta.id, instanceId)
  const url = `/widgets/${key}`

  const window = new WebviewWindow(key, {
    url,
    title: meta.name,
    height,
    width,
    x,
    y,
    resizable: !isLocked,
    minHeight: 240,
    minWidth: 240,
    shadow: false,
    focus: false,
    visible: false,
    transparent: true,
    decorations: false,
    skipTaskbar: true,
    maximizable: false,
    minimizable: false,
  })

  window.once('tauri://created', async () => {
    if (import.meta.env.PROD) await window.setAlwaysOnBottom(true)
    await window.show()
  })

  window.once('tauri://error', err => console.error(err))

  return window
}

export function extractErrorMessage(err: unknown) {
  const defaultMessage = 'unknown error'

  if (!err) return defaultMessage
  else if (err instanceof Error) return err.message
  else if (typeof err === 'string') return err
  else if (typeof err === 'object' && 'message' in err) return err.message
  else {
    try {
      return JSON.stringify(err)
    } catch {
      return defaultMessage
    }
  }
}
