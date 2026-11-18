<script lang="ts">
  interface Props {
    scene: string;
    gifUrl: string;
    expanded: boolean;
  }

  let { scene, gifUrl, expanded }: Props = $props();

  let gifBroken = $state(false);
  $effect(() => {
    void gifUrl;
    gifBroken = false;
  });
</script>

{#if scene !== "none"}
  <div
    class="scene"
    class:expanded
    class:has-gif={scene === "gif"}
    data-scene={scene}
    aria-hidden="true"
  >
    {#if scene === "stars"}
      {#each Array.from({ length: 26 }, (_, index) => index) as index (index)}
        <span
          class="star"
          class:bright={index % 5 === 0}
          style:left={`${(index * 37 + 7) % 100}%`}
          style:top={`${(index * 53 + 11) % 100}%`}
          style:animation-delay={`${(index * 0.43) % 4}s`}
          style:animation-duration={`${2.4 + (index % 5) * 0.9}s`}
        ></span>
      {/each}
      <span class="shooting"></span>
    {:else if scene === "aurora"}
      <span class="aurora a"></span>
      <span class="aurora b"></span>
      <span class="aurora c"></span>
    {:else if scene === "bubbles"}
      {#each Array.from({ length: 14 }, (_, index) => index) as index (index)}
        <span
          class="bubble"
          style:left={`${(index * 41 + 9) % 96}%`}
          style:animation-delay={`${(index * 0.9) % 6}s`}
          style:animation-duration={`${5 + (index % 4) * 1.7}s`}
          style:--size={`${4 + (index % 4) * 2}px`}
        ></span>
      {/each}
    {:else if scene === "matrix"}
      {#each Array.from({ length: 12 }, (_, index) => index) as index (index)}
        <span
          class="column"
          style:left={`${(index * 17 + 4) % 100}%`}
          style:animation-delay={`${(index * 0.62) % 5}s`}
          style:animation-duration={`${3.4 + (index % 4) * 1.1}s`}
        ></span>
      {/each}
    {:else if scene === "fps" && expanded}
      <!-- Retired: live load data belongs to the System tab. Kept as a no-op so
           an older settings payload renders nothing instead of a floating HUD. -->
    {:else if scene === "gif" && gifUrl && !gifBroken}
      <img
        class="gif"
        src={gifUrl}
        alt=""
        loading="lazy"
        onerror={() => (gifBroken = true)}
      />
      <span class="scrim"></span>
    {/if}
  </div>
{/if}

<style>
  .scene {
    position: absolute;
    inset: 0;
    z-index: 0;
    overflow: hidden;
    pointer-events: none;
    --scene-opacity: 0.55;
    opacity: var(--scene-opacity);
    border-radius: inherit;
    mask-image: linear-gradient(to bottom, transparent, #000 14%, #000 86%, transparent);
    animation: scene-in 0.55s cubic-bezier(0.22, 1, 0.36, 1);
  }

  @keyframes scene-in {
    from {
      opacity: 0;
      transform: scale(1.025);
    }
    to {
      opacity: var(--scene-opacity);
      transform: scale(1);
    }
  }

  .scene.expanded {
    --scene-opacity: 0.75;
  }

  .scene:not(.expanded) {
    mask-image: none;
    --scene-opacity: 0.4;
  }

  .scene:not(.expanded).has-gif {
    --scene-opacity: 0.3;
  }

  .scene.has-gif {
    mask-image: none;
  }

  /* Stars */
  .star {
    position: absolute;
    width: 2px;
    height: 2px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--live-accent) 82%, #fff);
    box-shadow: 0 0 4px color-mix(in srgb, var(--live-accent) 70%, transparent);
    opacity: 0;
    animation: twinkle ease-in-out infinite;
  }

  .star.bright {
    width: 3px;
    height: 3px;
    background: #fff;
  }

  .shooting {
    position: absolute;
    top: 18%;
    left: -20%;
    width: 42%;
    height: 1px;
    background: linear-gradient(90deg, transparent, #fff);
    opacity: 0;
    transform: rotate(16deg);
    animation: shoot 7s ease-in infinite;
  }

  @keyframes twinkle {
    0%,
    100% {
      opacity: 0.08;
      transform: scale(0.7);
    }
    50% {
      opacity: 0.9;
      transform: scale(1.15);
    }
  }

  @keyframes shoot {
    0%,
    78% {
      opacity: 0;
      transform: translateX(0) rotate(16deg);
    }
    82% {
      opacity: 0.8;
    }
    92%,
    100% {
      opacity: 0;
      transform: translateX(320%) rotate(16deg);
    }
  }

  /* Aurora */
  .aurora {
    position: absolute;
    width: 70%;
    height: 90%;
    border-radius: 50%;
    filter: blur(22px);
    opacity: 0.5;
    mix-blend-mode: screen;
    animation: drift 11s ease-in-out infinite alternate;
  }

  .aurora.a {
    left: -18%;
    top: -30%;
    background: radial-gradient(circle, #22d3ee, transparent 68%);
  }

  .aurora.b {
    right: -22%;
    top: -10%;
    background: radial-gradient(circle, #a78bfa, transparent 68%);
    animation-delay: -3.5s;
  }

  .aurora.c {
    left: 20%;
    bottom: -50%;
    background: radial-gradient(circle, #34d399, transparent 66%);
    animation-delay: -6.5s;
  }

  @keyframes drift {
    from {
      transform: translate3d(-6%, -2%, 0) scale(0.9);
    }
    to {
      transform: translate3d(8%, 5%, 0) scale(1.18);
    }
  }

  /* Bubbles */
  .bubble {
    position: absolute;
    bottom: -8%;
    width: var(--size, 6px);
    height: var(--size, 6px);
    border-radius: 50%;
    border: 1px solid color-mix(in srgb, var(--live-accent) 70%, #fff);
    background: color-mix(in srgb, var(--live-accent) 22%, transparent);
    opacity: 0;
    animation: rise ease-in infinite;
  }

  @keyframes rise {
    0% {
      opacity: 0;
      transform: translateY(0) scale(0.6);
    }
    16% {
      opacity: 0.85;
    }
    100% {
      opacity: 0;
      transform: translateY(-360px) translateX(14px) scale(1.25);
    }
  }

  /* Matrix columns */
  .column {
    position: absolute;
    top: -60%;
    width: 2px;
    height: 60%;
    border-radius: 2px;
    background: linear-gradient(to bottom, transparent, var(--live-accent), transparent);
    opacity: 0;
    animation: fall linear infinite;
    filter: drop-shadow(0 0 4px var(--live-accent));
  }

  @keyframes fall {
    0% {
      opacity: 0;
      transform: translateY(0);
    }
    12% {
      opacity: 0.65;
    }
    100% {
      opacity: 0;
      transform: translateY(260%);
    }
  }

  /* GIF */
  .gif {
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0.42;
    filter: saturate(1.1);
  }

  .scrim {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      to bottom,
      color-mix(in srgb, var(--bg) 35%, transparent),
      transparent 35%,
      color-mix(in srgb, var(--bg) 45%, transparent)
    );
  }

  :global(:root[data-reduced-motion="true"]) .scene {
    display: none;
  }
</style>
