import { getVersion } from '@tauri-apps/api/app'
import { openUrl } from '@tauri-apps/plugin-opener'
import { Button, Separator } from '@heroui/react'
import { PiGithubLogo } from 'react-icons/pi'

const appVersion = await getVersion()

export function Help() {
  return (
    <div className="flex flex-col h-full overflow-auto p-2">
      <div className="flex flex-col p-12 items-center gap-2">
        <img src="/logo.png" className="size-48 shrink-0 mb-4" />
        <div className="text-3xl leading-none my-2">Wendy Widget Framework</div>
        <div className="text-muted tracking-wide">Version {appVersion}</div>
        <div className="text-xs text-muted mb-8">Copyright © {new Date().getFullYear()} Cyan Froste</div>

        <Button variant="tertiary" onPress={() => openUrl('https://github.com/CyanFroste/wendy')}>
          <PiGithubLogo className="text-lg" /> View source code
        </Button>
      </div>

      <Separator />
    </div>
  )
}
