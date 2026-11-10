export type Widget = {
  id: string
  name: string
  path: string
  description: string
  version: string
  previewPath: string
  permissions: string[]
}

export type ActiveWidget = {
  meta: Widget
  instanceId: string
  data: Nullable<unknown>
  isLocked: boolean
  width: number
  height: number
  x: number
  y: number
}

export type Nullable<T> = T | null | undefined
export type Dictionary<T = unknown> = Record<string, T>

declare module 'react' {
  interface CSSProperties {
    [key: `--${string}`]: string | number
  }
}
