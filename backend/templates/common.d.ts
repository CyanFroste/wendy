export {}

declare global {
  interface Widget {
    id: string
    name: string
    path: string
    description: string
    version: string
    previewPath?: string | null
    isBackground?: boolean | null
    permissions: string[]
    allowedDomains: string[]
    allowedStores: string[]
    author: string
    source?: string | null
    tags: string[]
  }

  interface CpuInfo {
    brand: string
    globalUsage: number
    coreCount: number
    coreUsages: number[]
  }

  interface RamInfo {
    totalBytes: number
    usedBytes: number
    availableBytes: number
    swapTotalBytes: number
    swapUsedBytes: number
  }

  interface DiskInfo {
    name: string
    mountPoint: string
    fileSystem: string
    totalBytes: number
    availableBytes: number
    isRemovable: boolean
  }

  interface MediaTimeline {
    startMs: number
    endMs: number
    positionMs: number
    canSeek: boolean
  }

  interface MediaControls {
    canPlay: boolean
    canPause: boolean
    canToggle: boolean
    canNext: boolean
    canPrev: boolean
  }

  interface MediaSessionInfo {
    playerId: string
    playerName: string
    title: string
    artist: string
    album: string
    isPlaying: boolean
    isCurrent: boolean
    thumbnail: Uint8Array
    controls: MediaControls
    timeline?: MediaTimeline | null
  }

  type MediaCommand =
    | { type: 'play' }
    | { type: 'pause' }
    | { type: 'toggle' }
    | { type: 'next' }
    | { type: 'prev' }
    | { type: 'seek'; positionMs: number }

  interface HttpResponse {
    status: number
    ok: boolean
    url: string
    headers: Record<string, string>
    body: string
  }

  interface HttpOptions {
    method?: String | null
    headers?: Record<string, string> | null
    body?: string | null
    timeout_ms?: number | null
  }

  interface Volume {
    level: number
    muted: boolean
  }

  interface InstalledApp {
    name: string
    path: string
    iconBase64?: string | null
  }

  interface FileEntry {
    path: string
    isDir: boolean
  }

  /**
   * @see https://github.com/sindresorhus/pretty-bytes#options
   */
  interface FormatBytesOptions {
    readonly signed?: boolean
    readonly locale?: boolean | string | readonly string[]
    readonly bits?: boolean
    readonly binary?: boolean
    readonly minimumFractionDigits?: number
    readonly maximumFractionDigits?: number
    readonly space?: boolean
    readonly nonBreakingSpace?: boolean
    readonly fixedWidth?: number
  }
}
