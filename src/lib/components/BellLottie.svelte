<script lang="ts">
  import { onMount } from "svelte";

  interface Props {
    size?: number;
  }

  let { size = 44 }: Props = $props();
  let host: HTMLDivElement;

  // A small hand-authored Lottie animation (gold bell + pulsing ring) so the
  // player is only downloaded the first time a timer actually finishes.
  const bell = {
    v: "5.7.4",
    fr: 30,
    ip: 0,
    op: 60,
    w: 120,
    h: 120,
    nm: "timer-bell",
    ddd: 0,
    assets: [],
    layers: [
      {
        ddd: 0,
        ind: 1,
        ty: 4,
        nm: "ring",
        sr: 1,
        ks: {
          o: { a: 0, k: 100 },
          r: { a: 0, k: 0 },
          p: { a: 0, k: [60, 60] },
          a: { a: 0, k: [0, 0] },
          s: {
            a: 1,
            k: [
              { t: 0, s: [88, 88, 100] },
              { t: 18, s: [112, 112, 100] },
              { t: 36, s: [88, 88, 100] },
              { t: 60, s: [88, 88, 100] },
            ],
          },
        },
        ao: 0,
        shapes: [
          {
            ty: "gr",
            it: [
              { ty: "el", p: { a: 0, k: [0, 0] }, s: { a: 0, k: [86, 86] }, d: 1, nm: "ring-shape" },
              {
                ty: "st",
                c: { a: 0, k: [0.98, 0.66, 0.22, 1] },
                o: { a: 0, k: 80 },
                w: { a: 0, k: 5 },
                lc: 2,
                lj: 1,
                nm: "ring-stroke",
              },
              {
                ty: "tr",
                p: { a: 0, k: [0, 0] },
                a: { a: 0, k: [0, 0] },
                s: { a: 0, k: [100, 100] },
                r: { a: 0, k: 0 },
                o: { a: 0, k: 100 },
                sk: { a: 0, k: 0 },
                sa: { a: 0, k: 0 },
              },
            ],
            nm: "ring-group",
          },
        ],
        ip: 0,
        op: 60,
        st: 0,
        bm: 0,
      },
      {
        ddd: 0,
        ind: 2,
        ty: 4,
        nm: "bell",
        sr: 1,
        ks: {
          o: { a: 0, k: 100 },
          r: {
            a: 1,
            k: [
              { t: 0, s: [-12] },
              { t: 15, s: [12] },
              { t: 30, s: [-12] },
              { t: 45, s: [12] },
              { t: 60, s: [-12] },
            ],
          },
          p: { a: 0, k: [60, 54] },
          a: { a: 0, k: [0, 0] },
          s: { a: 0, k: [100, 100] },
        },
        ao: 0,
        shapes: [
          {
            ty: "gr",
            it: [
              { ty: "el", p: { a: 0, k: [0, -6] }, s: { a: 0, k: [42, 46] }, d: 1, nm: "dome" },
              { ty: "fl", c: { a: 0, k: [1, 0.78, 0.3, 1] }, o: { a: 0, k: 100 }, r: 1, nm: "fill" },
              {
                ty: "tr",
                p: { a: 0, k: [0, 0] },
                a: { a: 0, k: [0, 0] },
                s: { a: 0, k: [100, 100] },
                r: { a: 0, k: 0 },
                o: { a: 0, k: 100 },
                sk: { a: 0, k: 0 },
                sa: { a: 0, k: 0 },
              },
            ],
            nm: "dome-group",
          },
          {
            ty: "gr",
            it: [
              {
                ty: "rc",
                p: { a: 0, k: [0, 18] },
                s: { a: 0, k: [50, 9] },
                r: { a: 0, k: 4 },
                d: 1,
                nm: "base",
              },
              { ty: "fl", c: { a: 0, k: [1, 0.78, 0.3, 1] }, o: { a: 0, k: 100 }, r: 1, nm: "fill2" },
              {
                ty: "tr",
                p: { a: 0, k: [0, 0] },
                a: { a: 0, k: [0, 0] },
                s: { a: 0, k: [100, 100] },
                r: { a: 0, k: 0 },
                o: { a: 0, k: 100 },
                sk: { a: 0, k: 0 },
                sa: { a: 0, k: 0 },
              },
            ],
            nm: "base-group",
          },
        ],
        ip: 0,
        op: 60,
        st: 0,
        bm: 0,
      },
    ],
  };

  onMount(() => {
    let animation: { destroy: () => void; goToAndStop: (v: number, f?: boolean) => void } | null =
      null;
    let disposed = false;
    const reduced =
      document.documentElement.dataset.reducedMotion === "true" ||
      window.matchMedia("(prefers-reduced-motion: reduce)").matches;

    void (async () => {
      const module = await import("lottie-web/build/player/esm/lottie_light.min.js");
      if (disposed) return;
      animation = module.default.loadAnimation({
        container: host,
        renderer: "svg",
        loop: true,
        autoplay: !reduced,
        animationData: bell,
      });
      if (reduced) animation.goToAndStop(18, true);
    })().catch(() => {
      /* the static fallback icon stays visible if Lottie cannot load */
    });

    return () => {
      disposed = true;
      animation?.destroy();
    };
  });
</script>

<div
  class="bell-lottie"
  bind:this={host}
  style:width={`${size}px`}
  style:height={`${size}px`}
  aria-hidden="true"
></div>

<style>
  .bell-lottie {
    display: block;
    flex: none;
    filter: drop-shadow(0 0 10px rgba(251, 191, 36, 0.35));
  }

  :global(:root[data-reduced-motion="true"]) .bell-lottie {
    filter: none;
  }
</style>
