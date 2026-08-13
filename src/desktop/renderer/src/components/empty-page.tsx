import type { ReactNode } from "react"

import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty"

export function EmptyPage({
  title,
  description,
  icon,
}: {
  title: string
  description: string
  icon: ReactNode
}) {
  return (
    <div className="mx-auto flex min-h-full max-w-4xl px-8 py-10 lg:px-12">
      <Empty className="border bg-card/20">
        <EmptyHeader>
          <EmptyMedia variant="icon">{icon}</EmptyMedia>
          <EmptyTitle>{title}</EmptyTitle>
          <EmptyDescription>{description}</EmptyDescription>
        </EmptyHeader>
      </Empty>
    </div>
  )
}
