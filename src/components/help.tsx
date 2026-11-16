import { getVersion } from '@tauri-apps/api/app'
import { openUrl } from '@tauri-apps/plugin-opener'
import { Button, Code, Link, Separator } from '@heroui/react'
import { PiGithubLogo } from 'react-icons/pi'

const appVersion = await getVersion()

export function Help() {
  return (
    <div className="flex flex-col h-full overflow-auto p-2 gap-4">
      <div className="flex flex-col p-20 items-center gap-2">
        <img src="/logo.png" className="size-48 shrink-0 mb-4" />
        <div className="text-3xl leading-none my-2">Wendy Widget Framework</div>
        <div className="text-muted tracking-wide">Version {appVersion}</div>
        <div className="text-xs text-muted mb-8">Copyright © {new Date().getFullYear()} Cyan Froste</div>

        <Button variant="tertiary" onPress={() => openUrl('https://github.com/CyanFroste/wendy')}>
          <PiGithubLogo className="text-lg" /> View source code
        </Button>
      </div>

      <Separator />

      <div className="flex flex-col gap-2">
        <div className="text-muted text-xs tracking-wide mb-2">CONTROLS AND SHORTCUTS</div>

        <div className="flex items-center">
          <div className="size-2 mx-4 rounded-full bg-warning" />
          <div>
            Press <Code>Ctrl + t</Code> on any widget window (except the "background" widgets) to show its Transform
            Controls
          </div>
        </div>

        <div className="flex items-center">
          <div className="size-2 mx-4 rounded-full bg-warning" />
          <div>
            Download and install the starter widget pack directly from inside the app, which fetches the latest release
            from{' '}
            <Link
              onClick={evt => {
                evt.preventDefault()
                openUrl('https://github.com/CyanFroste/widgets')
              }}>
              CyanFroste/widgets
              <Link.Icon />
            </Link>{' '}
            into your configured Widgets Folder
          </div>
        </div>

        <div className="flex items-center">
          <div className="size-2 mx-4 rounded-full bg-warning" />
          <div>
            Make sure to allow the folders under "Allowed Folders" in settings for certain widgets that may want to read
            files from them
          </div>
        </div>
      </div>
    </div>
  )
}
