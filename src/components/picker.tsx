import { useEffect, useState } from 'react'
import { Link } from 'react-router'
import { convertFileSrc } from '@tauri-apps/api/core'
import { getWidgets } from '@/commands'
import { store as settingsStore } from '@/components/settings'
import type { Widget } from '@/types'

export function Picker() {
  const [widgets, setWidgets] = useState<Widget[]>([])

  useEffect(() => {
    ;(async () => {
      const data = await getWidgets(settingsStore.getState().widgetsPath)
      setWidgets(data)
    })()
  }, [])

  return (
    <div className="p-2 flex h-full overflow-auto">
      <div className="grid grid-cols-4">
        {widgets.map(it => {
          return (
            <Link key={it.id} to={`/editor/${it.id}`} className="flex flex-col gap-2 items-center">
              <img src={convertFileSrc(it.previewPath)} className="h-60" />
              <div>{it.name}</div>
              <div className="text-xs text-muted">{it.description}</div>
            </Link>
          )
        })}
      </div>
    </div>
  )
}
