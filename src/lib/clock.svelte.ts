export const clock = $state<{ now: Date }>({ now: new Date() });

let started = false;

export function initClock(): void {
  if (started) return;
  started = true;
  const tick = (): void => {
    clock.now = new Date();
    const delay = 1000 - (Date.now() % 1000) + 8;
    setTimeout(tick, delay);
  };
  tick();
}
