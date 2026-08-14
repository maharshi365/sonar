import { useState } from "react"
import { Check, Clock3, Copy, Database, Loader2, Trash2 } from "lucide-react"

import { Button } from "@/components/ui/button"
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import type { HistoryEntry } from "../../../../shared/history"

const dateFormatter = new Intl.DateTimeFormat(undefined, {
  dateStyle: "medium",
  timeStyle: "short",
})

interface HistoryListProps {
  entries: HistoryEntry[]
  deletingId?: number
  hasNextPage: boolean
  isFetchingNextPage: boolean
  onDelete: (id: number) => void
  onLoadMore: () => void
}

export function HistoryList({
  entries,
  deletingId,
  hasNextPage,
  isFetchingNextPage,
  onDelete,
  onLoadMore,
}: HistoryListProps) {
  const [copiedId, setCopiedId] = useState<number | null>(null)

  async function copyEntry(entry: HistoryEntry): Promise<void> {
    await navigator.clipboard.writeText(entry.text)
    setCopiedId(entry.id)
    window.setTimeout(
      () => setCopiedId((current) => (current === entry.id ? null : current)),
      1500
    )
  }

  return (
    <>
      <div className="mt-8 grid gap-3">
        {entries.map((entry) => (
          <HistoryListItem
            key={entry.id}
            entry={entry}
            copied={copiedId === entry.id}
            deleting={deletingId === entry.id}
            onCopy={() => void copyEntry(entry)}
            onDelete={() => onDelete(entry.id)}
          />
        ))}
      </div>

      {hasNextPage ? (
        <div className="flex justify-center py-8">
          <Button
            variant="outline"
            disabled={isFetchingNextPage}
            onClick={onLoadMore}
          >
            {isFetchingNextPage ? (
              <Loader2 className="animate-spin" />
            ) : (
              <Database />
            )}
            Load older transcriptions
          </Button>
        </div>
      ) : (
        <p className="py-8 text-center text-xs uppercase tracking-[0.18em] text-muted-foreground">
          End of local history
        </p>
      )}
    </>
  )
}

interface HistoryListItemProps {
  entry: HistoryEntry
  copied: boolean
  deleting: boolean
  onCopy: () => void
  onDelete: () => void
}

function HistoryListItem({
  entry,
  copied,
  deleting,
  onCopy,
  onDelete,
}: HistoryListItemProps) {
  return (
    <Card size="sm">
      <CardHeader className="border-b">
        <CardTitle className="flex min-w-0 items-center gap-2 text-sm">
          <Clock3 className="size-3.5 shrink-0 text-primary" />
          <time dateTime={new Date(entry.createdAt).toISOString()}>
            {dateFormatter.format(entry.createdAt)}
          </time>
        </CardTitle>
        <CardDescription
          className="min-w-0 truncate font-mono text-xs"
          title={entry.modelId}
        >
          {entry.modelId}
        </CardDescription>
        <CardAction className="flex gap-1">
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label="Copy transcription"
            title="Copy transcription"
            onClick={onCopy}
          >
            {copied ? <Check /> : <Copy />}
          </Button>
          <Button
            variant="destructive"
            size="icon-sm"
            aria-label="Delete transcription"
            title="Delete transcription"
            disabled={deleting}
            onClick={onDelete}
          >
            {deleting ? <Loader2 className="animate-spin" /> : <Trash2 />}
          </Button>
        </CardAction>
      </CardHeader>
      <CardContent>
        <p className="whitespace-pre-wrap wrap-break-word text-sm leading-7 text-foreground">
          {entry.text}
        </p>
      </CardContent>
    </Card>
  )
}
