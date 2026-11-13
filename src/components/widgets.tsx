import { useEffect, useState } from 'react'
import { Link } from 'react-router'
import { convertFileSrc } from '@tauri-apps/api/core'
import { useStore } from 'zustand'
import MiniSearch from 'minisearch'
import { useDebounce } from 'use-debounce'
import { SearchField, Switch } from '@heroui/react'
import { PiImageFill, PiMagnifyingGlass } from 'react-icons/pi'
import { getWidgets } from '@/commands'
import { globalStore } from '@/utils'
import type { Widget } from '@/types'

const searchIndex = new MiniSearch({
  idField: 'id',
  fields: ['name', 'description', 'permissions'],
  storeFields: ['id'],
  searchOptions: { boost: { name: 2 }, prefix: true, fuzzy: true },
})

export function Widgets() {
  const store = useStore(globalStore)
  const [widgets, setWidgets] = useState<Widget[]>([])

  const map = new Map(widgets.map(it => [it.id, it]))
  const [filtered, setFiltered] = useState(widgets)

  const [searchQuery, setSearchQuery] = useState('')
  const [debouncedSearchQuery] = useDebounce(searchQuery, 500)
  const [showBackgroundsOnly, setShowBackgroundOnly] = useState(false)

  useEffect(() => {
    if (!debouncedSearchQuery.trim()) {
      if (showBackgroundsOnly) return setFiltered(widgets.filter(it => it.isBackground))
      return setFiltered(widgets)
    }

    const data: Widget[] = []

    for (const { id } of searchIndex.search(debouncedSearchQuery)) {
      const item = map.get(id)
      if (!item || (showBackgroundsOnly && !item.isBackground)) continue

      data.push(item)
    }

    setFiltered(data)
  }, [widgets, debouncedSearchQuery, showBackgroundsOnly])

  useEffect(() => {
    searchIndex.removeAll()
    searchIndex.addAll(widgets)
  }, [widgets])

  useEffect(() => {
    ;(async () => {
      const data = await getWidgets(store.widgetsPath)
      setWidgets(data)
    })()
  }, [store.widgetsPath, store.refetchCounter])

  return (
    <div className="flex flex-col h-full overflow-auto">
      <div className="flex items-center gap-4 w-full shrink-0 sticky top-0 z-10 bg-background/90 backdrop-blur-lg rounded-lg p-2">
        <SearchField className="w-1/4 min-w-100 ml-auto" value={searchQuery} onChange={setSearchQuery}>
          <SearchField.Group className="rounded-full">
            <SearchField.SearchIcon className="text-lg">
              <PiMagnifyingGlass />
            </SearchField.SearchIcon>
            <SearchField.Input />
            <SearchField.ClearButton />
          </SearchField.Group>
        </SearchField>

        <Switch isSelected={showBackgroundsOnly} onChange={setShowBackgroundOnly}>
          <Switch.Content>
            <Switch.Control>
              <Switch.Thumb>
                <Switch.Icon>
                  <PiImageFill />
                </Switch.Icon>
              </Switch.Thumb>
            </Switch.Control>
            Backgrounds only?
          </Switch.Content>
        </Switch>
      </div>

      <div className="grid grid-cols-4 gap-8 h-fit py-4">
        {filtered.map(it => {
          return (
            <Link key={it.id} to={`/editor/${it.id}`} className="flex flex-col gap-2 items-center">
              <img
                src={convertFileSrc(it.previewPath)}
                className="h-60 shrink-0 object-contain data-[placeholder=true]:object-cover border border-separator rounded-lg"
                onError={evt => {
                  evt.currentTarget.src = '/placeholder.png'
                  evt.currentTarget.setAttribute('data-placeholder', 'true')
                }}
              />

              <div className="flex items-center gap-2">
                {it.isBackground && <PiImageFill className="text-success" />} {it.name}
              </div>
              <div className="text-xs text-muted">{it.description}</div>
            </Link>
          )
        })}
      </div>
    </div>
  )
}
