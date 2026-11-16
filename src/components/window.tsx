import { useEffect, useState } from 'react'
import { Link as RouterLink, Outlet, useLocation, useNavigate } from 'react-router'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { Button, Chip, Separator, Toast } from '@heroui/react'
import { buttonVariants } from '@heroui/styles'
import { PiArrowClockwise, PiCheckCircle, PiCode, PiGear, PiQuestion, PiX } from 'react-icons/pi'
import { BiSolidWidget } from 'react-icons/bi'
import { useStore } from 'zustand'
import { activeWidgetsStore, globalStore } from '@/utils'
import { getWidgets } from '@/commands'

export function Window() {
  const navigate = useNavigate()
  const location = useLocation()
  const currentWindow = getCurrentWindow()
  const store = useStore(globalStore)
  const [activeWidgetsCount, setActiveWidgetsCount] = useState(0)

  const onActiveWidgetsChange = async () => {
    setActiveWidgetsCount(await activeWidgetsStore.length())
  }

  const refetch = async () => {
    try {
      await activeWidgetsStore.reload()
      await onActiveWidgetsChange()
      await getWidgets(store.widgetsPath, true)

      globalStore.setState(prev => ({ refetchCounter: prev.refetchCounter + 1 }))
    } catch (err) {
      console.error(err)
    }
  }

  useEffect(() => {
    const redirectListener = listen<string>('redirect', async evt => {
      navigate(evt.payload)
      await currentWindow.setFocus()
    })

    const activeWidgetsStoreListener = activeWidgetsStore.onChange(onActiveWidgetsChange)
    onActiveWidgetsChange()

    return () => {
      redirectListener.then(fn => fn())
      activeWidgetsStoreListener.then(fn => fn())
    }
  }, [])

  const getLinkClasses = (path: string) =>
    buttonVariants({ size: 'sm', variant: location.pathname === path ? 'tertiary' : 'ghost' })

  return (
    <>
      <div className="flex flex-col h-full overflow-auto p-2 gap-2 bg-background">
        <div
          data-tauri-drag-region
          className="shrink-0 flex items-center gap-2 w-full bg-surface rounded-lg pointer-events-auto p-2">
          <RouterLink to="/" className="flex items-center gap-2 px-1">
            <img src="/logo.png" className="size-6" />
            {/* <div className="leading-none tracking-wide font-medium">Wendy Widget Framework</div> */}
          </RouterLink>
          <Separator orientation="vertical" />

          <RouterLink to="/" className={getLinkClasses('/')}>
            <BiSolidWidget className="text-lg text-muted" /> Widgets
          </RouterLink>

          <RouterLink to="/active" className={getLinkClasses('/active')}>
            <PiCheckCircle className="text-lg text-muted" />
            Active
            {activeWidgetsCount > 0 && (
              <Chip size="sm" color="success" variant="primary">
                <Chip.Label>{activeWidgetsCount}</Chip.Label>
              </Chip>
            )}
          </RouterLink>

          <RouterLink to="/settings" className={getLinkClasses('/settings')}>
            <PiGear className="text-lg text-muted" /> Settings
          </RouterLink>

          <RouterLink to="/develop" className={getLinkClasses('/develop')}>
            <PiCode className="text-lg text-muted" /> Develop
          </RouterLink>

          <RouterLink to="/help" className={getLinkClasses('/help')}>
            <PiQuestion className="text-lg text-muted" /> Help
          </RouterLink>

          <div className="flex gap-2 items-center ml-auto">
            <Button isIconOnly size="sm" variant="tertiary" onPress={refetch}>
              <PiArrowClockwise className="text-lg" />
            </Button>

            <Button isIconOnly size="sm" variant="danger-soft" onPress={() => currentWindow.hide()}>
              <PiX className="text-lg" />
            </Button>
          </div>
        </div>

        <Outlet />
      </div>

      <Toast.Provider />
    </>
  )
}
