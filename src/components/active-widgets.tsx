import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router'
import { convertFileSrc } from '@tauri-apps/api/core'
import { getAllWindows } from '@tauri-apps/api/window'
import { useStore } from 'zustand'
import { Button } from '@heroui/react'
import { PiImageFill, PiNotePencil, PiTrash } from 'react-icons/pi'
import { activeWidgetsStore, getWidgetKey, globalStore } from '@/utils'
import type { ActiveWidget } from '@/types'

export function ActiveWidgets() {
  const navigate = useNavigate()
  const store = useStore(globalStore)
  const [activeWidgets, setActiveWidgets] = useState<ActiveWidget[]>([])

  const onRemove = async (widget: ActiveWidget) => {
    const key = getWidgetKey(widget.meta.id, widget.instanceId)

    await activeWidgetsStore.delete(key)
    await activeWidgetsStore.save()
    setActiveWidgets(prev => prev.filter(it => getWidgetKey(it.meta.id, it.instanceId) !== key))

    for (const window of await getAllWindows()) {
      if (window.label === key) await window.close()
    }
  }

  useEffect(() => {
    ;(async () => {
      if (store.refetchCounter > 0) await activeWidgetsStore.reload()
      setActiveWidgets(await activeWidgetsStore.values<ActiveWidget>())
    })()
  }, [store.refetchCounter])

  return (
    <div className="h-full flex flex-col px-2 overflow-auto divide-y divide-surface">
      {activeWidgets.map(it => {
        return (
          <div key={it.instanceId} className="flex gap-2 py-2">
            <img
              src={convertFileSrc(it.meta.previewPath)}
              className="size-40 shrink-0 object-contain data-[placeholder=true]:object-cover border border-separator rounded-lg"
              onError={evt => {
                evt.currentTarget.src = '/placeholder.png'
                evt.currentTarget.setAttribute('data-placeholder', 'true')
              }}
            />

            <div className="flex flex-col gap-2">
              <div className="text-lg flex items-center gap-2">
                {it.meta.isBackground && <PiImageFill className="text-success" />} {it.meta.name}
              </div>

              <div className="text-sm">Version: {it.meta.version}</div>
              <div className="text-muted text-xs tracking-wide">INSTANCE: {it.instanceId}</div>
            </div>

            <div className="flex gap-2 ml-auto mt-auto">
              <Button
                size="sm"
                variant="outline"
                onPress={() => navigate(`/editor/${getWidgetKey(it.meta.id, it.instanceId)}`)}>
                <PiNotePencil className="text-lg" /> Edit
              </Button>

              <Button size="sm" variant="danger-soft" onPress={() => onRemove(it)}>
                <PiTrash className="text-lg" /> Remove
              </Button>
            </div>
          </div>
        )
      })}
    </div>
  )
}
