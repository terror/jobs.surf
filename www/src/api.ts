import { client } from "@/client/client.gen"

client.setConfig({
  baseUrl: import.meta.env.VITE_API_URL ?? "",
})

export { getJob, listJobs, listSources } from "@/client/sdk.gen"
export type { JobResponse, SourceResponse } from "@/client/types.gen"
