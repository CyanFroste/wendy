import '@/styles.css'

import React from 'react'
import ReactDOM from 'react-dom/client'
import { createBrowserRouter, RouterProvider } from 'react-router'
import { getCurrentWindow, getAllWindows } from '@tauri-apps/api/window'
import { TrayIcon } from '@tauri-apps/api/tray'
import { Menu } from '@tauri-apps/api/menu'
import { defaultWindowIcon } from '@tauri-apps/api/app'
import { warn, debug, trace, info, error } from '@tauri-apps/plugin-log'
import { Window } from '@/components/window'
import { Settings } from '@/components/settings'
import { Widgets } from '@/components/widgets'
import { WidgetEditor } from '@/components/widget-editor'
import { Widget } from '@/components/widget'
import { ActiveWidgets } from '@/components/active-widgets'
import { Develop } from '@/components/develop'
import { Help } from '@/components/help'
import { init as initBackend } from '@/commands'
import { activeWidgetsStore, createWidgetWindow } from '@/utils'
import type { ActiveWidget } from '@/types'

const currentWindow = getCurrentWindow()

const router = createBrowserRouter([
  {
    path: '/',
    Component: Window,
    children: [
      { path: '/', Component: Widgets },
      { path: '/active', Component: ActiveWidgets },
      { path: '/settings', Component: Settings },
      { path: '/editor/:id/:instanceId?', Component: WidgetEditor },
      { path: '/develop', Component: Develop },
      { path: '/help', Component: Help },
    ],
  },
  { path: '/widgets/:id/:instanceId', Component: Widget },
])

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <RouterProvider router={router} />
  </React.StrictMode>,
)

document.addEventListener('contextmenu', evt => {
  if (import.meta.env.PROD) evt.preventDefault()
})

window.addEventListener('keydown', evt => {
  const key = evt.key.toLowerCase()
  if (import.meta.env.PROD && evt.ctrlKey && (key === 'r' || key === 'f')) evt.preventDefault()
})

try {
  if (currentWindow.label === 'main' && window.location.pathname === '/') {
    console.info('creating widgets and tray icon...')

    for (const widget of await activeWidgetsStore.values<ActiveWidget>()) {
      createWidgetWindow(widget)
    }

    currentWindow.onCloseRequested(async () => {
      try {
        console.info('closing all widgets...')

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
      } catch (err) {
        console.error(err)
      }
    })

    await TrayIcon.new({
      title: 'wendy',
      icon: (await defaultWindowIcon())!,
      menuOnLeftClick: false,
      menu: await Menu.new({
        items: [
          { id: 'show-widgets', text: 'Show all widgets', action: focusAllWidgets },
          { id: 'show-main', text: 'Show main window', action: showMainWindow },
          { id: 'close', text: 'Close', action: () => currentWindow.close() },
        ],
      }),
      action: async evt => {
        switch (evt.type) {
          case 'DoubleClick':
            await showMainWindow()
            break
        }
      },
    })
  }

  await showMainWindow()
  await initBackend()
} catch (err) {
  console.error(err)
}

if (import.meta.env.PROD) {
  forwardConsole('log', trace)
  forwardConsole('debug', debug)
  forwardConsole('info', info)
  forwardConsole('warn', warn)
  forwardConsole('error', error)
}

async function showMainWindow() {
  await currentWindow.show()
  await currentWindow.setFocus()
  await currentWindow.unminimize()
}

async function focusAllWidgets() {
  // unable to focus all widget windows if main window is hidden
  await showMainWindow()
  await Promise.allSettled((await getAllWindows()).filter(w => w.label !== 'main').map(w => w.setFocus()))
}

function forwardConsole(type: 'log' | 'debug' | 'info' | 'warn' | 'error', logger: (message: string) => Promise<void>) {
  const original = console[type]

  console[type] = (message: string) => {
    original(message)
    logger(message)
  }
}
