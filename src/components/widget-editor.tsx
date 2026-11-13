import { useEffect, useRef, useState } from 'react'
import { useParams } from 'react-router'
import { readTextFile } from '@tauri-apps/plugin-fs'
import { convertFileSrc } from '@tauri-apps/api/core'
import { getAllWindows, currentMonitor as getCurrentMonitor } from '@tauri-apps/api/window'
import { Button, Chip, Code, Modal, Separator } from '@heroui/react'
import { PiArrowClockwise, PiImageFill, PiPlus } from 'react-icons/pi'
import { getWidget, getWidgetAsset, invokeWidgetRpc } from '@/commands'
import { activeWidgetsStore, createWidgetWindow, extractErrorMessage, getWidgetKey, globalStore } from '@/utils'
import preludeScript from '@/scripts/widget-editor-prelude?raw'
import commonPreludeCss from '@/scripts/common-prelude.css?raw'
import type { ActiveWidget, Nullable, RpcRequest, RpcResponse, Widget } from '@/types'

type PathParams = { id: string; instanceId?: string }

export function WidgetEditor() {
  const { id, instanceId } = useParams<PathParams>()
  const editorRef = useRef<HTMLIFrameElement>(null)

  const [widget, setWidget] = useState<Nullable<Widget>>(null)
  const [srcDoc, setSrcDoc] = useState('')

  const onUpdate = async (widget: Widget, instanceId: string, data?: Nullable<unknown>) => {
    const key = getWidgetKey(widget.id, instanceId)
    const activeWidget = await activeWidgetsStore.get<ActiveWidget>(key)
    if (!activeWidget) return

    activeWidget.meta = widget
    activeWidget.data = data

    await activeWidgetsStore.set(key, activeWidget)
    await activeWidgetsStore.save()

    await (await getAllWindows()).find(w => w.label === key)?.close()
    createWidgetWindow(activeWidget)
  }

  const onCreate = async (widget: Widget, data?: Nullable<unknown>) => {
    const instanceId = crypto.randomUUID()
    const key = getWidgetKey(widget.id, instanceId)
    const currentMonitor = await getCurrentMonitor()
    let [width, height] = [400, 400]

    if (widget.isBackground && currentMonitor) {
      const monitorSize = currentMonitor.size.toLogical(currentMonitor.scaleFactor)

      width = monitorSize.width
      height = monitorSize.height
    }

    const activeWidget: ActiveWidget = {
      instanceId,
      meta: widget,
      data,
      isLocked: false,
      isPinned: false,
      height,
      width,
      x: 0,
      y: 0,
    }

    await activeWidgetsStore.set(key, activeWidget)
    await activeWidgetsStore.save()

    createWidgetWindow(activeWidget)
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

    if (instanceId) onUpdate(widget, instanceId, data)
    else onCreate(widget, data)
  }

  useEffect(() => {
    const containerWindow = editorRef.current?.contentWindow
    if (!containerWindow || !widget) return

    const handler = async (event: MessageEvent<RpcRequest>) => {
      if (event.source !== containerWindow) return
      const { type, id, method, params } = event.data

      if (type === 'RPC_REQUEST') {
        try {
          const res: RpcResponse = { type: 'RPC_RESPONSE', id, payload: null }

          switch (method) {
            case 'utils.get_asset_url':
              res.payload = convertFileSrc(`${widget.path}/assets/${params.path}`)
              break
            case 'utils.get_asset':
              res.payload = await getWidgetAsset(widget.id, params.path as string)
              break
            default:
              res.payload = await invokeWidgetRpc(widget.id, method, params)
              break
          }

          containerWindow.postMessage(res, '*')
        } catch (err) {
          const payload = extractErrorMessage(err)
          containerWindow.postMessage({ type: 'RPC_ERROR_RESPONSE', id, payload } satisfies RpcResponse, '*')
        }
      }
    }

    window.addEventListener('message', handler)
    return () => window.removeEventListener('message', handler)
  }, [editorRef, srcDoc, widget])

  useEffect(() => {
    if (!id) return
    ;(async () => {
      const widget = await getWidget(globalStore.getState().widgetsPath, id)
      if (!widget) return

      setWidget(widget)
      setSrcDoc(await createSrcDoc(widget))
    })()
  }, [id, instanceId])

  return (
    <div className="h-full flex flex-col gap-4 p-2">
      {widget && (
        <>
          <div className="flex gap-2">
            <img
              src={convertFileSrc(widget.previewPath)}
              className="size-40 shrink-0 object-contain data-[placeholder=true]:object-cover border border-separator rounded-lg"
              onError={evt => {
                evt.currentTarget.src = '/placeholder.png'
                evt.currentTarget.setAttribute('data-placeholder', 'true')
              }}
            />

            <div className="flex flex-col gap-2">
              <div className="text-lg flex items-center gap-2">
                {widget.isBackground && <PiImageFill className="text-success" />} {widget.name}
              </div>

              <div className="text-sm">Version: {widget.version}</div>
              <div className="text-muted text-sm">{widget.description}</div>
            </div>

            <div className="flex ml-auto mt-auto">
              <Modal>
                <Button size="sm" variant="outline">
                  Show more details
                </Button>
                <Modal.Backdrop>
                  <Modal.Container>
                    <Modal.Dialog className="p-4 rounded-lg max-w-200">
                      <div className="flex flex-col gap-4">
                        <img
                          src={convertFileSrc(widget.previewPath)}
                          className="h-100 w-full shrink-0 object-contain data-[placeholder=true]:object-cover border border-separator rounded-lg"
                          onError={evt => {
                            evt.currentTarget.src = '/placeholder.png'
                            evt.currentTarget.setAttribute('data-placeholder', 'true')
                          }}
                        />

                        {/* TODO?: render long description as markdown */}

                        <Separator />
                        <div className="flex flex-col gap-2 text-sm">
                          {/* <div>Author: {widget.author}</div> */}
                          <div>
                            ID: <Code>{widget.id}</Code>
                          </div>
                          <div>
                            Location: <Code>{widget.path}</Code>
                          </div>
                        </div>

                        <Separator />
                        <div className="flex flex-col gap-2">
                          <div className="text-muted text-xs tracking-wide">PERMISSIONS</div>

                          <div className="flex flex-wrap gap-2">
                            {widget.permissions.map(it => (
                              <Chip key={it}>{it}</Chip>
                            ))}
                          </div>
                        </div>
                      </div>

                      <Separator className="my-4" />
                      <Modal.Footer>
                        <Button slot="close" variant="outline">
                          Close
                        </Button>
                      </Modal.Footer>
                    </Modal.Dialog>
                  </Modal.Container>
                </Modal.Backdrop>
              </Modal>
            </div>
          </div>

          <Separator />
        </>
      )}

      <div className="flex items-center">
        <div className="text-muted text-xs tracking-wide">
          {instanceId ? `UPDATE INSTANCE (${instanceId})` : 'CREATE NEW INSTANCE'}
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

async function createSrcDoc(widget: Widget, data?: unknown) {
  const html = await readTextFile(`${widget.path}/editor/index.html`)
  const css = await readTextFile(`${widget.path}/editor/styles.css`)
  const script = await readTextFile(`${widget.path}/editor/script.js`)

  return `
    <!DOCTYPE html>
    <html>
      <head>
        <meta charset="utf-8" />
        <meta name="color-scheme" content="light dark">
        <style>${commonPreludeCss}</style>
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
