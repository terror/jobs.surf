import * as React from "react"

import { cn } from "@/lib/utils"

const styles = {
  default: "bg-primary text-primary-foreground",
  outline: "border-border text-foreground",
  secondary: "bg-secondary text-secondary-foreground",
} as const

function Badge({
  className,
  variant = "default",
  ...props
}: React.ComponentProps<"span"> & { variant?: keyof typeof styles }) {
  return (
    <span
      className={cn(
        "inline-flex h-5 w-fit shrink-0 items-center justify-center overflow-hidden border border-transparent px-2 py-0.5 text-xs font-medium whitespace-nowrap",
        styles[variant],
        className
      )}
      {...props}
    />
  )
}

export { Badge }
