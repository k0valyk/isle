<script lang="ts">
  interface Props {
    values: number[];
    max?: number;
    strokeWidth?: number;
    fill?: boolean;
  }

  let { values, max = 36, strokeWidth = 1.6, fill = true }: Props = $props();

  const clamp = (value: number): number =>
    Number.isFinite(value) ? Math.max(0, Math.min(100, value)) : 0;

  let shown = $state<number[]>([]);
  let animation: number | undefined;
  const gradientId = `spark-${Math.random().toString(36).slice(2, 9)}`;

  function reduced(): boolean {
    return document.documentElement.dataset.reducedMotion === "true";
  }

  function align(previous: number[], next: number[]): number[] {
    if (previous.length === next.length) return previous.slice();
    if (previous.length > next.length) return previous.slice(previous.length - next.length);
    return [...Array(next.length - previous.length).fill(previous[0] ?? next[0]), ...previous];
  }

  $effect(() => {
    const next = values.slice(-max).map(clamp);
    if (!next.length) {
      shown = [];
      return;
    }
    if (!shown.length || reduced()) {
      shown = next;
      return;
    }
    const from = align(shown, next);
    const start = performance.now();
    const duration = 420;
    if (animation) cancelAnimationFrame(animation);
    const step = (now: number): void => {
      const t = Math.min(1, (now - start) / duration);
      const eased = 1 - Math.pow(1 - t, 3);
      shown = next.map((value, index) => from[index] + (value - from[index]) * eased);
      if (t < 1) animation = requestAnimationFrame(step);
    };
    animation = requestAnimationFrame(step);
    return () => {
      if (animation) cancelAnimationFrame(animation);
    };
  });

  const points = $derived.by(() => {
    const list = shown.length ? shown : values.slice(-max).map(clamp);
    if (!list.length) return [];
    const stepX = 100 / Math.max(1, list.length - 1);
    return list.map((value, index) => ({ x: index * stepX, y: 100 - value }));
  });

  const line = $derived.by(() => {
    const list = points;
    if (list.length < 2) return "";
    const fmt = (value: number): string => (Math.round(value * 100) / 100).toString();
    let path = `M ${fmt(list[0].x)} ${fmt(list[0].y)}`;
    for (let index = 0; index < list.length - 1; index += 1) {
      const p0 = list[index - 1] ?? list[index];
      const p1 = list[index];
      const p2 = list[index + 1];
      const p3 = list[index + 2] ?? p2;
      const c1x = p1.x + (p2.x - p0.x) / 6;
      const c1y = p1.y + (p2.y - p0.y) / 6;
      const c2x = p2.x - (p3.x - p1.x) / 6;
      const c2y = p2.y - (p3.y - p1.y) / 6;
      path += ` C ${fmt(c1x)} ${fmt(c1y)}, ${fmt(c2x)} ${fmt(c2y)}, ${fmt(p2.x)} ${fmt(p2.y)}`;
    }
    return path;
  });

  const area = $derived(line ? `${line} L 100 100 L 0 100 Z` : "");
</script>

<svg class="sparkline" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
  {#if fill && area}
    <defs>
      <linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="var(--live-accent, var(--accent))" stop-opacity="0.26" />
        <stop offset="100%" stop-color="var(--live-accent, var(--accent))" stop-opacity="0" />
      </linearGradient>
    </defs>
    <path class="area" d={area} fill={`url(#${gradientId})`} />
  {/if}
  {#if line}
    <path class="line" d={line} style:stroke-width={strokeWidth} />
  {/if}
</svg>

<style>
  .sparkline {
    display: block;
    width: 100%;
    height: 100%;
    overflow: visible;
  }

  .line {
    fill: none;
    stroke: var(--live-accent, var(--accent));
    stroke-linecap: round;
    stroke-linejoin: round;
    vector-effect: non-scaling-stroke;
    filter: drop-shadow(0 0 3px color-mix(in srgb, var(--live-accent, var(--accent)) 55%, transparent));
  }

  .area {
    stroke: none;
  }
</style>
