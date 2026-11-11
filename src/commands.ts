import { invoke } from '@tauri-apps/api/core'
import type { Nullable, Widget } from '@/types'

export async function addPathToFsScope(path: string, recursive = true) {
  await invoke('add_path_to_fs_scope', { path, recursive })
}

export async function getWidgets(widgetsPath: string, refetch = false) {
  return await invoke<Widget[]>('get_widgets', { widgetsPath, refetch })
}

export async function getWidget(widgetsPath: string, id: string) {
  return await invoke<Nullable<Widget>>('get_widget', { widgetsPath, id })
}

export async function invokeWidgetRpc(method: string, params: unknown) {
  return await invoke<unknown>('widget_rpc', { method, params })
}

export async function getWidgetAsset(widgetId: string, path: string) {
  return await invoke<unknown>('get_widget_asset', { widgetId, path })
}
