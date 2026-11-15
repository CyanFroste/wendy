import { useEffect, useState } from 'react'
import { Button, Input, Label, Separator } from '@heroui/react'
import { open as openFilePicker } from '@tauri-apps/plugin-dialog'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import { PiCheck, PiFolder, PiMagnifyingGlass, PiPlus, PiX } from 'react-icons/pi'
import { useStore } from 'zustand'
import { addDirToFsScope, getFsScopeList, removeDirFromFsScope } from '@/commands'
import { globalStore } from '@/utils'
import type { ScopedDir } from '@/types'

export function Settings() {
  const store = useStore(globalStore)
  const [widgetsPath, setWidgetsPath] = useState('')
  const [fsScopeList, setFsScopeList] = useState<ScopedDir[]>([])
  const [allowedPath, setAllowedPath] = useState('')

  const isChanged = store.widgetsPath !== widgetsPath

  const onSave = async () => {
    await removeDirFromFsScope(store.widgetsPath)
    await addDirToFsScope(widgetsPath)

    globalStore.setState({ widgetsPath })
  }

  useEffect(() => {
    setWidgetsPath(store.widgetsPath)
    getFsScopeList().then(data => setFsScopeList(data.filter(it => it.path !== store.widgetsPath)))
  }, [])

  return (
    <div className="flex flex-col gap-4 p-2">
      <div className="flex items-center">
        <div className="text-2xl leading-none my-2">Settings</div>

        <Button className="ml-auto" isDisabled={!isChanged} onPress={onSave}>
          <PiCheck className="text-lg" /> Save changes
        </Button>
      </div>

      <Separator />

      <div className="flex items-end gap-4">
        <div className="flex flex-col gap-2 w-1/2 min-w-200">
          <Label className="text-muted text-xs tracking-wide">WIDGETS FOLDER</Label>

          <Input
            className="font-mono"
            value={widgetsPath}
            onChange={evt => setWidgetsPath(evt.target.value)}
            placeholder="The folder containing your widget files"
          />
        </div>

        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            onPress={async () => {
              const dir = await openFilePicker({ directory: true })
              if (dir) setWidgetsPath(dir)
            }}>
            <PiFolder className="text-lg text-muted" /> Select
          </Button>

          {widgetsPath && (
            <Button variant="outline" isDisabled={!widgetsPath} onPress={() => revealItemInDir(widgetsPath)}>
              <PiMagnifyingGlass className="text-lg text-muted" /> Show in Explorer
            </Button>
          )}
        </div>
      </div>

      <div className="flex flex-col gap-4">
        <div className="text-muted text-xs tracking-wide">ALLOWED FOLDERS</div>

        <div className="flex items-end gap-4">
          <Input
            className="font-mono w-1/2 min-w-200"
            value={allowedPath}
            onChange={evt => setAllowedPath(evt.target.value)}
            placeholder="Add folders that can be accessed by your widgets"
          />

          <div className="flex items-center gap-2">
            {allowedPath && (
              <Button
                onPress={async () => {
                  await addDirToFsScope(allowedPath)
                  // always recursive for now and not refetching the list again
                  setFsScopeList(prev => [...prev, { path: allowedPath, recursive: true }])
                  setAllowedPath('')
                }}>
                <PiPlus className="text-lg" /> Add
              </Button>
            )}

            <Button
              variant="outline"
              onPress={async () => {
                const dir = await openFilePicker({ directory: true })
                if (dir) setAllowedPath(dir)
              }}>
              <PiFolder className="text-lg text-muted" /> Select
            </Button>

            {widgetsPath && (
              <Button variant="outline" isDisabled={!allowedPath} onPress={() => revealItemInDir(allowedPath)}>
                <PiMagnifyingGlass className="text-lg text-muted" /> Show in Explorer
              </Button>
            )}
          </div>
        </div>

        {fsScopeList.length > 0 && (
          <div className="flex flex-col p-4 gap-2 border border-separate rounded-lg w-1/2 min-w-200">
            {fsScopeList.map(it => (
              <div key={it.path} className="flex items-center gap-4">
                <Button
                  size="sm"
                  isIconOnly
                  variant="ghost"
                  className="shrink-0"
                  onPress={async () => {
                    await removeDirFromFsScope(it.path)
                    setFsScopeList(prev => prev.filter(dir => dir.path !== it.path))
                  }}>
                  <PiX />
                </Button>

                <div className="font-mono text-sm text-muted">{it.path}</div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
