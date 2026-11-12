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
    audio: {
      getMasterVolume: () => call('audio.get_master_volume'),
      setMasterVolume: level => call('audio.set_master_volume', { level }),
      toggleMasterMute: () => call('audio.toggle_master_mute'),

      onAudioCaptured: callback => {
        window.addEventListener('message', evt => {
          if (evt.data.type === 'AUDIO_CAPTURED') callback(evt.data.payload)
        })
      },
    },
    media: {
      getActiveMedia: () => call('media.get_active_media'),
      sendCommand: (playerId, command) => call('media.send_command', { playerId, command }),

      onMediaUpdated: callback => {
        window.addEventListener('message', evt => {
          if (evt.data.type === 'MEDIA_UPDATED') callback(evt.data.payload)
        })
      },
    },
    system: {
      getCpuInfo: () => call('system.get_cpu_info'),
      getRamInfo: () => call('system.get_ram_info'),
      getDiskInfo: () => call('system.get_disk_info'),
      getInstalledApps: () => call('system.get_installed_apps'),
      openInstalledApp: path => call('system.open_installed_app', { path }),
      openRecycleBin: () => call('system.open_recycle_bin'),
      getRecycleBinCount: () => call('system.get_recycle_bin_count'),
    },
    utils: {
      getAssetUrl: path => call('utils.get_asset_url', { path }),
      getAsset: path => call('utils.get_asset', { path }),
      formatBytes: (value, options) => call('utils.format_bytes', { value, options }),
    },
  }
})()
