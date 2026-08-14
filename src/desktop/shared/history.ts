export interface HistoryEntry {
  id: number
  createdAt: number
  text: string
  modelId: string
}

export interface HistoryPage {
  entries: HistoryEntry[]
  hasMore: boolean
}
