import { invoke } from "@tauri-apps/api/core";

interface TimerState {
  total: number;
  remaining: number;
  running: boolean;
  finished: boolean;
}

export const timer = $state<TimerState>({
  total: 25 * 60,
  remaining: 25 * 60,
  running: false,
  finished: false,
});

let ticker: ReturnType<typeof setInterval> | undefined;

function startTicker(): void {
  if (ticker) return;
  ticker = setInterval(() => {
    if (!timer.running) return;
    if (timer.remaining > 0) {
      timer.remaining -= 1;
      if (timer.remaining === 0) {
        timer.finished = true;
        timer.running = false;
        stopTicker();
      }
    }
  }, 1000);
}

function stopTicker(): void {
  if (ticker) {
    clearInterval(ticker);
    ticker = undefined;
  }
}

function schedule(): void {
  if (!timer.running || timer.remaining <= 0) return;
  void invoke("stop_timer_sound").catch(() => {});
  void invoke("timer_schedule", { seconds: timer.remaining }).catch(() => {
    /* the JS ticker is still a fallback for the visual state */
  });
}

function cancel(): void {
  void invoke("timer_cancel").catch(() => {});
}

export function toggleTimer(): void {
  if (timer.finished) {
    timer.remaining = timer.total;
    timer.finished = false;
    timer.running = true;
    startTicker();
    schedule();
    return;
  }
  timer.running = !timer.running;
  if (timer.running) {
    startTicker();
    schedule();
  } else {
    stopTicker();
    cancel();
  }
}

export function resetTimer(): void {
  cancel();
  stopTicker();
  timer.running = false;
  timer.finished = false;
  timer.remaining = timer.total;
}

export function presetTimer(minutes: number): void {
  cancel();
  stopTicker();
  timer.total = minutes * 60;
  timer.remaining = timer.total;
  timer.running = false;
  timer.finished = false;
}

export function addFiveMinutes(): void {
  timer.total += 5 * 60;
  timer.remaining += 5 * 60;
  timer.finished = false;
  if (timer.running) schedule();
}

export function againTimer(): void {
  timer.remaining = timer.total;
  timer.finished = false;
  timer.running = true;
  startTicker();
  schedule();
}

export function markTimerFinished(): void {
  timer.remaining = 0;
  timer.finished = true;
  timer.running = false;
  stopTicker();
}

export function timerVisible(): boolean {
  return timer.running || timer.finished || timer.remaining !== timer.total;
}

export function timerLabel(): string {
  return `${String(Math.floor(timer.remaining / 60)).padStart(2, "0")}:${String(
    timer.remaining % 60,
  ).padStart(2, "0")}`;
}
