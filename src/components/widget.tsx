import { useEffect, useRef, useState } from 'react'
import { useParams } from 'react-router'
import { convertFileSrc } from '@tauri-apps/api/core'
import { readTextFile } from '@tauri-apps/plugin-fs'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { emitTo } from '@tauri-apps/api/event'
import { Button, ToggleButton } from '@heroui/react'
import { cn } from '@heroui/styles'
import formatBytes from 'pretty-bytes'
import { PiLock, PiNotePencil, PiTrash } from 'react-icons/pi'
import { getWidgetAsset, invokeWidgetRpc } from '@/commands'
import { activeWidgetsStore, getWidgetKey } from '@/utils'
import preludeScript from '@/scripts/widget-prelude?raw'
import commonPreludeCss from '@/scripts/common-prelude.css?raw'
import preludeCss from '@/scripts/widget-prelude.css?raw'
import type { Options as FormatBytesOptions } from 'pretty-bytes'
import type { ActiveWidget, Nullable, Widget, RpcRequest, RpcResponse } from '@/types'

type PathParams = { id: string; instanceId: string }

export function Widget() {
  const currentWindow = getCurrentWindow()
  const { id, instanceId } = useParams<PathParams>()
  const containerRef = useRef<HTMLIFrameElement>(null)

  const [widget, setWidget] = useState<Nullable<ActiveWidget>>(null)
  const [srcDoc, setSrcDoc] = useState('')

  const [showControls, setShowControls] = useState(false)

  const onToggleLock = async (isLocked: boolean) => {
    if (!widget) return
    const key = getWidgetKey(widget.meta.id, widget.instanceId)
    const updated = { ...widget, isLocked }

    await activeWidgetsStore.set(key, updated)
    await activeWidgetsStore.save()

    setWidget(updated)
  }

  const onRemove = async () => {
    if (!widget) return
    const key = getWidgetKey(widget.meta.id, widget.instanceId)

    await activeWidgetsStore.delete(key)
    await activeWidgetsStore.save()
    currentWindow.close()
  }

  const onEdit = () => {
    if (!widget) return
    const key = getWidgetKey(widget.meta.id, widget.instanceId)

    emitTo('main', 'redirect', `/editor/${key}`)
  }

  useEffect(() => {
    if (!id || !instanceId) return
    ;(async () => {
      const key = getWidgetKey(id, instanceId)
      const widget = await activeWidgetsStore.get<ActiveWidget>(key)
      if (!widget) return

      setWidget(widget)
      setSrcDoc(await createWidgetSrcDoc(widget))
    })()
  }, [id, instanceId])

  useEffect(() => {
    const containerWindow = containerRef.current?.contentWindow
    if (!containerWindow || !widget) return

    const handler = async (event: MessageEvent<RpcRequest>) => {
      if (event.source !== containerWindow) return
      const { type, id, method, params } = event.data

      if (type === 'RPC_REQUEST') {
        try {
          const res: RpcResponse = { type: 'RPC_RESPONSE', id, payload: null }

          switch (method) {
            case 'utils.get_asset_url':
              res.payload = convertFileSrc(`${widget.meta.path}/assets/${params.path}`)
              break
            case 'utils.get_asset':
              res.payload = await getWidgetAsset(widget.meta.id, params.path as string)
              break
            case 'utils.format_bytes':
              res.payload = formatBytes(params.value as number, params.options as FormatBytesOptions)
              break
            default:
              res.payload = await invokeWidgetRpc(widget.meta.id, method, params)
              break
          }

          containerWindow.postMessage(res, '*')
        } catch (err) {
          const payload = !err
            ? 'unknown error'
            : err instanceof Error
              ? err.message
              : typeof err === 'string'
                ? err
                : typeof err === 'object' && 'message' in err
                  ? err.message
                  : JSON.stringify(err)

          containerWindow.postMessage({ type: 'RPC_ERROR_RESPONSE', id, payload } satisfies RpcResponse, '*')
        }
      }
    }

    window.addEventListener('message', handler)
    return () => window.removeEventListener('message', handler)
  }, [containerRef, srcDoc, widget])

  useEffect(() => {
    const toggleControls = async (key: string, ctrl: boolean) => {
      if (ctrl && key === 't') setShowControls(prev => !prev)
    }

    const onKeyDown = async (evt: KeyboardEvent) => {
      const key = evt.key.toLowerCase()
      if (!evt.ctrlKey || key !== 't') return

      evt.preventDefault()
      await toggleControls(key, evt.ctrlKey)
    }

    const onMessage = async (evt: MessageEvent) => {
      if (evt.data?.type === 'FORWARDED_KEYDOWN') {
        const { key, ctrlKey, metaKey } = evt.data
        await toggleControls(key, ctrlKey || metaKey)
      }
    }

    window.addEventListener('keydown', onKeyDown)
    window.addEventListener('message', onMessage)

    return () => {
      window.removeEventListener('keydown', onKeyDown)
      window.removeEventListener('message', onMessage)
    }
  }, [widget])

  useEffect(() => {
    ;(async () => {
      if (widget?.isLocked) await currentWindow.setResizable(false)
      else await currentWindow.setResizable(showControls)
    })()
  }, [widget, showControls])

  return (
    <div className="size-full flex flex-col gap-4">
      <div
        className={cn(
          'flex flex-col fixed inset-0 z-100 transition-opacity duration-300 ease-in-out',
          showControls ? 'pointer-events-auto opacity-100' : 'pointer-events-none opacity-0',
        )}>
        <div className="shrink-0 absolute top-4 inset-x-4 z-10 rounded-lg flex bg-accent/25 backdrop-blur-lg">
          <div
            data-tauri-drag-region={!widget?.isLocked}
            className={cn('flex-1', showControls && !widget?.isLocked && 'pointer-events-auto cursor-move')}
          />

          <div className="flex items-center gap-2 p-2">
            <ToggleButton isIconOnly size="sm" variant="ghost" isSelected={widget?.isLocked} onChange={onToggleLock}>
              <PiLock className="text-lg" />
            </ToggleButton>

            <Button isIconOnly size="sm" variant="ghost" onPress={onEdit}>
              <PiNotePencil className="text-lg" />
            </Button>

            <Button isIconOnly size="sm" variant="danger-soft" onPress={onRemove}>
              <PiTrash className="text-lg" />
            </Button>
          </div>
        </div>

        <ResizeControls />
      </div>

      {srcDoc && <iframe ref={containerRef} srcDoc={srcDoc} sandbox="allow-scripts" className="size-full flex" />}
    </div>
  )
}

function ResizeControls() {
  const currentWindow = getCurrentWindow()

  const onMouseDown = async (evt: React.MouseEvent<HTMLDivElement>) => {
    const dir = evt.currentTarget.dataset.dir as Parameters<typeof currentWindow.startResizeDragging>[0]
    await currentWindow.startResizeDragging(dir)
  }

  const commonClasses = 'absolute size-8 border-accent bg-accent/10'

  return (
    <div className="absolute inset-0 size-full bg-accent/10">
      <div
        data-dir="North"
        onMouseDown={onMouseDown}
        className={cn(commonClasses, 'top-0 left-1/2 -translate-x-1/2 border-t-2 cursor-n-resize')}
      />
      <div
        data-dir="East"
        onMouseDown={onMouseDown}
        className={cn(commonClasses, 'top-1/2 right-0 -translate-y-1/2 border-r-2 cursor-e-resize')}
      />
      <div
        data-dir="South"
        onMouseDown={onMouseDown}
        className={cn(commonClasses, 'left-1/2 bottom-0 -translate-x-1/2 border-b-2 cursor-s-resize')}
      />
      <div
        data-dir="West"
        onMouseDown={onMouseDown}
        className={cn(commonClasses, 'left-0 top-1/2 -translate-y-1/2 border-l-2 cursor-w-resize')}
      />
      <div
        data-dir="NorthWest"
        onMouseDown={onMouseDown}
        className={cn(commonClasses, 'top-0 left-0 border-l-2 border-t-2 cursor-nw-resize rounded-tl-lg')}
      />
      <div
        data-dir="NorthEast"
        onMouseDown={onMouseDown}
        className={cn(commonClasses, 'top-0 right-0 border-r-2 border-t-2 cursor-ne-resize rounded-tr-lg')}
      />
      <div
        data-dir="SouthEast"
        onMouseDown={onMouseDown}
        className={cn(commonClasses, 'bottom-0 right-0 border-r-2 border-b-2 cursor-se-resize rounded-br-lg')}
      />
      <div
        data-dir="SouthWest"
        onMouseDown={onMouseDown}
        className={cn(commonClasses, 'bottom-0 left-0 border-l-2 border-b-2 cursor-sw-resize rounded-bl-lg')}
      />
    </div>
  )
}

export async function createWidgetSrcDoc(widget: ActiveWidget) {
  const html = await readTextFile(`${widget.meta.path}/content/index.html`)
  const css = await readTextFile(`${widget.meta.path}/content/styles.css`)
  const script = await readTextFile(`${widget.meta.path}/content/script.js`)

  return `
    <!DOCTYPE html>
    <html>
      <head>
        <meta charset="utf-8" />
        <meta name="color-scheme" content="light dark">        
        <style>${commonPreludeCss}</style>
        <style>${preludeCss}</style>
        <style>${css}</style>
      </head>
      <body>
        ${html}        
        <script>window.widgetInstanceData = ${widget.data ? JSON.stringify(widget.data) : 'null'}</script>
        <script>${preludeScript}</script>
        <script>${script}</script>
      </body>
    </html>
  `
}
