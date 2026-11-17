import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface AgentEvent {
  id: number;
  source: string;
  event: string;
  title: string;
  detail: string;
  ts: number;
}

export interface AgentsStatus {
  listening: boolean;
  endpoint: string;
  requests: number;
  startedAt: number;
  lastAt: number;
  opencodePlugin: boolean;
  opencodePath: string | null;
}

export interface ModelUsage {
  provider: string;
  model: string;
  label: string;
  sessions: number;
  inputTokens: number;
  outputTokens: number;
  reasoningTokens: number;
  cacheTokens: number;
  cost: number;
  contextUsed: number;
  contextLimit: number | null;
  contextLeft: number | null;
  lastActive: number;
}

export const agents = $state<{
  events: AgentEvent[];
  status: AgentsStatus | null;
  loaded: boolean;
}>({ events: [], status: null, loaded: false });

export const usage = $state<{ models: ModelUsage[]; loaded: boolean }>({
  models: [],
  loaded: false,
});

let started = false;

export type AgentPhase = "working" | "done" | "error" | "attention" | "idle";

export function agentPhase(event: Pick<AgentEvent, "event"> | undefined): AgentPhase {
  if (!event) return "idle";
  const name = event.event.toLowerCase();
  if (name.includes("error") || name.includes("fail")) return "error";
  if (name.includes("done") || name.includes("stop") || name.includes("finish")) return "done";
  if (name.includes("attention") || name.includes("permission") || name.includes("notify")) {
    return "attention";
  }
  if (name.includes("working") || name.includes("tool") || name.includes("start")) {
    return "working";
  }
  return "idle";
}

export async function initAgents(): Promise<void> {
  if (started) return;
  started = true;
  await Promise.allSettled([
    invoke<AgentEvent[]>("agents_events").then((events) => {
      agents.events = events;
    }),
    invoke<AgentsStatus>("agents_status").then((status) => {
      agents.status = status;
    }),
  ]);
  agents.loaded = true;
  void refreshUsage();
  await listen<AgentEvent>("agent-event", (event) => {
    agents.events = [
      event.payload,
      ...agents.events.filter((item) => item.id !== event.payload.id),
    ].slice(0, 120);
    if (agents.status) {
      agents.status = {
        ...agents.status,
        lastAt: event.payload.ts,
        requests: agents.status.requests + 1,
      };
    }
  });
  await listen("agents-cleared", () => {
    agents.events = [];
  });
  await listen<AgentsStatus>("agents-status", (event) => {
    agents.status = event.payload;
  });
}

export async function refreshAgents(): Promise<void> {
  try {
    agents.events = await invoke<AgentEvent[]>("agents_events");
    agents.status = await invoke<AgentsStatus>("agents_status");
  } catch {
    /* the hook server may be offline */
  }
}

export async function refreshUsage(): Promise<void> {
  try {
    usage.models = await invoke<ModelUsage[]>("agent_usage");
    usage.loaded = true;
  } catch {
    /* OpenCode may not be installed or the database may be locked */
  }
}

export async function clearAgents(): Promise<void> {
  await invoke("agents_clear");
  agents.events = [];
}

export async function testAgent(): Promise<void> {
  await invoke("agents_test");
}

export async function installOpenCode(): Promise<string> {
  const path = await invoke<string>("agents_install_opencode");
  await refreshAgents();
  return path;
}
