import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { useQueryClient } from "@tanstack/react-query"

import type { ModelStatus } from "../../../shared/models"

/** Live, per-model download progress derived from IPC progress events. */
export interface ModelProgress {
  /** 0–100. */
  percentage: number
  downloaded: number
  total: number
  /** Smoothed download speed in bytes/sec. */
  bytesPerSecond: number
}

export interface UseModelsResult {
  models: ModelStatus[]
  isLoading: boolean
  error: string | null
  /** model_id -> live progress while downloading. */
  progress: Record<string, ModelProgress>
  /** model_id -> error message from the most recent failed action. */
  errors: Record<string, string>
  refresh: () => Promise<void>
  download: (modelId: string) => Promise<void>
  cancel: (modelId: string) => Promise<void>
  remove: (modelId: string) => Promise<void>
}

/**
 * Owns model list state and subscribes to download-progress events.
 *
 * The list is the source of truth for `isDownloaded` / `isDownloading`; the
 * `progress` map holds transient byte/percent/speed info for in-flight
 * downloads. We refresh the list whenever an action completes so status stays
 * accurate.
 */
export function useModels(): UseModelsResult {
  const queryClient = useQueryClient()
  const [models, setModels] = useState<ModelStatus[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [progress, setProgress] = useState<Record<string, ModelProgress>>({})
  const [errors, setErrors] = useState<Record<string, string>>({})

  // Per-model sample used to compute a smoothed speed.
  const speedSamples = useRef<
    Record<string, { time: number; downloaded: number; smoothed: number }>
  >({})

  const refresh = useCallback(async () => {
    try {
      const next = await window.sonar.models.list()
      setModels(next)
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setIsLoading(false)
    }
  }, [])

  useEffect(() => {
    void refresh()
  }, [refresh])

  // Subscribe to progress events once.
  useEffect(() => {
    const unsubscribe = window.sonar.models.onProgress((event) => {
      const now = performance.now()
      const prevSample = speedSamples.current[event.modelId]

      let bytesPerSecond = prevSample?.smoothed ?? 0
      if (prevSample) {
        const dt = (now - prevSample.time) / 1000
        const dBytes = event.downloaded - prevSample.downloaded
        if (dt > 0 && dBytes >= 0) {
          const instant = dBytes / dt
          // Exponential moving average to smooth out jitter.
          bytesPerSecond = prevSample.smoothed * 0.8 + instant * 0.2
        }
      }

      speedSamples.current[event.modelId] = {
        time: now,
        downloaded: event.downloaded,
        smoothed: bytesPerSecond,
      }

      setProgress((current) => ({
        ...current,
        [event.modelId]: {
          percentage: event.percentage,
          downloaded: event.downloaded,
          total: event.total,
          bytesPerSecond,
        },
      }))
    })

    return unsubscribe
  }, [])

  const clearProgress = useCallback((modelId: string) => {
    delete speedSamples.current[modelId]
    setProgress((current) => {
      const next = { ...current }
      delete next[modelId]
      return next
    })
  }, [])

  const setModelError = useCallback((modelId: string, message: string | null) => {
    setErrors((current) => {
      const next = { ...current }
      if (message) next[modelId] = message
      else delete next[modelId]
      return next
    })
  }, [])

  const download = useCallback(
    async (modelId: string) => {
      setModelError(modelId, null)
      // Optimistically flip the flag so the UI shows progress immediately.
      setModels((current) =>
        current.map((m) =>
          m.id === modelId ? { ...m, isDownloading: true } : m
        )
      )
      try {
        await window.sonar.models.download(modelId)
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err)
        // Cancellation is a normal outcome, not an error to surface loudly.
        if (!message.toLowerCase().includes("cancel")) {
          setModelError(modelId, message)
        }
      } finally {
        clearProgress(modelId)
        await refresh()
      }
    },
    [clearProgress, refresh, setModelError]
  )

  const cancel = useCallback(async (modelId: string) => {
    await window.sonar.models.cancel(modelId)
  }, [])

  const remove = useCallback(
    async (modelId: string) => {
      setModelError(modelId, null)
      try {
        await window.sonar.models.remove(modelId)
      } catch (err) {
        setModelError(modelId, err instanceof Error ? err.message : String(err))
      } finally {
        await refresh()
        // Removing a model may have cleared it as the default speech model in
        // the main process, so refresh any cached settings.
        void queryClient.invalidateQueries({ queryKey: ["settings"] })
      }
    },
    [queryClient, refresh, setModelError]
  )

  return useMemo(
    () => ({
      models,
      isLoading,
      error,
      progress,
      errors,
      refresh,
      download,
      cancel,
      remove,
    }),
    [models, isLoading, error, progress, errors, refresh, download, cancel, remove]
  )
}
