window.widgetApi = (() => {
  const pendingRequests = new Map()

  window.addEventListener('message', event => {
    const { type, id, payload } = event.data

    if (type === 'RPC_RESPONSE' || type === 'RPC_ERROR_RESPONSE') {
      const handler = pendingRequests.get(id)
      if (!handler) return
      pendingRequests.delete(id)

      if (type === 'RPC_RESPONSE') handler.resolve(payload)
      else handler.reject(new Error(payload))
    }
  })

  window.addEventListener('keydown', evt => {
    if (!(evt.ctrlKey || evt.metaKey || evt.altKey)) return

    const { ctrlKey, metaKey, shiftKey, altKey } = evt
    const key = evt.key.toLowerCase()

    window.parent.postMessage({ type: 'FORWARDED_KEYDOWN', key, ctrlKey, metaKey, shiftKey, altKey }, '*')
  })

  function call(method, params = {}) {
    return new Promise((resolve, reject) => {
      const id = crypto.randomUUID()

      pendingRequests.set(id, { resolve, reject })
      window.parent.postMessage({ type: 'RPC_REQUEST', id, method, params }, '*')
    })
  }

  return {
    system: {
      getCpuInfo: () => call('system.get_cpu_info'),
      getRamInfo: () => call('system.get_ram_info'),
      getDiskInfo: () => call('system.get_disk_info'),
      getInstalledApps: () => call('system.get_installed_apps'),
      openInstalledApp: path => call('system.open_installed_app', { path }),
    },
    utils: {
      getAssetUrl: path => call('utils.get_asset_url', { path }),
      getAsset: path => call('utils.get_asset', { path }),
      formatBytes: (value, options) => call('utils.format_bytes', { value, options }),
    },
  }
})()
