<script lang="ts">
  interface Props {
    value: number;
    label: string;
    disabled?: boolean;
    onInput?: (value: number) => void;
    onCommit?: (value: number) => void;
  }

  let { value, label, disabled = false, onInput, onCommit }: Props = $props();

  let dragging = $state(false);
  const clamped = $derived(Math.max(0, Math.min(1, value)));

  function read(event: Event): number {
    return Number((event.currentTarget as HTMLInputElement).valueAsNumber) / 1000;
  }

  function input(event: Event): void {
    if (disabled) return;
    dragging = true;
    onInput?.(read(event));
  }

  function grab(event: PointerEvent): void {
    if (disabled) return;
    try {
      (event.currentTarget as HTMLInputElement).setPointerCapture(event.pointerId);
    } catch {
      /* pointer capture is optional */
    }
  }

  function commit(event: Event): void {
    // pointerup and change both fire on release; only the first one may seek.
    if (disabled || !dragging) return;
    dragging = false;
    onCommit?.(read(event));
  }
</script>

<div class="slider" class:dragging class:disabled>
  <div class="rail" aria-hidden="true">
    <span class="fill" style:transform={`scaleX(${clamped})`}></span>
  </div>
  <span class="thumb" style:left={`${clamped * 100}%`} aria-hidden="true"></span>
  <input
    type="range"
    min="0"
    max="1000"
    step="1"
    value={Math.round(clamped * 1000)}
    {disabled}
    aria-label={label}
    onpointerdown={grab}
    oninput={input}
    onchange={commit}
    onpointerup={commit}
    onpointercancel={commit}
  />
</div>

<style>
  .slider {
    position: relative;
    display: flex;
    align-items: center;
    flex: 1;
    min-width: 0;
    height: 18px;
    touch-action: none;
  }

  .rail {
    position: absolute;
    left: 0;
    right: 0;
    top: 50%;
    height: 4px;
    margin-top: -2px;
    border-radius: 999px;
    background: var(--surface-strong);
    overflow: hidden;
    transition: height 0.22s cubic-bezier(0.22, 1, 0.36, 1), margin-top 0.22s cubic-bezier(0.22, 1, 0.36, 1);
  }

  .slider:hover .rail,
  .slider.dragging .rail {
    height: 5px;
    margin-top: -2.5px;
  }

  .fill {
    display: block;
    width: 100%;
    height: 100%;
    transform-origin: left center;
    border-radius: inherit;
    background: linear-gradient(
      90deg,
      color-mix(in srgb, var(--live-accent) 78%, var(--accent)),
      var(--live-accent)
    );
    box-shadow: 0 0 10px color-mix(in srgb, var(--live-accent) 45%, transparent);
    /* One frame of media sampling is ~120 ms; a matching linear glide keeps
       the bar continuous instead of stepping. */
    transition: transform 0.18s linear;
    will-change: transform;
  }

  .thumb {
    position: absolute;
    top: 50%;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #fff;
    border: 1px solid color-mix(in srgb, var(--live-accent) 55%, transparent);
    box-shadow:
      0 0 0 3px color-mix(in srgb, var(--live-accent) 12%, transparent),
      0 2px 8px rgba(0, 0, 0, 0.4);
    transform: translate(-50%, -50%);
    pointer-events: none;
    transition:
      left 0.18s linear,
      transform 0.24s cubic-bezier(0.34, 1.4, 0.64, 1),
      box-shadow 0.24s ease;
  }

  .slider:hover .thumb,
  .slider:focus-within .thumb {
    transform: translate(-50%, -50%) scale(1.18);
    box-shadow:
      0 0 0 5px color-mix(in srgb, var(--live-accent) 16%, transparent),
      0 3px 10px rgba(0, 0, 0, 0.45);
  }

  .slider.dragging .thumb {
    transform: translate(-50%, -50%) scale(1.25);
  }

  .slider.dragging .fill,
  .slider.dragging .thumb {
    transition: none;
  }

  input {
    position: absolute;
    inset: -3px 0;
    width: 100%;
    height: 24px;
    margin: 0;
    opacity: 0;
    cursor: pointer;
    accent-color: var(--live-accent);
  }

  .slider.disabled {
    opacity: 0.45;
    pointer-events: none;
  }

  :global(:root[data-reduced-motion="true"]) .fill,
  :global(:root[data-reduced-motion="true"]) .thumb {
    transition: none;
  }
</style>
