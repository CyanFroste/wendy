import { useEffect, useState } from 'react'
import { Button, Input, Label, Separator } from '@heroui/react'
import { open as openFilePicker } from '@tauri-apps/plugin-dialog'
import { createStore, useStore } from 'zustand'
import { persist } from 'zustand/middleware'
import { PiCheck, PiFolder } from 'react-icons/pi'
import { addPathToFsScope } from '@/commands'

export const store = createStore<Store>()(persist(initialState, { name: 'settings' }))

export function Settings() {
  const existing = useStore(store)
  const [widgetsPath, setWidgetsPath] = useState('')

  const isChanged = true // TODO - also save only changed values

  const onSave = async () => {
    // TODO? clear other scopes first?
    await addPathToFsScope(widgetsPath)

    store.setState({ widgetsPath })
  }

  useEffect(() => {
    setWidgetsPath(existing.widgetsPath)
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

      <div className="text-muted text-xs tracking-wide">WIDGETS</div>

      <div className="flex items-end gap-4">
        <div className="flex flex-col gap-2 w-1/2 min-w-200">
          <Label>Location</Label>
          <Input
            className="font-mono"
            value={widgetsPath}
            onChange={evt => setWidgetsPath(evt.target.value)}
            placeholder="Path to the folder containing widget files"
          />
        </div>

        <Button
          variant="outline"
          onPress={async () => {
            const dir = await openFilePicker({ directory: true })
            if (dir) setWidgetsPath(dir)
          }}>
          <PiFolder className="text-lg text-muted" /> Select
        </Button>
      </div>
    </div>
  )
}

type Store = {
  widgetsPath: string
}

// had to put this in a fn because false is not assignable to boolean? WTF?
function initialState(): Store {
  return {
    widgetsPath: '',
  }
}
