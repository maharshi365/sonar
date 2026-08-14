import { useEffect } from "react"
import { useInfiniteQuery, useMutation, useQueryClient } from "@tanstack/react-query"

const PAGE_SIZE = 30
const historyQueryKey = ["history"] as const

export function useHistory() {
  const queryClient = useQueryClient()
  const historyQuery = useInfiniteQuery({
    queryKey: historyQueryKey,
    queryFn: ({ pageParam }) => window.sonar.history.list(pageParam, PAGE_SIZE),
    initialPageParam: undefined as number | undefined,
    getNextPageParam: (lastPage) => {
      if (!lastPage.hasMore) return undefined
      return lastPage.entries.at(-1)?.id
    },
  })
  const deleteMutation = useMutation({
    mutationFn: (id: number) => window.sonar.history.delete(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: historyQueryKey }),
  })
  const clearMutation = useMutation({
    mutationFn: () => window.sonar.history.clear(),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: historyQueryKey }),
  })

  useEffect(
    () =>
      window.sonar.history.onChanged(() => {
        void queryClient.invalidateQueries({ queryKey: historyQueryKey })
      }),
    [queryClient]
  )

  return {
    entries: historyQuery.data?.pages.flatMap((page) => page.entries) ?? [],
    error: historyQuery.error,
    isPending: historyQuery.isPending,
    hasNextPage: historyQuery.hasNextPage,
    isFetchingNextPage: historyQuery.isFetchingNextPage,
    fetchNextPage: historyQuery.fetchNextPage,
    deleteEntry: deleteMutation.mutate,
    deletingId: deleteMutation.isPending ? deleteMutation.variables : undefined,
    clearHistory: clearMutation.mutate,
    isClearing: clearMutation.isPending,
    updateError: deleteMutation.error ?? clearMutation.error,
  }
}
