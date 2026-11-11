import { useEffect, useRef, useState } from 'react'
import { useParams } from 'react-router'
import { readTextFile } from '@tauri-apps/plugin-fs'
import { convertFileSrc } from '@tauri-apps/api/core'
import { Button, Separator } from '@heroui/react'
import { PiArrowClockwise, PiPlus } from 'react-icons/pi'
import { getWidget } from '@/commands'
import { activeWidgetsStore, createWidgetWindow, getWidgetKey, widgetWindowsStore } from '@/utils'
import { store as settingsStore } from '@/components/settings'
import preludeScript from '@/scripts/widget-editor-prelude?raw'
import type { ActiveWidget, Nullable, Widget } from '@/types'

type PathParams = { id: string; instanceId?: string }

export function Editor() {
  const { id, instanceId } = useParams<PathParams>()
  const editorRef = useRef<HTMLIFrameElement>(null)

  const [widget, setWidget] = useState<Nullable<Widget>>(null)
  const [srcDoc, setSrcDoc] = useState('')

  const onCreate = async (widget: Widget, data?: Nullable<unknown>) => {
    const instanceId = crypto.randomUUID()
    const key = getWidgetKey(widget.id, instanceId)

    const activeWidget: ActiveWidget = {
      instanceId,
      meta: widget,
      isLocked: false,
      height: 400,
      width: 400,
      x: 20,
      y: 20,
      data,
    }

    await activeWidgetsStore.set(key, activeWidget)
    await activeWidgetsStore.save()

    const window = createWidgetWindow(activeWidget)
    widgetWindowsStore.setState(state => ({ ...state, [key]: window }))
  }

  const onAction = async () => {
    if (!editorRef.current || !widget) return

    const data = await new Promise((resolve, reject) => {
      const editorWindow = editorRef.current?.contentWindow
      if (!editorWindow) return reject(new Error('no editor window'))

      const timeoutId = setTimeout(() => {
        cleanup()
        reject(new Error('widget editor response timed out'))
      }, 5000)

      const handler = (evt: MessageEvent) => {
        if (evt.source === editorWindow && evt.data.type === 'WIDGET_EDITOR_DATA') {
          cleanup()
          resolve(evt.data.payload)
        }
      }

      const cleanup = () => {
        clearTimeout(timeoutId)
        window.removeEventListener('message', handler)
      }

      window.addEventListener('message', handler)
      editorWindow.postMessage({ type: 'REQUEST_WIDGET_EDITOR_DATA' }, '*')
    })

    // TODO
    if (instanceId) return
    else onCreate(widget, data)
  }

  useEffect(() => {
    if (!id) return
    ;(async () => {
      const widget = await getWidget(settingsStore.getState().widgetsPath, id)
      if (!widget) return

      setWidget(widget)
      setSrcDoc(await createWidgetEditorSrcDoc(widget))
    })()
  }, [id, instanceId])

  return (
    <div className="h-full flex flex-col gap-4 p-2">
      {widget && (
        <>
          <div className="flex gap-2">
            <img
              src={convertFileSrc(widget.previewPath)}
              className="size-40 object-contain border border-separator rounded-lg"
            />

            <div className="flex flex-col gap-2">
              <div className="text-lg">{widget.name}</div>
              <div className="text-sm">{widget.version}</div>
              <div className="text-muted text-sm">{widget.description}</div>
            </div>
          </div>

          <Separator />
        </>
      )}

      <div className="flex items-center">
        <div className="text-muted text-xs tracking-wide">
          {instanceId ? `UPDATE INSTANCE (${instanceId})` : 'NEW INSTANCE'}
        </div>

        <Button className="ml-auto" onPress={onAction}>
          {instanceId ? (
            <>
              <PiArrowClockwise className="text-lg" /> Update
            </>
          ) : (
            <>
              <PiPlus className="text-lg" /> Create
            </>
          )}
        </Button>
      </div>
      <Separator />

      {srcDoc && (
        <iframe ref={editorRef} srcDoc={srcDoc} sandbox="allow-scripts" className="size-full flex rounded-lg" />
      )}
    </div>
  )
}

export async function createWidgetEditorSrcDoc(widget: Widget, data?: unknown) {
  const html = await readTextFile(`${widget.path}/editor/index.html`)
  const css = await readTextFile(`${widget.path}/editor/styles.css`)
  const script = await readTextFile(`${widget.path}/editor/script.js`)

  // we need 2 things
  // - send the form data back to the host
  // - send existing data to the form

  return `
    <!DOCTYPE html>
    <html>
      <head>
        <meta charset="utf-8" />
        <meta name="color-scheme" content="light dark">
        <style>${css}</style>
      </head>
      <body>
        ${html}        
        ${data ? `<script>window.widgetInstanceData = ${JSON.stringify(data)}</script>` : ''}
        <script>${preludeScript}</script>
        <script>${script}</script>
      </body>
    </html>
  `
}
