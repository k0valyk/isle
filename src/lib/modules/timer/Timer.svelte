<script lang="ts">
  import Icon from "../../components/Icon.svelte";
  import BellLottie from "../../components/BellLottie.svelte";
  import { t } from "../../store.svelte";
  import {
    addFiveMinutes as addFive,
    againTimer as again,
    presetTimer as preset,
    resetTimer as reset,
    timer,
    timerLabel,
    toggleTimer as toggle,
  } from "../../timer.svelte";

  const PRESETS = [5, 10, 15, 25, 45];

  let customMinutes = $state(25);

  const total = $derived(timer.total);
  const remaining = $derived(timer.remaining);
  const running = $derived(timer.running);
  const finished = $derived(timer.finished);
  const progress = $derived(total > 0 ? remaining / total : 0);
  const clock = $derived(timerLabel());

  function setCustom(): void {
    const minutes = Math.max(1, Math.min(180, Math.round(Number(customMinutes) || 1)));
    customMinutes = minutes;
    preset(minutes);
  }
</script>

<div class="timer" class:finished class:running>
  <div class="dial" style:--progress={progress}>
    <svg viewBox="0 0 120 120" class="ring" aria-hidden="true">
      <circle class="track" cx="60" cy="60" r="52" />
      <circle
        class="value"
        cx="60"
        cy="60"
        r="52"
        style:stroke-dashoffset={`${(1 - progress) * 326.7}`}
      />
    </svg>
    <div class="center">
      {#if finished}
        <BellLottie size={38} />
      {/if}
      <span class="clock">{clock}</span>
      <span
        class="label {finished
          ? 'animate__animated animate__headShake animate__infinite animate__slower'
          : ''}"
      >
        {#if finished}
          {t("timer.finished")}
        {:else if running}
          {t("timer.continue")}
        {:else if remaining === total}
          {t("timer.ready")}
        {:else}
          {t("timer.pause")}
        {/if}
      </span>
    </div>
  </div>

  <div class="controls">
    {#if finished}
      <button class="btn primary" onclick={again}>{t("timer.again")}</button>
      <button class="btn" onclick={reset}>{t("timer.stop")}</button>
    {:else}
      <button class="btn primary" onclick={toggle}>
        <Icon name={running ? "pause" : "play"} size={14} />
        {running ? t("timer.pause") : t("timer.start")}
      </button>
      <button class="btn" onclick={reset}>{t("timer.reset")}</button>
      <button class="btn" onclick={addFive}>{t("timer.add5")}</button>
    {/if}
  </div>

  <div class="presets">
    {#each PRESETS as minutes}
      <button
        class="preset"
        class:active={total === minutes * 60}
        onclick={() => preset(minutes)}
      >
        {t("timer.minutes", { count: minutes })}
      </button>
    {/each}
  </div>

  <div class="custom">
    <input
      type="number"
      min="1"
      max="180"
      bind:value={customMinutes}
      aria-label={t("timer.custom")}
    />
    <button class="btn ghost" onclick={setCustom}>{t("timer.set")}</button>
  </div>

  {#if finished}
    <div class="finished-hint">{t("timer.finished_hint")}</div>
  {/if}
</div>

<style>
  .timer {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
  }

  .dial {
    position: relative;
    width: 168px;
    height: 168px;
  }

  .ring {
    width: 100%;
    height: 100%;
    transform: rotate(-90deg);
  }

  .track {
    fill: none;
    stroke: var(--surface-strong);
    stroke-width: 7;
  }

  .value {
    fill: none;
    stroke: var(--accent);
    stroke-width: 7;
    stroke-linecap: round;
    stroke-dasharray: 326.7;
    transition: stroke-dashoffset 0.6s linear;
  }

  .finished .value {
    stroke: var(--danger);
  }

  .running .value {
    filter: drop-shadow(0 0 5px color-mix(in srgb, var(--accent) 65%, transparent));
  }

  .running .dial::after {
    content: "";
    position: absolute;
    inset: 16px;
    border-radius: 50%;
    box-shadow: 0 0 26px color-mix(in srgb, var(--accent) 26%, transparent);
    animation: timer-breathe 3.4s ease-in-out infinite;
    pointer-events: none;
  }

  @keyframes timer-breathe {
    0%, 100% { opacity: 0.3; transform: scale(0.98); }
    50% { opacity: 0.85; transform: scale(1.02); }
  }

  .center {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
  }

  .clock {
    font-size: 38px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    letter-spacing: -1.2px;
  }

  .label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
  }

  .controls {
    display: flex;
    gap: 6px;
  }

  .presets {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
    justify-content: center;
  }

  .preset {
    border: 1px solid var(--border);
    background: transparent;
    color: var(--muted);
    border-radius: 999px;
    padding: 3px 9px;
    font-size: 10.5px;
    transition: background 0.14s ease, color 0.14s ease;
  }

  .preset:hover {
    background: var(--surface);
    color: var(--fg);
  }

  .preset.active {
    background: var(--surface-strong);
    color: var(--fg);
    border-color: var(--border);
  }

  .custom {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .custom input {
    width: 56px;
    height: 24px;
    padding: 2px 6px;
    text-align: center;
    font-size: 10.5px;
    font-variant-numeric: tabular-nums;
  }

  .custom .btn {
    padding: 3px 8px;
    min-height: 24px;
    font-size: 10px;
  }

  .finished-hint {
    font-size: 10.5px;
    color: var(--danger);
  }

  .finished .dial {
    animation: pulse 1.6s ease-in-out infinite;
  }

  @keyframes pulse {
    0%,
    100% {
      transform: scale(1);
    }
    50% {
      transform: scale(1.035);
    }
  }

  :global(:root[data-reduced-motion="true"]) .finished .dial {
    animation: none;
  }

  :global(:root[data-reduced-motion="true"]) .running .dial::after {
    animation: none;
    opacity: 0.4;
  }
</style>
