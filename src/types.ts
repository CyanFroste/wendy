export type Widget = {
  id: string
  name: string
  path: string
  description: string
  version: string
  previewPath: string
  isBackground: boolean
  permissions: string[]
  allowedDomains: string[]
  author: string
  source: string
  tags: string[]
}

export type ActiveWidget = {
  meta: Widget
  instanceId: string
  data: Nullable<unknown>
  isLocked: boolean
  isPinned: boolean
  width: number
  height: number
  x: number
  y: number
}

export type MediaTimeline = {
  startMs: number
  endMs: number
  positionMs: number
  canSeek: boolean
}

export type MediaControls = {
  canPlay: boolean
  canPause: boolean
  canToggle: boolean
  canNext: boolean
  canPrev: boolean
}

export type MediaSessionInfo = {
  playerId: string
  playerName: string
  title: string
  artist: string
  album: string
  isPlaying: boolean
  isCurrent: boolean
  thumbnail: number[]
  controls: MediaControls
  timeline: Nullable<MediaTimeline>
}

export type MediaCommand =
  | { type: 'play' }
  | { type: 'pause' }
  | { type: 'toggle' }
  | { type: 'next' }
  | { type: 'prev' }
  | { type: 'seek'; positionMs: number }

export type RpcRequest = {
  type: 'RPC_REQUEST'
  id: string
  method: string
  params: Dictionary
}

export type RpcResponse = {
  type: 'RPC_RESPONSE' | 'RPC_ERROR_RESPONSE'
  id: string
  payload: unknown
}

export type Nullable<T> = T | null | undefined
export type Dictionary<T = unknown> = Record<string, T>

declare module 'react' {
  interface CSSProperties {
    [key: `--${string}`]: string | number
  }
}
