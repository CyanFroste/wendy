export {}

declare global {
  interface Window {
    widgetInstanceData?: unknown | null

    widgetEditorApi: {
      system: {
        readDir: (path: string) => Promise<FileEntry[]>
        readFile: (path: string) => Promise<Uint8Array>
        getFileUrl: (path: string) => Promise<string>
        getInstalledApps: () => Promise<InstalledApp[]>
      }
      utils: {
        getAssetUrl: (path: string) => Promise<string>
        getAsset: (path: string) => Promise<Uint8Array>
      }

      sendData: <T>(payload: T) => void
      onRequestData: (callback: () => void) => void
    }
  }
}
