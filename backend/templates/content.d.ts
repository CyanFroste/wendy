export {}

declare global {
  interface Window {
    widgetMeta: Widget
    widgetInstanceData?: unknown | null

    widgetApi: {
      network: {
        request: (url: string, options?: HttpOptions) => Promise<HttpResponse>
      }
      audio: {
        getMasterVolume: () => Promise<Volume>
        setMasterVolume: (level: number) => Promise<Volume>
        toggleMasterMute: () => Promise<Volume>

        onAudioCaptured: (callback: (data: Float32Array) => void) => void
      }
      media: {
        getActiveMedia: () => Promise<MediaSessionInfo | null>
        sendCommand: (playerId: string, command: MediaCommand) => Promise<void>

        onMediaUpdated: (callback: (data: MediaSessionInfo) => void) => void
      }
      system: {
        readDir: (path: string) => Promise<FileEntry[]>
        readFile: (path: string) => Promise<Uint8Array>
        getFileUrl: (path: string) => Promise<string>
        getInstalledApps: () => Promise<InstalledApp[]>
        getCpuInfo: () => Promise<CpuInfo>
        getRamInfo: () => Promise<RamInfo>
        getDiskInfo: () => Promise<DiskInfo[]>
        openInstalledApp: (path: string) => Promise<void>
        openRecycleBin: () => Promise<void>
        getRecycleBinCount: () => Promise<number>
      }
      store: {
        getStore: <T = unknown>(
          name: string,
        ) => {
          entries: () => Promise<[string, T][]>
          keys: () => Promise<string[]>
          values: () => Promise<T[]>
          get: (key: string) => Promise<T | null>
          set: (key: string, value: T) => Promise<void>
          remove: (key: string) => Promise<T | null>
          clear: () => Promise<void>
        }
      }
      utils: {
        getAssetUrl: (path: string) => Promise<string>
        getAsset: (path: string) => Promise<Uint8Array>
        openExternalUrl: (url: string) => Promise<void>
        formatBytes: (value: number, options: FormatBytesOptions) => Promise<string>
      }
    }
  }
}
