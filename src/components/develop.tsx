import { useState } from 'react'
import { kebabCase } from 'lodash'
import { useStore } from 'zustand'
import { Button, Checkbox, CheckboxGroup, Code, Input, Label, Separator, Switch, TextArea, toast } from '@heroui/react'
import { PiImageFill, PiPlus } from 'react-icons/pi'
import { createNewProject } from '@/commands'
import { constants, globalStore } from '@/utils'
import type { Widget } from '@/types'

const PERMISSION_OPTIONS = [
  constants.PERM_SYSTEM_READ,
  constants.PERM_SYSTEM_EXEC,
  constants.PERM_STORE_READ,
  constants.PERM_STORE_WRITE,
  constants.PERM_MEDIA_READ,
  constants.PERM_MEDIA_EXEC,
  constants.PERM_AUDIO_READ,
  constants.PERM_AUDIO_EXEC,
  constants.PERM_NETWORK_EXEC,
].reduce<Map<string, string[]>>((acc, it) => {
  const [ns, perm] = it.split(':')
  const ex = acc.get(ns) ?? []

  ex.push(perm)
  acc.set(ns, ex)

  return acc
}, new Map())

export function Develop() {
  const store = useStore(globalStore)

  const [id, setId] = useState('')
  const [name, setName] = useState('')
  const [author, setAuthor] = useState('')
  const [isBackground, setIsBackground] = useState(false)
  const [permissions, setPermissions] = useState<Map<string, string[]>>(new Map())
  const [allowedDomains, setAllowedDomains] = useState<string[]>([])
  const [allowedStores, setAllowedStores] = useState<string[]>([])
  const [isLoading, setIsLoading] = useState(false)

  const isCreateDisabled = !id.trim() || !name.trim() || !author.trim()

  const onCreate = () => {
    const fn = async () => {
      try {
        setIsLoading(true)

        const widget: Widget = {
          id: id.trim(),
          name: name.trim(),
          author: author.trim(),
          isBackground,
          version: '1.0.0',
          allowedDomains: permissions.has('network') ? allowedDomains.map(it => it.trim()) : [],
          allowedStores: permissions.has('store') ? allowedStores.map(it => it.trim()) : [],
          permissions: permissions
            .entries()
            .reduce<string[]>((acc, [ns, perms]) => acc.concat(perms.map(p => `${ns}:${p}`)), []),
          path: '',
          description: '',
          previewPath: '',
          source: '',
          tags: [],
        }

        const data = await createNewProject(store.widgetsPath, widget)

        setAuthor('')
        setName('')
        setId('')

        return data
      } catch (err) {
        console.error(err)
        throw err
      } finally {
        setIsLoading(false)
      }
    }

    toast.promise(fn, {
      error: err => err.message,
      loading: 'Creating new project...',
      success: ({ path }) => `Project created at ${path}`,
    })
  }

  const onPermissionChange = (ns: string) => (value: string[]) => {
    const newPermissions = new Map(permissions)

    if (value.length) newPermissions.set(ns, value)
    else newPermissions.delete(ns)

    setPermissions(newPermissions)
  }

  return (
    <div className="flex flex-col gap-4 p-2">
      <div className="flex items-center w-full">
        <div className="text-2xl leading-none my-2">Develop</div>

        <Button className="ml-auto" onPress={onCreate} isDisabled={isCreateDisabled} isPending={isLoading}>
          <PiPlus className="text-lg" /> Create new project
        </Button>
      </div>

      <Separator />

      <div className="flex flex-col gap-2 w-1/2 min-w-200">
        <Label className="text-muted text-xs tracking-wide">NAME</Label>

        <Input
          value={name}
          placeholder="Name of the project"
          onChange={evt => {
            const value = evt.target.value

            setName(value)
            setId(kebabCase(value))
          }}
        />
      </div>

      {/* TODO - check registry for dupes if hosting */}
      <div className="flex flex-col gap-2 w-1/2 min-w-200">
        <Label className="text-muted text-xs tracking-wide">ID</Label>

        <Input
          className="font-mono"
          value={id}
          onChange={evt => setId(evt.target.value)}
          placeholder="ID of the project"
        />
      </div>

      <div className="flex flex-col gap-2 w-1/2 min-w-200">
        <Label className="text-muted text-xs tracking-wide">AUTHOR</Label>

        <Input value={author} placeholder="Name of the project author" onChange={evt => setAuthor(evt.target.value)} />
      </div>

      <Switch isSelected={isBackground} onChange={setIsBackground} className="my-2">
        <Switch.Content>
          <Switch.Control>
            <Switch.Thumb>
              <Switch.Icon>
                <PiImageFill />
              </Switch.Icon>
            </Switch.Thumb>
          </Switch.Control>
          Background?
        </Switch.Content>
      </Switch>

      <div className="flex flex-col gap-2 w-1/2 min-w-200">
        <Label className="text-muted text-xs tracking-wide">PERMISSIONS</Label>

        <div
          className="grid divide-x divide-separator border border-separator rounded-lg"
          style={{ gridTemplateColumns: `repeat(${PERMISSION_OPTIONS.size}, minmax(0, 1fr))` }}>
          {PERMISSION_OPTIONS.entries()
            .map(([ns, perms]) => (
              <CheckboxGroup
                key={ns}
                className="p-4"
                value={permissions.get(ns) ?? []}
                onChange={onPermissionChange(ns)}>
                <Label className="uppercase text-sm">{ns}</Label>

                {perms.map(perm => (
                  <Checkbox key={perm} value={perm}>
                    <Checkbox.Content>
                      <Checkbox.Control>
                        <Checkbox.Indicator />
                      </Checkbox.Control>
                      {perm}
                    </Checkbox.Content>
                  </Checkbox>
                ))}
              </CheckboxGroup>
            ))
            .toArray()}
        </div>
      </div>

      {!!permissions.get('network')?.length && (
        <div className="flex flex-col gap-2 w-1/2 min-w-200">
          <Label className="text-muted text-xs tracking-wide">ALLOWED DOMAINS</Label>

          <TextArea
            rows={4}
            className="font-mono"
            value={allowedDomains.join('\n')}
            onChange={evt => setAllowedDomains(evt.target.value.split('\n'))}
            placeholder="List of allowed domains (separated by line breaks) eg: http://*.example.com"
          />
        </div>
      )}

      {!!permissions.get('store')?.length && (
        <div className="flex flex-col gap-2 w-1/2 min-w-200">
          <Label className="text-muted text-xs tracking-wide">ALLOWED STORES</Label>

          <TextArea
            rows={4}
            className="font-mono"
            value={allowedStores.join('\n')}
            onChange={evt => setAllowedStores(evt.target.value.split('\n'))}
            placeholder="List of store names the widget can access (separated by line breaks)"
          />
        </div>
      )}

      <div className="text-muted italic mt-8">
        You can edit the <Code className="text-sm">manifest.json</Code> file inside the project folder for more control
      </div>
    </div>
  )
}
