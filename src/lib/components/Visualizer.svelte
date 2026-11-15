<script lang="ts">
  import { audio, media } from "../media.svelte";

  interface Props {
    variant?: string;
    compact?: boolean;
  }

  let { variant = "auto", compact = false }: Props = $props();

  const AUTO = [
    "bars",
    "wave",
    "pulse",
    "orbit",
    "sparks",
    "aurora",
    "meteors",
    "glow",
    "ribbon",
  ];

  const resolved = $derived.by(() => {
    if (variant !== "auto") return variant;
    const key = `${media.current.title}|${media.current.artist}`;
    let hash = 0;
    for (let i = 0; i < key.length; i += 1) hash = (hash * 31 + key.charCodeAt(i)) | 0;
    return AUTO[Math.abs(hash) % AUTO.length];
  });

  const values = $derived(
    audio.bands.length
      ? audio.bands
      : Array.from({ length: compact ? 8 : 16 }, () => 0.06),
  );

  const bars = $derived(
    (compact ? values.slice(0, 8) : values).map((value) => Math.max(4, Math.round(value * 100))),
  );

  const level = $derived(Math.max(0, Math.min(1, audio.level)));

  const wavePath = $derived.by(() => {
    const list = compact ? values.slice(0, 10) : values;
    if (!list.length) return "M 0 24 L 100 24";
    const points = list.map((value, index) => {
      const x = (index / Math.max(1, list.length - 1)) * 100;
      const y = 25 - Math.max(0.04, value) * 20;
      return `${x.toFixed(1)} ${y.toFixed(1)}`;
    });
    let path = `M ${points[0]}`;
    for (let i = 1; i < points.length; i += 1) {
      const [px, py] = points[i - 1].split(" ").map(Number);
      const [cx, cy] = points[i].split(" ").map(Number);
      const mx = (px + cx) / 2;
      path += ` Q ${px} ${py} ${mx} ${(py + cy) / 2} T ${cx} ${cy}`;
    }
    return path;
  });

  const ribbonArea = $derived.by(() => {
    const list = compact ? values.slice(0, 10) : values;
    if (!list.length) return "";
    const points = list.map((value, index) => {
      const x = (index / Math.max(1, list.length - 1)) * 100;
      const y = 30 - Math.max(0.05, value) * 22;
      return [x, y];
    });
    let path = `M 0 30 L ${points[0][0].toFixed(1)} ${points[0][1].toFixed(1)}`;
    for (let i = 1; i < points.length; i += 1) {
      const [px, py] = points[i - 1];
      const [cx, cy] = points[i];
      const mx = (px + cx) / 2;
      path += ` Q ${px.toFixed(1)} ${py.toFixed(1)} ${mx.toFixed(1)} ${((py + cy) / 2).toFixed(1)} T ${cx.toFixed(1)} ${cy.toFixed(1)}`;
    }
    return `${path} L 100 30 Z`;
  });

  const sparks = $derived(
    values.slice(0, compact ? 7 : 14).map((value, index) => ({
      height: Math.max(6, Math.round(value * 100)),
      delay: (index * 0.13).toFixed(2),
    })),
  );

  const blobs = $derived(
    [values[1] ?? 0.1, values[5] ?? 0.1, values[9] ?? 0.1]
      .map((value) => Math.max(0.12, Math.min(1, value)))
      .map((value, index) => ({
        scale: 0.55 + value * 0.9,
        height: 40 + value * 60,
        delay: (index * 0.9).toFixed(2),
        x: [12, 50, 84][index],
      })),
  );

  const meteors = $derived(
    Array.from({ length: compact ? 3 : 6 }, (_, index) => ({
      delay: (index * 0.42).toFixed(2),
      top: [18, 62, 36, 80, 26, 54][index],
      length: 26 + ((values[index] ?? 0.2) * 60),
    })),
  );
</script>

<div
  class="visualizer"
  class:compact
  class:bars={resolved === "bars"}
  class:wave={resolved === "wave"}
  class:pulse={resolved === "pulse"}
  class:orbit={resolved === "orbit"}
  class:sparks={resolved === "sparks"}
  class:aurora={resolved === "aurora"}
  class:meteors={resolved === "meteors"}
  class:glow={resolved === "glow"}
  class:ribbon={resolved === "ribbon"}
  style:--viz-level={level}
  aria-hidden="true"
>
  {#if resolved === "bars"}
    {#each bars as height, index (index)}
      <span class="bar" style:height={`${height}%`}></span>
    {/each}
  {:else if resolved === "wave"}
    <svg viewBox="0 0 100 30" preserveAspectRatio="none">
      <path class="wave-glow" d={wavePath} />
      <path class="wave-line" d={wavePath} />
    </svg>
  {:else if resolved === "pulse"}
    <span class="ring ring-1"></span>
    <span class="ring ring-2"></span>
    <span class="ring ring-3"></span>
    <span class="pulse-core"></span>
  {:else if resolved === "orbit"}
    <span class="orbit-core"></span>
    {#each Array.from({ length: 8 }, (_, index) => index) as index (index)}
      <span
        class="orbit-dot"
        style:--angle={`${index * 45}deg`}
        style:--dot-scale={0.65 + (values[index % values.length] ?? 0.1) * 0.9}
      ></span>
    {/each}
  {:else if resolved === "sparks"}
    {#each sparks as spark, index (index)}
      <span
        class="spark"
        style:height={`${spark.height}%`}
        style:animation-delay={`${spark.delay}s`}
      ></span>
    {/each}
  {:else if resolved === "aurora"}
    {#each blobs as blob, index (index)}
      <span
        class="aurora-blob"
        style:left={`${blob.x}%`}
        style:--blob-scale={blob.scale}
        style:--blob-height={blob.height}
        style:animation-delay={`${blob.delay}s`}
      ></span>
    {/each}
    <span class="aurora-haze"></span>
  {:else if resolved === "meteors"}
    {#each meteors as meteor, index (index)}
      <span
        class="meteor"
        style:top={`${meteor.top}%`}
        style:--meteor-length={`${meteor.length}px`}
        style:animation-delay={`${meteor.delay}s`}
      ></span>
    {/each}
  {:else if resolved === "glow"}
    <span class="glow-ray"></span>
    <span class="glow-core"></span>
    <span class="glow-spot"></span>
  {:else}
    <svg viewBox="0 0 100 30" preserveAspectRatio="none">
      <defs>
        <linearGradient id="ribbon-fill" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stop-color="var(--live-accent, var(--accent))" stop-opacity="0.55" />
          <stop offset="100%" stop-color="var(--live-accent, var(--accent))" stop-opacity="0" />
        </linearGradient>
      </defs>
      <path class="ribbon-fill" d={ribbonArea} fill="url(#ribbon-fill)" />
      <path class="ribbon-line" d={wavePath} />
    </svg>
  {/if}
</div>

<style>
  .visualizer {
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    gap: 2px;
    min-height: 0;
    overflow: hidden;
    pointer-events: none;
  }

  .bar {
    flex: 1;
    min-width: 1px;
    min-height: 3px;
    border-radius: 3px 3px 1px 1px;
    background: linear-gradient(
      to top,
      var(--live-accent, var(--accent)),
      color-mix(in srgb, var(--live-accent, var(--accent)) 40%, #93c5fd)
    );
    transition: height 90ms linear;
    opacity: 0.9;
  }

  .wave svg,
  .ribbon svg {
    width: 100%;
    height: 100%;
    overflow: visible;
  }

  .wave-line,
  .wave-glow,
  .ribbon-line {
    fill: none;
    stroke: var(--live-accent, var(--accent));
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
    vector-effect: non-scaling-stroke;
  }

  .wave-glow {
    stroke-width: 7;
    opacity: 0.18;
    filter: blur(3px);
  }

  .ribbon-line {
    stroke-width: 2;
    opacity: 1;
  }

  .ribbon-fill {
    opacity: 0.85;
  }

  .pulse .ring {
    position: absolute;
    left: 50%;
    top: 50%;
    height: 80%;
    width: auto;
    aspect-ratio: 1;
    border-radius: 50%;
    border: 1.5px solid var(--live-accent, var(--accent));
    transform: translate(-50%, -50%) scale(calc(0.7 + var(--viz-level) * 1.1));
    opacity: calc(0.2 + var(--viz-level) * 0.65);
    animation: pulse-ring 1.8s ease-in-out infinite;
  }

  .compact.pulse .ring {
    width: 14px;
    height: 14px;
  }

  .pulse .ring-2 {
    animation-delay: -0.6s;
    border-width: 1px;
  }

  .pulse .ring-3 {
    animation-delay: -1.2s;
    border-width: 0.75px;
  }

  .pulse-core {
    height: 30%;
    width: auto;
    aspect-ratio: 1;
    border-radius: 50%;
    background: radial-gradient(circle, var(--live-accent, var(--accent)), transparent 72%);
    box-shadow: 0 0 18px color-mix(in srgb, var(--live-accent, var(--accent)) 65%, transparent);
    transform: scale(calc(0.8 + var(--viz-level) * 0.8));
    transition: transform 90ms linear;
  }

  .compact .pulse-core {
    height: 34%;
    width: auto;
  }

  @keyframes pulse-ring {
    0%, 100% { opacity: 0.25; }
    50% { opacity: 0.8; }
  }

  .orbit-core {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--live-accent, var(--accent));
    box-shadow: 0 0 10px var(--live-accent, var(--accent));
    transform: translate(-50%, -50%);
  }

  .orbit-dot {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--live-accent, var(--accent));
    transform: translate(-50%, -50%) rotate(var(--angle)) translateX(17px) scale(var(--dot-scale));
    opacity: 0.85;
    transition: transform 90ms linear;
  }

  .compact .orbit-dot {
    width: 3px;
    height: 3px;
    transform: translate(-50%, -50%) rotate(var(--angle)) translateX(11px) scale(var(--dot-scale));
  }

  .sparks {
    align-items: flex-end;
    gap: 4px;
    padding: 0 4px;
  }

  .spark {
    width: 3px;
    min-height: 3px;
    border-radius: 999px;
    background: linear-gradient(to top, var(--live-accent, var(--accent)), transparent);
    opacity: 0.85;
    animation: spark-breathe 1.1s ease-in-out infinite alternate;
    transition: height 100ms linear;
  }

  @keyframes spark-breathe {
    from { opacity: 0.3; transform: scaleY(0.65); }
    to { opacity: 0.95; transform: scaleY(1.08); }
  }

  /* Aurora: three blurred colour columns breathing with the low bands. */
  .aurora {
    align-items: stretch;
  }

  .aurora-blob {
    position: absolute;
    bottom: -25%;
    width: 42%;
    height: var(--blob-height, 60%);
    border-radius: 50%;
    background: radial-gradient(
      circle at 50% 100%,
      var(--live-accent, var(--accent)),
      transparent 72%
    );
    filter: blur(7px);
    opacity: 0.62;
    transform: translateX(-50%) scaleY(var(--blob-scale, 1));
    transform-origin: 50% 100%;
    transition: transform 120ms linear, height 160ms linear;
    animation: aurora-drift 5.5s ease-in-out infinite alternate;
  }

  .aurora-haze {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      to top,
      color-mix(in srgb, var(--live-accent, var(--accent)) 18%, transparent),
      transparent
    );
    opacity: 0.5;
  }

  @keyframes aurora-drift {
    from { transform: translateX(-50%) translateY(6%) scaleY(calc(var(--blob-scale, 1) * 0.85)); }
    to { transform: translateX(-50%) translateY(-6%) scaleY(calc(var(--blob-scale, 1) * 1.1)); }
  }

  /* Meteors: diagonal streaks that fire faster with the beat. */
  .meteors {
    align-items: stretch;
  }

  .meteor {
    position: absolute;
    left: -30%;
    width: var(--meteor-length, 40px);
    height: 2px;
    border-radius: 999px;
    background: linear-gradient(
      to right,
      transparent,
      color-mix(in srgb, var(--live-accent, var(--accent)) 90%, #fff),
      transparent
    );
    filter: drop-shadow(0 0 6px color-mix(in srgb, var(--live-accent, var(--accent)) 80%, transparent));
    opacity: 0.95;
    animation: meteor-fall 2.4s linear infinite;
    animation-duration: calc(2.8s - var(--viz-level) * 1.4s);
  }

  @keyframes meteor-fall {
    from { transform: translate3d(0, 0, 0) rotate(18deg); opacity: 0; }
    12% { opacity: 0.9; }
    to { transform: translate3d(160%, 260%, 0) rotate(18deg); opacity: 0; }
  }

  /* Glow: a spotlight that widens with the audio level. */
  .glow {
    align-items: center;
  }

  .glow-spot {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 120%;
    aspect-ratio: 1;
    border-radius: 50%;
    background: radial-gradient(
      circle,
      color-mix(in srgb, var(--live-accent, var(--accent)) 60%, transparent),
      transparent 62%
    );
    transform: translate(-50%, -50%) scale(calc(0.35 + var(--viz-level) * 0.75));
    opacity: calc(0.25 + var(--viz-level) * 0.5);
    filter: blur(6px);
    transition: transform 90ms linear, opacity 90ms linear;
  }

  .glow-core {
    width: 22%;
    height: 40%;
    max-width: 22px;
    border-radius: 50%;
    background: linear-gradient(
      to bottom,
      color-mix(in srgb, var(--live-accent, var(--accent)) 85%, #fff),
      transparent
    );
    box-shadow: 0 0 22px color-mix(in srgb, var(--live-accent, var(--accent)) 80%, transparent);
    transform: scaleY(calc(0.5 + var(--viz-level)));
    transition: transform 90ms linear;
  }

  .glow-ray {
    position: absolute;
    inset: -30% -10%;
    background: conic-gradient(
      from 90deg,
      transparent 0deg,
      color-mix(in srgb, var(--live-accent, var(--accent)) 22%, transparent) 18deg,
      transparent 40deg,
      transparent 180deg,
      color-mix(in srgb, var(--live-accent, var(--accent)) 18%, transparent) 205deg,
      transparent 230deg
    );
    opacity: 0.6;
    animation: glow-sweep 7s linear infinite;
  }

  @keyframes glow-sweep {
    to { transform: rotate(360deg); }
  }

  .compact {
    gap: 1.5px;
  }

  .compact .bar {
    border-radius: 1px;
  }

  .compact .meteor {
    height: 1px;
  }

  :global(:root[data-reduced-motion="true"]) .spark,
  :global(:root[data-reduced-motion="true"]) .ring,
  :global(:root[data-reduced-motion="true"]) .aurora-blob,
  :global(:root[data-reduced-motion="true"]) .meteor,
  :global(:root[data-reduced-motion="true"]) .glow-ray {
    animation: none;
  }

  :global(:root[data-reduced-motion="true"]) .bar,
  :global(:root[data-reduced-motion="true"]) .spark,
  :global(:root[data-reduced-motion="true"]) .glow-spot,
  :global(:root[data-reduced-motion="true"]) .glow-core,
  :global(:root[data-reduced-motion="true"]) .aurora-blob {
    transition: none;
  }
</style>
