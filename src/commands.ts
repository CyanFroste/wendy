import { invoke } from '@tauri-apps/api/core'
import type { Nullable, ScopedDir, Widget } from '@/types'

export async function getWidgets(widgetsPath: string, refetch = false) {
  return await invoke<Widget[]>('get_widgets', { widgetsPath, refetch })
}

export async function getWidget(widgetsPath: string, id: string) {
  return await invoke<Nullable<Widget>>('get_widget', { widgetsPath, id })
}

export async function invokeWidgetRpc(widgetId: string, method: string, params: unknown) {
  return await invoke<unknown>('widget_rpc', { widgetId, method, params })
}

export async function getWidgetAsset(widgetId: string, path: string) {
  return await invoke<ArrayBuffer>('get_widget_asset', { widgetId, path })
}

export async function startMediaListener(windowLabel: string) {
  await invoke('start_media_listener', { windowLabel })
}

export async function startAudioCapture(windowLabel: string) {
  await invoke('start_audio_capture', { windowLabel })
}

export async function getFsScopeList() {
  return await invoke<ScopedDir[]>('get_fs_scope_list')
}

export async function addDirToFsScope(path: string, recursive = true) {
  await invoke('add_dir_to_fs_scope', { path, recursive })
}

export async function removeDirFromFsScope(path: string, recursive = true) {
  await invoke('remove_dir_from_fs_scope', { path, recursive })
}

export async function init() {
  await invoke('init')
}
