window.widgetEditorApi = {
  sendData: payload => window.parent.postMessage({ type: 'WIDGET_EDITOR_DATA', payload }, '*'),
  onRequestData: callback => {
    window.addEventListener('message', evt => {
      if (evt.data.type === 'REQUEST_WIDGET_EDITOR_DATA') callback()
    })
  },
}
