// please don't remove this listener even if the editor doesn't send any data back
window.widgetEditorApi.onRequestData(() => {
  window.widgetEditorApi.sendData({})
})
