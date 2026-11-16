import { Fragment, useEffect, useRef, useState } from 'react'
import { useParams } from 'react-router'
import { readTextFile } from '@tauri-apps/plugin-fs'
import { convertFileSrc } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { getAllWindows, currentMonitor as getCurrentMonitor } from '@tauri-apps/api/window'
import { Button, Chip, Code, Link, Modal, Separator, toast } from '@heroui/react'
import { PiArrowClockwise, PiImageFill, PiPlus } from 'react-icons/pi'
import { invokeWidgetRpc, getWidget, readFile } from '@/commands'
import {
  constants,
  activeWidgetsStore,
  createWidgetWindow,
  extractErrorMessage,
  getWidgetKey,
  globalStore,
  htmlSafeJSON,
} from '@/utils'
import preludeScript from '@/scripts/widget-editor-prelude?raw'
import commonPreludeCss from '@/scripts/common-prelude.css?raw'
import type { ActiveWidget, Nullable, RpcRequest, RpcResponse, Widget } from '@/types'

type PathParams = { id: string; instanceId?: string }

export function WidgetEditor() {
  const { id, instanceId } = useParams<PathParams>()
  const editorRef = useRef<HTMLIFrameElement>(null)

  const [widget, setWidget] = useState<Nullable<Widget>>(null)
  const [srcDoc, setSrcDoc] = useState('')
  const [isLoading, setIsLoading] = useState(false)

  const onUpdate = async (widget: Widget, instanceId: string, data?: Nullable<unknown>) => {
    const key = getWidgetKey(widget.id, instanceId)

    console.info(`updating widget: '${key}'`)

    const activeWidget = await activeWidgetsStore.get<ActiveWidget>(key)
    if (!activeWidget) throw new Error(`widget not found with key: '${key}'`)

    activeWidget.meta = widget
    activeWidget.data = data

    await activeWidgetsStore.set(key, activeWidget)
    await activeWidgetsStore.save()

    await (await getAllWindows()).find(w => w.label === key)?.close()
    createWidgetWindow(activeWidget)

    return key
  }

  const onCreate = async (widget: Widget, data?: Nullable<unknown>) => {
    const instanceId = crypto.randomUUID()
    const key = getWidgetKey(widget.id, instanceId)

    console.info(`creating widget: '${key}'`)

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

    return key
  }

  const onAction = () => {
    if (!editorRef.current || !widget) return

    const fn = async () => {
      try {
        setIsLoading(true)

        const data = await new Promise((resolve, reject) => {
          const editorWindow = editorRef.current?.contentWindow
          if (!editorWindow) return reject(new Error('no editor window'))

          const timeoutId = setTimeout(() => {
            cleanup()
            reject(new Error('widget editor response timed out'))
          }, 3000)

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

        if (instanceId) return await onUpdate(widget, instanceId, data)
        return await onCreate(widget, data)
      } catch (err) {
        console.error(err)
        // TODO?: try to create the widget with no data ?
        throw err
      } finally {
        setIsLoading(false)
      }
    }

    toast.promise(fn, {
      error: err => err.message,
      loading: instanceId ? 'Updating widget...' : 'Creating widget...',
      success: key => `Widget ${instanceId ? 'updated' : 'created'} (${key})`,
    })
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
          let transfer: Transferable[] = []

          switch (method) {
            case constants.RPC_SYSTEM_GET_FILE_URL:
              if (!widget.permissions.includes('system:read')) throw new Error('permission denied')
              res.payload = convertFileSrc(params.path as string)
              break

            case constants.RPC_SYSTEM_READ_FILE:
            case constants.RPC_UTILS_GET_ASSET_URL:
            case constants.RPC_UTILS_GET_ASSET: {
              const assetPath = `${widget.path}/assets/${params.path}`

              if (method === constants.RPC_UTILS_GET_ASSET_URL) {
                res.payload = convertFileSrc(assetPath)
                break
              }

              const data = new Uint8Array(
                method === constants.RPC_SYSTEM_READ_FILE
                  ? await invokeWidgetRpc<number[]>(widget.id, method, params)
                  : await readFile(assetPath),
              )

              res.payload = data
              transfer = [data.buffer]
              break
            }

            default:
              res.payload = await invokeWidgetRpc(widget.id, method, params)
              break
          }

          containerWindow.postMessage(res, '*', transfer)
        } catch (err) {
          console.error(err)

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
      try {
        const widget = await getWidget(globalStore.getState().widgetsPath, id)
        if (!widget) return

        setWidget(widget)
        setSrcDoc(await createSrcDoc(widget))
      } catch (err) {
        console.error(err)
      }
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
                    <Modal.Dialog className="p-0 rounded-lg max-w-200">
                      <div className="flex flex-col gap-4 p-4 overflow-auto">
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
                          <div>Author: {widget.author}</div>

                          {widget.source && (
                            <div>
                              Source:{' '}
                              <Link
                                onClick={evt => {
                                  evt.preventDefault()
                                  openUrl(widget.source)
                                }}>
                                {widget.source}
                                <Link.Icon />
                              </Link>
                            </div>
                          )}

                          <div>
                            ID: <Code>{widget.id}</Code>
                          </div>
                          <div>
                            Location: <Code>{widget.path}</Code>
                          </div>
                        </div>

                        {(
                          [
                            ['PERMISSIONS', widget.permissions],
                            ['TAGS', widget.tags],
                            ['ALLOWED DOMAINS', widget.allowedDomains],
                            ['ALLOWED STORES', widget.allowedStores],
                          ] as const
                        ).map(([category, values]) => {
                          if (!values.length) return null
                          return (
                            <Fragment key={category}>
                              <Separator />
                              <div className="flex flex-col gap-2">
                                <div className="text-muted text-xs tracking-wide">{category}</div>

                                <div className="flex flex-wrap gap-2">
                                  {values.map(it => (
                                    <Chip key={it} className="font-mono">
                                      {it}
                                    </Chip>
                                  ))}
                                </div>
                              </div>
                            </Fragment>
                          )
                        })}
                      </div>

                      <Separator />
                      <Modal.Footer className="p-4">
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

        <Button className="ml-auto" onPress={onAction} isPending={isLoading}>
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
  const csp = createCsp()

  return `
    <!DOCTYPE html>
    <html>
      <head>
        <meta charset="utf-8" />
        <meta http-equiv="Content-Security-Policy" content="${csp}" />
        <meta name="referrer" content="no-referrer" />
        <meta name="color-scheme" content="light dark" />
        <style>${commonPreludeCss}</style>
        <style>${css}</style>
      </head>
      <body>
        ${html}
        <script>
          window.widgetInstanceData = ${htmlSafeJSON(data)};
        </script>
        <script>${preludeScript}</script>
        <script>${script}</script>
      </body>
    </html>`
}

function createCsp(): string {
  const localSources = "'self' data: blob: asset: http://asset.localhost https://asset.localhost"

  return [
    "default-src 'none'",
    `script-src 'unsafe-inline' ${localSources}`,
    `style-src 'unsafe-inline' ${localSources}`,
    `img-src ${localSources}`,
    `media-src ${localSources}`,
    `font-src ${localSources}`,
    `connect-src ${localSources}`,
    "frame-src 'none'",
    "object-src 'none'",
    "base-uri 'none'",
    "form-action 'none'",
  ].join('; ')
}
