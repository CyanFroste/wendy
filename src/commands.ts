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
