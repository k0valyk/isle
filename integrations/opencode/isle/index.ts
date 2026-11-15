import { Plugin } from "@opencode/plugin"

const ENDPOINT = "http://127.0.0.1:45871/event"
const WORKING_INTERVAL = 4000

let lastWorking = 0

async function send(payload: Record<string, unknown>): Promise<void> {
  try {
    await fetch(ENDPOINT, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(payload),
      signal: AbortSignal.timeout(2500),
    })
  } catch {
    // Isle is optional: never break an OpenCode session because the island is
    // closed or its local hook port is unavailable.
  }
}

function describe(event: unknown): string {
  try {
    const value = event as { type?: string; properties?: Record<string, unknown> }
    if (!value?.properties) return value?.type ?? "event"
    const properties = value.properties
    const session = properties.sessionID ?? properties.info
    if (typeof session === "string") return `${value.type} · ${session}`
    return value.type ?? "event"
  } catch {
    return "event"
  }
}

export default Plugin.define({
  id: "isle",
  async setup(ctx) {
    const controller = new AbortController()

    await send({
      source: "opencode",
      event: "hook",
      title: "OpenCode plugin connected",
      detail: ctx.location.directory,
    })

    const watch = async () => {
      while (!controller.signal.aborted) {
        try {
          for await (const event of ctx.event.subscribe({ signal: controller.signal })) {
            if (controller.signal.aborted) return
            const type = String((event as { type?: string }).type ?? "")
            const now = Date.now()

            if (type.includes("error") || type.includes("failed")) {
              await send({
                source: "opencode",
                event: "error",
                title: "OpenCode error",
                detail: describe(event),
              })
              continue
            }
            if (
              type === "session.idle" ||
              type.endsWith(".idle") ||
              type.includes("completed") ||
              type.includes("finished")
            ) {
              await send({
                source: "opencode",
                event: "done",
                title: "OpenCode finished",
                detail: describe(event),
              })
              continue
            }
            if (type.includes("permission") && (type.includes("ask") || type.includes("request"))) {
              await send({
                source: "opencode",
                event: "attention",
                title: "OpenCode needs permission",
                detail: describe(event),
              })
              continue
            }
            if (
              (type.startsWith("session.") ||
                type.startsWith("tool.") ||
                type.startsWith("message.")) &&
              now - lastWorking > WORKING_INTERVAL
            ) {
              lastWorking = now
              await send({
                source: "opencode",
                event: "working",
                title: "OpenCode is working",
                detail: describe(event),
              })
            }
          }
        } catch (error) {
          if (controller.signal.aborted) return
          await send({
            source: "opencode",
            event: "error",
            title: "OpenCode event stream stopped",
            detail: String(error).slice(0, 300),
          })
          await new Promise((resolve) => setTimeout(resolve, 3000))
        }
      }
    }

    void watch()

    return () => {
      controller.abort()
      void send({
        source: "opencode",
        event: "stop",
        title: "OpenCode plugin disconnected",
        detail: "",
      })
    }
  },
})
