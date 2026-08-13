import { contextBridge } from "electron"

contextBridge.exposeInMainWorld("sonar", {
  platform: process.platform,
})
