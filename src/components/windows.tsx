import { Link, Outlet } from 'react-router'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { Button, Separator } from '@heroui/react'
import { buttonVariants } from '@heroui/styles'
import { PiGear, PiX } from 'react-icons/pi'

export function Window() {
  const currentWindow = getCurrentWindow()

  return (
    <div className="flex flex-col h-full overflow-auto p-2 gap-2 bg-background">
      <div
        data-tauri-drag-region
        className="shrink-0 flex items-center gap-2 w-full bg-surface rounded-lg pointer-events-auto p-2">
        <Link to="/" className="leading-none tracking-wide font-medium mx-2">
          Wendy
        </Link>

        <Separator orientation="vertical" />

        <Link to="/settings" className={buttonVariants({ size: 'sm', variant: 'ghost' })}>
          <PiGear className="text-lg text-muted" /> Settings
        </Link>

        <div className="flex gap-2 items-center ml-auto">
          <Button isIconOnly size="sm" variant="danger-soft" onPress={() => currentWindow.close()}>
            <PiX />
          </Button>
        </div>
      </div>

      <Outlet />
    </div>
  )
}
