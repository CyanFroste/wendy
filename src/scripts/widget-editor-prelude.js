window.widgetEditorApi = (() => {
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

  function call(method, params = {}) {
    return new Promise((resolve, reject) => {
      const id = crypto.randomUUID()

      pendingRequests.set(id, { resolve, reject })
      window.parent.postMessage({ type: 'RPC_REQUEST', id, method, params }, '*')
    })
  }

  return {
    system: {
      readDir: path => call('system.read_dir', { path }),
      readFile: path => call('system.read_file', { path }),
      getInstalledApps: () => call('system.get_installed_apps'),
    },
    utils: {
      getAssetUrl: path => call('utils.get_asset_url', { path }),
      getAsset: path => call('utils.get_asset', { path }),
    },

    sendData: payload => window.parent.postMessage({ type: 'WIDGET_EDITOR_DATA', payload }, '*'),
    onRequestData: callback => {
      window.addEventListener('message', evt => {
        if (evt.data.type === 'REQUEST_WIDGET_EDITOR_DATA') callback()
      })
    },
  }
})()
