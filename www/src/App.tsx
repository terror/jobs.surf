import {
  ArrowUpRight,
  BriefcaseBusiness,
  ChevronRight,
  CircleAlert,
  MapPin,
  Search,
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
      <SheetContent className="w-full overflow-y-auto border-l-2 border-l-primary bg-background p-0 shadow-[-6px_0_0_var(--accent)] sm:max-w-2xl">
        <SheetHeader className="border-b-2 border-primary bg-secondary px-6 py-8 pr-14 sm:px-10">
          <div className="mb-4 flex items-center gap-2 text-[0.68rem] font-bold uppercase tracking-[0.12em] text-muted-foreground">
            <span className="size-2 bg-accent ring-1 ring-primary" />
            position / open
          </div>
          <SheetTitle className="text-2xl font-extrabold leading-tight tracking-[-0.04em] sm:text-4xl">
            {job?.title ?? "Loading position..."}
          </SheetTitle>
          <SheetDescription className="mt-4 flex flex-wrap items-center gap-x-3 gap-y-2 text-xs text-foreground/70 sm:text-sm">
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
            <div className="flex gap-3 border-2 border-destructive bg-destructive/5 p-4 text-sm text-destructive">
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
                  <Badge
                    variant="secondary"
                    className="rounded-[1px] border border-primary/30 uppercase"
                  >
                    {humanize(job.workplace)}
                  </Badge>
                ) : null}
                {job.employmentType ? (
                  <Badge
                    variant="outline"
                    className="rounded-[1px] border-primary/30 uppercase"
                  >
                    {humanize(job.employmentType)}
                  </Badge>
                ) : null}
                <Badge
                  variant="outline"
                  className="rounded-[1px] border-primary/30 uppercase"
                >
                  {formatDate(job.publishedAt)}
                </Badge>
              </div>

              <hr className="mb-7 h-px border-0 bg-primary/20" />

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
          <SheetFooter className="sticky bottom-0 border-t-2 border-primary bg-background/95 px-6 py-4 backdrop-blur sm:px-10">
            <Button
              asChild
              className="tp-button h-11 w-full rounded-[2px] border-2 border-primary bg-accent px-5 text-accent-foreground hover:bg-accent/90 sm:w-auto"
            >
              <a href={job.applyUrl} rel="noreferrer" target="_blank">
                Apply at {source?.organization ?? "source"}
                <ArrowUpRight />
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
      className="group grid w-full grid-cols-[1fr_auto] gap-4 border-b border-primary/20 px-4 py-5 text-left transition-colors hover:bg-secondary/45 focus-visible:bg-secondary/45 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring sm:grid-cols-[minmax(0,1fr)_15rem_auto] sm:items-center sm:px-6"
      onClick={() => onSelect(job.id)}
    >
      <span className="min-w-0">
        <span className="mb-1.5 block text-[0.65rem] font-bold uppercase tracking-[0.1em] text-muted-foreground">
          {source?.organization ?? job.sourceId}
        </span>
        <span className="block text-base font-bold leading-snug tracking-[-0.025em] text-foreground sm:text-lg">
          {job.title}
        </span>
        <span className="mt-2 flex flex-wrap gap-1.5 sm:hidden">
          {job.workplace ? (
            <Badge
              variant="secondary"
              className="rounded-[1px] border border-primary/20 text-[0.62rem] uppercase"
            >
              {humanize(job.workplace)}
            </Badge>
          ) : null}
          {job.employmentType ? (
            <Badge
              variant="outline"
              className="rounded-[1px] border-primary/20 text-[0.62rem] uppercase"
            >
              {humanize(job.employmentType)}
            </Badge>
          ) : null}
        </span>
      </span>

      <span className="hidden min-w-0 text-xs text-muted-foreground sm:block">
        <span className="flex items-center gap-2">
          <MapPin className="size-3.5 shrink-0" />
          <span className="truncate">{locations || "Location not listed"}</span>
        </span>
        <span className="mt-1.5 flex items-center gap-2">
          <BriefcaseBusiness className="size-3.5 shrink-0" />
          <span className="uppercase">
            {humanize(job.workplace) ?? humanize(job.employmentType) ?? "Open role"}
          </span>
        </span>
      </span>

      <span className="job-arrow self-center border-2 border-primary bg-background p-2 transition group-hover:bg-accent group-hover:text-accent-foreground">
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
    <div className="min-h-screen bg-background">
      <header className="bg-primary text-primary-foreground">
        <div className="mx-auto max-w-[83.5rem] px-4 sm:px-8">
          <nav className="flex h-16 items-center justify-between border-b border-primary-foreground/15">
            <a className="inline-flex items-center gap-2.5" href="/">
              <span className="grid size-7 place-items-center border border-primary-foreground/60 text-accent">
                <Waves className="size-4" />
              </span>
              <span className="text-sm font-extrabold tracking-[-0.05em] sm:text-base">
                jobs.surf
              </span>
            </a>
            <div className="flex items-center gap-4 text-[0.68rem] font-bold uppercase sm:gap-7 sm:text-xs">
              <a className="nav-link" href="#open-jobs">
                Jobs
              </a>
              <a className="nav-link" href="/docs">
                API docs
              </a>
              <a className="nav-link hidden sm:block" href="/healthz">
                Status
              </a>
            </div>
          </nav>
        </div>
      </header>

      <main
        id="open-jobs"
        className="mx-auto max-w-[83.5rem] scroll-mt-6 px-3 pb-12 sm:px-8 lg:pb-16"
      >
        <section>
          <fieldset className="mt-4 border-2 border-primary bg-background px-3 pb-4 pt-3 sm:px-5 sm:pb-5">
            <legend className="bg-background px-2 text-xs font-bold uppercase">
              Search controls
            </legend>
            <div className="grid gap-4 sm:grid-cols-[minmax(0,1fr)_17rem_12rem] sm:items-end">
              <label className="block">
                <span className="mb-2 block text-[0.68rem] font-bold uppercase text-muted-foreground">
                  Keywords
                </span>
                <span className="relative block">
                  <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
                  <Input
                    className="h-11 rounded-[2px] border-2 border-primary/40 bg-background pl-10 shadow-none focus-visible:border-primary focus-visible:ring-2"
                    onChange={(event) => updateQuery(event.target.value)}
                    placeholder="Rust, design systems, infrastructure..."
                    type="search"
                    value={query}
                  />
                  {searchPending ? (
                    <span className="absolute right-3 top-1/2 size-2 -translate-y-1/2 animate-pulse bg-accent ring-1 ring-primary" />
                  ) : null}
                </span>
              </label>

              <div>
                <span className="mb-2 block text-[0.68rem] font-bold uppercase text-muted-foreground">
                  Company source
                </span>
                <Select value={sourceId} onValueChange={updateSource}>
                  <SelectTrigger className="h-11 w-full rounded-[2px] border-2 border-primary/40 bg-background focus-visible:border-primary focus-visible:ring-2">
                    <SelectValue placeholder="All sources" />
                  </SelectTrigger>
                  <SelectContent
                    align="start"
                    className="rounded-[2px] border-2 border-primary shadow-[4px_4px_0_var(--primary)]"
                  >
                    <SelectItem className="rounded-none" value={ALL_SOURCES}>
                      All sources
                    </SelectItem>
                    {sources.map((source) => (
                      <SelectItem
                        className="rounded-none"
                        key={source.id}
                        value={source.id}
                      >
                        {source.organization}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              <div>
                <span className="mb-2 block text-[0.68rem] font-bold uppercase text-muted-foreground">
                  Workplace
                </span>
                <label className="flex h-11 cursor-pointer items-center gap-3 border-2 border-primary/40 bg-background px-3 text-xs font-bold uppercase hover:border-primary">
                  <Checkbox
                    checked={remote}
                    className="rounded-[1px] border-2 border-primary"
                    onCheckedChange={(checked) => updateRemote(checked === true)}
                  />
                  Remote only
                  {filtersActive ? (
                    <Badge className="ml-auto rounded-[1px] px-1.5 text-[0.6rem]">
                      {filtersActive} active
                    </Badge>
                  ) : null}
                </label>
              </div>
            </div>
          </fieldset>

          <div className="pixel-shadow mt-8">
            <div className="overflow-hidden border-2 border-primary bg-background">
              <div className="flex items-center justify-between border-b-2 border-primary bg-secondary px-4 py-3 text-[0.62rem] font-bold uppercase tracking-[0.06em] text-muted-foreground sm:px-6">
                <span>{loading ? "querying index..." : `${jobs.length} results loaded`}</span>
                <span>
                  sort // newest first
                  {filtersActive ? ` // ${filtersActive} active` : ""}
                </span>
              </div>

              {loading && jobs.length === 0 ? (
                <div className="space-y-px bg-primary/20" aria-label="Loading jobs">
                  {[0, 1, 2, 3].map((item) => (
                    <div className="bg-background px-6 py-6" key={item}>
                      <Skeleton className="mb-2 h-3 w-24 rounded-none" />
                      <Skeleton className="h-6 w-3/5 rounded-none" />
                      <Skeleton className="mt-3 h-4 w-2/5 rounded-none" />
                    </div>
                  ))}
                </div>
              ) : null}

              {error ? (
                <div className="flex items-center gap-3 border-b-2 border-destructive bg-destructive/5 px-5 py-4 text-sm text-destructive">
                  <CircleAlert className="size-4 shrink-0" />
                  {error}
                </div>
              ) : null}

              {!loading && !error && jobs.length === 0 ? (
                <div className="px-6 py-16 text-center">
                  <span className="mx-auto mb-5 grid size-12 place-items-center border-2 border-primary bg-secondary">
                    <BriefcaseBusiness className="size-5" />
                  </span>
                  <h3 className="text-xl font-extrabold tracking-[-0.04em]">
                    No jobs found.
                  </h3>
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
                <div className="p-5 text-center sm:p-7">
                  <Button
                    className="tp-button h-11 rounded-[2px] border-2 border-primary bg-accent px-5 text-accent-foreground hover:bg-accent/90"
                    disabled={loadingMore}
                    onClick={() => void loadMore()}
                  >
                    {loadingMore ? "Loading..." : "Load more results"}
                  </Button>
                </div>
              ) : null}
            </div>
          </div>
        </section>
      </main>

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
