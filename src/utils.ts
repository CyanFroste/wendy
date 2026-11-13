import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { load as loadStore } from '@tauri-apps/plugin-store'
import { attach as attachWindowAsWallpaper, setInteractive } from 'tauri-plugin-wallpaper'
import { createStore } from 'zustand'
import { persist } from 'zustand/middleware'
import { startAudioCapture, startMediaListener } from '@/commands'
import type { ActiveWidget } from '@/types'

export const activeWidgetsStore = await loadStore('active-widgets-store.json', { autoSave: false })

export function getWidgetKey(id: string, instanceId: string) {
  return `${id}/${instanceId}`
}

export function createWidgetWindow({ meta, instanceId, isLocked, isPinned, height, width, x, y }: ActiveWidget) {
  const key = getWidgetKey(meta.id, instanceId)
  const url = `/widgets/${key}`

  const window = new WebviewWindow(key, {
    title: meta.name,
    url,
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
    if (!meta.isBackground) {
      if (isPinned) await window.setAlwaysOnTop(true)
      else await window.setAlwaysOnBottom(true)
    }

    await window.show()

    if (meta.isBackground) {
      await attachWindowAsWallpaper(key)
      if (import.meta.env.DEV) await setInteractive(true, key)
    }

    if (meta.permissions.some(it => it === 'media:read')) {
      await startMediaListener(key)
    }

    if (meta.permissions.some(it => it === 'audio:read')) {
      await startAudioCapture(key)
    }
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

type GlobalStore = { widgetsPath: string; refetchCounter: number }

export const globalStore = createStore<GlobalStore>()(
  persist(() => ({ widgetsPath: '', refetchCounter: 0 }), {
    name: 'settings',
    partialize: ({ refetchCounter, ...state }) => state,
  }),
)

export function htmlSafeJSON(value: unknown) {
  return JSON.stringify(value ?? null).replace(/</g, '\\u003c')
}
