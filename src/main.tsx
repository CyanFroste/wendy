import '@/styles.css'

import React from 'react'
import ReactDOM from 'react-dom/client'
import { createBrowserRouter, RouterProvider } from 'react-router'
import {
  getCurrentWindow,
  getAllWindows,
  // currentMonitor as getCurrentMonitor,
  // PhysicalPosition,
  // PhysicalSize,
} from '@tauri-apps/api/window'
import { TrayIcon } from '@tauri-apps/api/tray'
import { Menu } from '@tauri-apps/api/menu'
import { defaultWindowIcon } from '@tauri-apps/api/app'
// import { attach as attachWindowAsWallpaper, setInteractive } from 'tauri-plugin-wallpaper'
import { Window } from '@/components/windows'
import { Settings } from '@/components/settings'
import { Picker } from '@/components/picker'
import { Editor } from '@/components/editor'
import { Widget } from '@/components/widget'
import { activeWidgetsStore, createWidgetWindow } from '@/utils'
import type { ActiveWidget } from '@/types'

// await attachWindowAsWallpaper('main')
// await setInteractive(true, 'main')

const currentWindow = getCurrentWindow()

const router = createBrowserRouter([
  {
    path: '/',
    Component: Window,
    children: [
      { path: '/', Component: Picker },
      { path: '/settings', Component: Settings },
      { path: '/editor/:id/:instanceId?', Component: Editor },
    ],
  },
  { path: '/widgets/:id/:instanceId', Component: Widget },
])

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <RouterProvider router={router} />
  </React.StrictMode>,
)

if (currentWindow.label === 'main' && window.location.pathname === '/') {
  for (const widget of await activeWidgetsStore.values<ActiveWidget>()) {
    createWidgetWindow(widget)
  }

  currentWindow.onCloseRequested(async () => {
    const windows = (await getAllWindows()).filter(w => w.label !== 'main')
    if (!windows.length) return

    await activeWidgetsStore.reload()

    for (const window of windows) {
      const key = window.label
      const existing = await activeWidgetsStore.get<ActiveWidget>(key)
      if (!existing) continue

      const { width, height } = await window.outerSize()
      const { x, y } = await window.outerPosition()

      await activeWidgetsStore.set(key, { ...existing, x, y, width, height })
      await window.close()
    }

    await activeWidgetsStore.save()
  })

  await TrayIcon.new({
    title: 'wendy',
    icon: (await defaultWindowIcon())!,
    menuOnLeftClick: false,
    menu: await Menu.new({
      items: [
        { id: 'open', text: 'Open', action: () => currentWindow.show() },
        { id: 'close', text: 'Close', action: () => currentWindow.close() },
      ],
    }),
    action: async evt => {
      switch (evt.type) {
        case 'DoubleClick':
          await currentWindow.show()
          break
      }
    },
  })
}

document.addEventListener('contextmenu', evt => {
  if (import.meta.env.PROD) evt.preventDefault()
})

window.addEventListener('keydown', evt => {
  const key = evt.key.toLowerCase()
  if (import.meta.env.PROD && evt.ctrlKey && (key === 'r' || key === 'f')) evt.preventDefault()
})
