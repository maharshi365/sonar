import { HistoryHeader } from "./history-header"
import { HistoryList } from "./history-list"
import {
  HistoryEmpty,
  HistoryLoading,
  HistoryLoadError,
  HistoryUpdateError,
} from "./history-states"
import { useHistory } from "@/hooks/use-history"

export function HistoryPage() {
  const history = useHistory()

  if (history.isPending) {
    return (
      <>
        <HistoryHeader />
        <HistoryLoading />
      </>
    )
  }

  if (history.error) {
    return (
      <>
        <HistoryHeader />
        <HistoryLoadError message={history.error.message} />
      </>
    )
  }

  if (history.entries.length === 0) {
    return (
      <>
        <HistoryHeader />
        <HistoryEmpty />
      </>
    )
  }

  function clearAll(): void {
    if (window.confirm("Delete all transcription history? This cannot be undone.")) {
      history.clearHistory()
    }
  }

  return (
    <>
      <HistoryHeader isClearing={history.isClearing} onClear={clearAll} />
      {history.updateError ? <HistoryUpdateError /> : null}
      <HistoryList
        entries={history.entries}
        deletingId={history.deletingId}
        hasNextPage={history.hasNextPage}
        isFetchingNextPage={history.isFetchingNextPage}
        onDelete={history.deleteEntry}
        onLoadMore={() => void history.fetchNextPage()}
      />
    </>
  )
}
