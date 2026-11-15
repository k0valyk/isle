<script lang="ts">
  import { animate } from "motion";

  interface Props {
    value?: number | null;
    decimals?: number;
    suffix?: string;
  }

  let { value = null, decimals = 0, suffix = "" }: Props = $props();

  let shown = $state(0);
  let current = 0;
  let primed = false;

  $effect(() => {
    const target = value;
    if (target === null || target === undefined || Number.isNaN(target)) return;
    if (
      !primed ||
      document.documentElement.dataset.reducedMotion === "true"
    ) {
      primed = true;
      current = target;
      shown = target;
      return;
    }
    const from = current;
    current = target;
    const controls = animate(from, target, {
      type: "spring",
      stiffness: 260,
      damping: 30,
      onUpdate: (next: number) => {
        shown = next;
      },
    });
    return () => controls.stop();
  });
</script>

<span class="animated-number" aria-hidden="true"
  >{value === null || value === undefined ? "—" : shown.toFixed(decimals)}{#if value !== null && value !== undefined}{suffix}{/if}</span
>

<style>
  .animated-number {
    font-variant-numeric: tabular-nums;
  }
</style>
