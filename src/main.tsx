import '@/styles.css'

import React from 'react'
import ReactDOM from 'react-dom/client'
import { createBrowserRouter, RouterProvider } from 'react-router'
// import {
//   getCurrentWindow,
//   currentMonitor as getCurrentMonitor,
//   PhysicalPosition,
//   PhysicalSize,
// } from '@tauri-apps/api/window'
// import { attach as attachWindowAsWallpaper, setInteractive } from 'tauri-plugin-wallpaper'
import { Window } from '@/components/windows'
import { Settings } from '@/components/settings'
import { Picker } from '@/components/picker'
import { Editor } from '@/components/editor'
import { Widget } from '@/components/widget'

// await attachWindowAsWallpaper('main')
// await setInteractive(true, 'main')

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
