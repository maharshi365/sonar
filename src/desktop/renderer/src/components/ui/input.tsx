import * as React from "react"
import { Input as InputPrimitive } from "@base-ui/react/input"
import { ChevronDownIcon, ChevronUpIcon } from "lucide-react"

import { cn } from "@/lib/utils"

function Input({ className, type, ...props }: React.ComponentProps<"input">) {
  return (
    <InputPrimitive
      type={type}
      data-slot="input"
      className={cn(
        "h-8 w-full min-w-0 rounded-lg border border-input bg-transparent px-2.5 py-1 text-base transition-colors outline-none file:inline-flex file:h-6 file:border-0 file:bg-transparent file:text-sm file:font-medium file:text-foreground placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:pointer-events-none disabled:cursor-not-allowed disabled:bg-input/50 disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-3 aria-invalid:ring-destructive/20 md:text-sm dark:bg-input/30 dark:disabled:bg-input/80 dark:aria-invalid:border-destructive/50 dark:aria-invalid:ring-destructive/40",
        className
      )}
      {...props}
    />
  )
}

function NumberInput({
  className,
  ...props
}: Omit<React.ComponentProps<"input">, "type">) {
  const inputRef = React.useRef<HTMLInputElement>(null)

  const step = (direction: "up" | "down") => {
    const input = inputRef.current
    if (!input) return
    direction === "up" ? input.stepUp() : input.stepDown()
    input.focus()
  }

  return (
    <div className={cn("relative", className)}>
      <Input
        ref={inputRef}
        type="number"
        className="appearance-none pr-7 [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none"
        {...props}
      />
      <div className="absolute inset-y-px right-px flex w-6 flex-col border-l border-input text-muted-foreground">
        <button
          type="button"
          tabIndex={-1}
          aria-label="Increase value"
          className="flex min-h-0 flex-1 items-center justify-center hover:bg-muted hover:text-foreground"
          onMouseDown={(event) => event.preventDefault()}
          onClick={() => step("up")}
        >
          <ChevronUpIcon className="size-3" />
        </button>
        <button
          type="button"
          tabIndex={-1}
          aria-label="Decrease value"
          className="flex min-h-0 flex-1 items-center justify-center border-t border-input hover:bg-muted hover:text-foreground"
          onMouseDown={(event) => event.preventDefault()}
          onClick={() => step("down")}
        >
          <ChevronDownIcon className="size-3" />
        </button>
      </div>
    </div>
  )
}

export { Input, NumberInput }
