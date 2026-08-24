import {
  ArrowUpRight,
  BriefcaseBusiness,
  ChevronRight,
  CircleAlert,
  MapPin,
  Radio,
  Search,
  SlidersHorizontal,
  Waves,
} from "lucide-react"
import {
  startTransition,
  useDeferredValue,
  useEffect,
  useState,
} from "react"

import { getJob, listJobs, listSources } from "@/api"
import type { JobResponse, SourceResponse } from "@/api"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Checkbox } from "@/components/ui/checkbox"
import { Input } from "@/components/ui/input"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Separator } from "@/components/ui/separator"
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet"
import { Skeleton } from "@/components/ui/skeleton"

const ALL_SOURCES = "all"
const PAGE_SIZE = 20

function apiErrorMessage(error: unknown) {
  if (
    typeof error === "object" &&
    error !== null &&
    "error" in error &&
    typeof error.error === "string"
  ) {
    return error.error
  }

  return "The jobs feed is unavailable. Please try again."
}

function formatDate(value?: string | null) {
  if (!value) {
    return "Recently listed"
  }

  return new Intl.DateTimeFormat(undefined, {
    day: "numeric",
    month: "short",
    year: "numeric",
  }).format(new Date(value))
}

function humanize(value?: string | null) {
  if (!value) {
    return null
  }

  return value.replaceAll("_", " ")
}

function JobDetail({
  jobId,
  onClose,
  source,
}: {
  jobId: string
  onClose: () => void
  source?: SourceResponse
}) {
  const [error, setError] = useState<string | null>(null)
  const [job, setJob] = useState<JobResponse | null>(null)

  useEffect(() => {
    const controller = new AbortController()

    void getJob({
      path: { id: jobId },
      signal: controller.signal,
    })
      .then((result) => {
        if (controller.signal.aborted) {
          return
        }

        if (result.error) {
          setError(apiErrorMessage(result.error))
          return
        }

        startTransition(() => setJob(result.data ?? null))
      })
      .catch((requestError: unknown) => {
        if (!controller.signal.aborted) {
          setError(apiErrorMessage(requestError))
        }
      })

    return () => controller.abort()
  }, [jobId])

  return (
    <Sheet open onOpenChange={(open) => !open && onClose()}>
      <SheetContent className="w-full overflow-y-auto border-l-4 border-l-primary p-0 sm:max-w-2xl">
        <SheetHeader className="border-b bg-secondary/45 px-6 py-8 pr-14 sm:px-10">
          <div className="mb-4 flex items-center gap-2 font-mono text-[0.68rem] font-semibold uppercase tracking-[0.2em] text-muted-foreground">
            <Radio className="size-3.5 text-accent-foreground" />
            Open position
          </div>
          <SheetTitle className="font-display text-3xl leading-[1.05] sm:text-5xl">
            {job?.title ?? "Loading position..."}
          </SheetTitle>
          <SheetDescription className="mt-3 flex flex-wrap items-center gap-x-3 gap-y-2 text-sm text-foreground/70">
            <span>{source?.organization ?? job?.sourceId ?? "jobs.surf"}</span>
            {job?.locations.length ? (
              <>
                <span aria-hidden="true">/</span>
                <span>{job.locations.map((location) => location.name).join(", ")}</span>
              </>
            ) : null}
          </SheetDescription>
        </SheetHeader>

        <div className="px-6 py-7 sm:px-10">
          {error ? (
            <div className="flex gap-3 border border-destructive/30 bg-destructive/5 p-4 text-sm text-destructive">
              <CircleAlert className="mt-0.5 size-4 shrink-0" />
              {error}
            </div>
          ) : null}

          {!job && !error ? (
            <div className="space-y-4" aria-label="Loading job details">
              <Skeleton className="h-4 w-2/5" />
              <Skeleton className="h-4 w-full" />
              <Skeleton className="h-4 w-5/6" />
              <Skeleton className="h-32 w-full" />
            </div>
          ) : null}

          {job ? (
            <>
              <div className="mb-7 flex flex-wrap gap-2">
                {job.workplace ? (
                  <Badge variant="secondary" className="capitalize">
                    {humanize(job.workplace)}
                  </Badge>
                ) : null}
                {job.employmentType ? (
                  <Badge variant="outline" className="capitalize">
                    {humanize(job.employmentType)}
                  </Badge>
                ) : null}
                <Badge variant="outline">{formatDate(job.publishedAt)}</Badge>
              </div>

              <Separator className="mb-7" />

              {job.descriptionHtml ? (
                <div
                  className="job-description"
                  dangerouslySetInnerHTML={{ __html: job.descriptionHtml }}
                />
              ) : (
                <p className="text-sm text-muted-foreground">
                  This source did not provide a full job description.
                </p>
              )}
            </>
          ) : null}
        </div>

        {job ? (
          <SheetFooter className="sticky bottom-0 border-t bg-background/95 px-6 py-4 backdrop-blur sm:px-10">
            <Button asChild size="lg" className="w-full sm:w-auto">
              <a href={job.applyUrl} rel="noreferrer" target="_blank">
                Apply at {source?.organization ?? "source"}
                <ArrowUpRight data-icon="inline-end" />
              </a>
            </Button>
          </SheetFooter>
        ) : null}
      </SheetContent>
    </Sheet>
  )
}

function JobRow({
  job,
  onSelect,
  source,
}: {
  job: JobResponse
  onSelect: (id: string) => void
  source?: SourceResponse
}) {
  const locations = job.locations.map((location) => location.name).join(", ")

  return (
    <button
      type="button"
      className="group grid w-full grid-cols-[1fr_auto] gap-4 border-b border-border/80 px-4 py-5 text-left transition-colors hover:bg-card focus-visible:bg-card focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring sm:grid-cols-[minmax(0,1fr)_13rem_auto] sm:items-center sm:px-6"
      onClick={() => onSelect(job.id)}
    >
      <span className="min-w-0">
        <span className="mb-1 block font-mono text-[0.65rem] font-semibold uppercase tracking-[0.17em] text-accent-foreground">
          {source?.organization ?? job.sourceId}
        </span>
        <span className="block font-display text-xl font-semibold leading-tight text-foreground sm:text-2xl">
          {job.title}
        </span>
        <span className="mt-2 flex flex-wrap gap-1.5 sm:hidden">
          {job.workplace ? (
            <Badge variant="secondary" className="capitalize">
              {humanize(job.workplace)}
            </Badge>
          ) : null}
          {job.employmentType ? (
            <Badge variant="outline" className="capitalize">
              {humanize(job.employmentType)}
            </Badge>
          ) : null}
        </span>
      </span>

      <span className="hidden min-w-0 text-sm text-muted-foreground sm:block">
        <span className="flex items-center gap-2">
          <MapPin className="size-3.5 shrink-0" />
          <span className="truncate">{locations || "Location not listed"}</span>
        </span>
        <span className="mt-1.5 flex items-center gap-2">
          <BriefcaseBusiness className="size-3.5 shrink-0" />
          <span className="capitalize">
            {humanize(job.workplace) ?? humanize(job.employmentType) ?? "Open role"}
          </span>
        </span>
      </span>

      <span className="self-center rounded-full border border-border p-2 transition group-hover:translate-x-1 group-hover:border-primary group-hover:bg-primary group-hover:text-primary-foreground">
        <ChevronRight className="size-4" />
      </span>
    </button>
  )
}

function App() {
  const [cursor, setCursor] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [jobs, setJobs] = useState<JobResponse[]>([])
  const [loading, setLoading] = useState(true)
  const [loadingMore, setLoadingMore] = useState(false)
  const [query, setQuery] = useState("")
  const [remote, setRemote] = useState(false)
  const [selectedJobId, setSelectedJobId] = useState<string | null>(null)
  const [sourceId, setSourceId] = useState(ALL_SOURCES)
  const [sources, setSources] = useState<SourceResponse[]>([])
  const [sourcesLoaded, setSourcesLoaded] = useState(false)
  const deferredQuery = useDeferredValue(query)

  useEffect(() => {
    const controller = new AbortController()

    void listSources({ signal: controller.signal })
      .then((result) => {
        if (!controller.signal.aborted && result.data) {
          startTransition(() => setSources(result.data.sources))
        }
      })
      .catch(() => undefined)
      .finally(() => {
        if (!controller.signal.aborted) {
          setSourcesLoaded(true)
        }
      })

    return () => controller.abort()
  }, [])

  useEffect(() => {
    const controller = new AbortController()

    void listJobs({
      query: {
        limit: PAGE_SIZE,
        query: deferredQuery.trim() || undefined,
        remote: remote || undefined,
        source: sourceId === ALL_SOURCES ? undefined : sourceId,
      },
      signal: controller.signal,
    })
      .then((result) => {
        if (controller.signal.aborted) {
          return
        }

        if (result.error) {
          setError(apiErrorMessage(result.error))
          setJobs([])
          setCursor(null)
          return
        }

        startTransition(() => {
          setCursor(result.data?.nextCursor ?? null)
          setJobs(result.data?.jobs ?? [])
        })
      })
      .catch((requestError: unknown) => {
        if (!controller.signal.aborted) {
          setError(apiErrorMessage(requestError))
          setJobs([])
          setCursor(null)
        }
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setLoading(false)
        }
      })

    return () => controller.abort()
  }, [deferredQuery, remote, sourceId])

  async function loadMore() {
    if (!cursor || loadingMore) {
      return
    }

    setLoadingMore(true)

    try {
      const result = await listJobs({
        query: {
          cursor,
          limit: PAGE_SIZE,
          query: deferredQuery.trim() || undefined,
          remote: remote || undefined,
          source: sourceId === ALL_SOURCES ? undefined : sourceId,
        },
      })

      if (result.error) {
        setError(apiErrorMessage(result.error))
        return
      }

      startTransition(() => {
        setCursor(result.data?.nextCursor ?? null)
        setJobs((current) => [...current, ...(result.data?.jobs ?? [])])
      })
    } catch (requestError) {
      setError(apiErrorMessage(requestError))
    } finally {
      setLoadingMore(false)
    }
  }

  function updateQuery(value: string) {
    setError(null)
    setLoading(true)
    setQuery(value)
  }

  function updateRemote(value: boolean) {
    setError(null)
    setLoading(true)
    setRemote(value)
  }

  function updateSource(value: string) {
    setError(null)
    setLoading(true)
    setSourceId(value)
  }

  const sourceById = new Map(sources.map((source) => [source.id, source]))
  const filtersActive = Number(remote) + Number(sourceId !== ALL_SOURCES)
  const searchPending = query !== deferredQuery
  const selectedSource = selectedJobId
    ? sourceById.get(jobs.find((job) => job.id === selectedJobId)?.sourceId ?? "")
    : undefined

  return (
    <div className="min-h-screen">
      <header className="border-b border-primary/20 bg-primary text-primary-foreground">
        <div className="mx-auto max-w-7xl px-5 sm:px-8 lg:px-12">
          <nav className="flex h-16 items-center justify-between border-b border-white/10">
            <a className="inline-flex items-center gap-2" href="/">
              <Waves className="size-6 text-accent" />
              <span className="font-mono text-sm font-bold tracking-[-0.04em]">
                jobs.surf
              </span>
            </a>
            <a
              className="font-mono text-[0.68rem] font-semibold uppercase tracking-[0.14em] text-primary-foreground/70 hover:text-primary-foreground"
              href="/docs"
            >
              API docs
            </a>
          </nav>

          <div className="flex flex-col gap-6 py-10 sm:flex-row sm:items-end sm:justify-between sm:py-12">
            <div>
              <h1 className="max-w-2xl font-display text-4xl font-semibold leading-tight tracking-[-0.025em] sm:text-5xl">
                Jobs from the source.
              </h1>
              <p className="mt-3 max-w-xl text-sm leading-relaxed text-primary-foreground/70 sm:text-base">
                Open roles collected directly from company careers pages.
              </p>
            </div>
            <div className="flex items-center gap-2 font-mono text-[0.68rem] font-semibold uppercase tracking-[0.14em] text-secondary">
              <Radio className="size-3.5 text-accent" />
              {sourcesLoaded ? `${sources.length} sources` : "Loading sources..."}
            </div>
          </div>
        </div>
      </header>

      <main className="mx-auto max-w-7xl px-5 py-8 sm:px-8 lg:px-12 lg:py-10">
        <section aria-labelledby="jobs-heading">
          <div className="mb-5 flex items-end justify-between gap-4">
            <div>
              <p className="font-mono text-[0.65rem] font-semibold uppercase tracking-[0.2em] text-accent-foreground">
                Job directory
              </p>
              <h2 id="jobs-heading" className="mt-1 font-display text-3xl font-semibold">
                Open jobs
              </h2>
            </div>
          </div>

          <div className="mb-5 grid gap-3 border-y border-border bg-card/60 p-3 sm:grid-cols-[minmax(0,1fr)_15rem_auto] sm:items-center sm:p-4">
            <label className="relative block">
              <span className="sr-only">Search jobs</span>
              <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                className="h-10 rounded-none border-0 bg-background pl-10 shadow-none ring-1 ring-border focus-visible:ring-2"
                onChange={(event) => updateQuery(event.target.value)}
                placeholder="Search Rust, design systems, infrastructure..."
                type="search"
                value={query}
              />
              {searchPending ? (
                <span className="absolute right-3 top-1/2 size-1.5 -translate-y-1/2 animate-pulse rounded-full bg-accent" />
              ) : null}
            </label>

            <Select value={sourceId} onValueChange={updateSource}>
              <SelectTrigger className="h-10 w-full rounded-none bg-background">
                <SelectValue placeholder="All sources" />
              </SelectTrigger>
              <SelectContent align="start">
                <SelectItem value={ALL_SOURCES}>All sources</SelectItem>
                {sources.map((source) => (
                  <SelectItem key={source.id} value={source.id}>
                    {source.organization}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>

            <label className="flex h-10 cursor-pointer items-center justify-between gap-3 border border-border bg-background px-3 text-sm sm:justify-start">
              <Checkbox
                checked={remote}
                onCheckedChange={(checked) => updateRemote(checked === true)}
              />
              Remote only
              {filtersActive ? (
                <Badge className="ml-auto size-5 justify-center p-0" variant="secondary">
                  {filtersActive}
                </Badge>
              ) : null}
            </label>
          </div>

          <div className="overflow-hidden border border-border bg-background">
            <div className="flex items-center justify-between border-b border-border bg-muted/60 px-4 py-2 font-mono text-[0.62rem] font-semibold uppercase tracking-[0.16em] text-muted-foreground sm:px-6">
              <span>{loading ? "Loading jobs..." : `${jobs.length} jobs loaded`}</span>
              <span className="flex items-center gap-1.5">
                <SlidersHorizontal className="size-3" /> newest first
              </span>
            </div>

            {loading && jobs.length === 0 ? (
              <div className="space-y-px bg-border" aria-label="Loading jobs">
                {[0, 1, 2, 3].map((item) => (
                  <div className="bg-background px-6 py-6" key={item}>
                    <Skeleton className="mb-2 h-3 w-24" />
                    <Skeleton className="h-7 w-3/5" />
                    <Skeleton className="mt-3 h-4 w-2/5" />
                  </div>
                ))}
              </div>
            ) : null}

            {error ? (
              <div className="flex items-center gap-3 border-b border-destructive/20 bg-destructive/5 px-5 py-4 text-sm text-destructive">
                <CircleAlert className="size-4 shrink-0" />
                {error}
              </div>
            ) : null}

            {!loading && !error && jobs.length === 0 ? (
              <div className="px-6 py-16 text-center">
                <BriefcaseBusiness className="mx-auto mb-4 size-7 text-muted-foreground" />
                <h3 className="font-display text-2xl font-semibold">No jobs found.</h3>
                <p className="mx-auto mt-2 max-w-md text-sm text-muted-foreground">
                  Clear a filter or try a broader search.
                </p>
              </div>
            ) : null}

            {jobs.map((job) => (
              <JobRow
                job={job}
                key={job.id}
                onSelect={setSelectedJobId}
                source={sourceById.get(job.sourceId)}
              />
            ))}

            {cursor ? (
              <div className="p-4 text-center sm:p-6">
                <Button
                  disabled={loadingMore}
                  onClick={() => void loadMore()}
                  variant="outline"
                >
                  {loadingMore ? "Loading..." : "Load more"}
                </Button>
              </div>
            ) : null}
          </div>
        </section>
      </main>

      <footer className="mt-8 border-t border-border bg-card/70">
        <div className="mx-auto flex max-w-7xl flex-col gap-2 px-5 py-7 font-mono text-[0.65rem] uppercase tracking-[0.14em] text-muted-foreground sm:flex-row sm:items-center sm:justify-between sm:px-8 lg:px-12">
          <span>Aggregated from company career pages</span>
          <a className="hover:text-foreground" href="/healthz">
            System status
          </a>
        </div>
      </footer>

      {selectedJobId ? (
        <JobDetail
          jobId={selectedJobId}
          onClose={() => setSelectedJobId(null)}
          source={selectedSource}
        />
      ) : null}
    </div>
  )
}

export default App
