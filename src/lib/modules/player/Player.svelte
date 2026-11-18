<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../../components/Icon.svelte";
  import SmoothSlider from "../../components/SmoothSlider.svelte";
  import Visualizer from "../../components/Visualizer.svelte";
  import { store, t } from "../../store.svelte";
  import {
    activeLyricIndex,
    formatTrackTime,
    isPlaying,
    media,
    mediaControl,
    refreshVolume,
    seekMedia,
    setVolume,
    toggleMute,
    volume,
  } from "../../media.svelte";

  let dragging = $state(false);
  let dragValue = $state(0);
  let volumeDragging = $state(false);

  const volumePercent = $derived(Math.round(volume.level * 100));

  function volumeInput(next: number): void {
    volumeDragging = true;
    volume.level = next;
    volume.muted = next === 0 ? volume.muted : false;
  }

  function volumeCommit(next: number): void {
    volumeDragging = false;
    void setVolume(next);
  }

  onMount(() => {
    void refreshVolume();
    const timer = setInterval(() => {
      if (!volumeDragging) void refreshVolume();
    }, 6000);
    return () => clearInterval(timer);
  });

  const current = $derived(media.current);
  const duration = $derived(current.duration || 0);
  const shownPosition = $derived(dragging ? dragValue : media.position);
  const progress = $derived(duration > 0 ? (shownPosition / duration) * 100 : 0);
  const activeIndex = $derived(activeLyricIndex(media.position));
  const synced = $derived(media.lyrics);
  const LINE_HEIGHT = 26;
  const lyricsShift = $derived(-(Math.max(0, activeIndex) + 0.5) * LINE_HEIGHT);
  const lyricsStyle = $derived(store.settings.lyrics_style || "slide");
  const karaokeProgress = $derived.by(() => {
    const line = synced[activeIndex];
    if (!line) return 0;
    const next = synced[activeIndex + 1]?.time ?? line.time + 4;
    const span = Math.max(0.6, next - line.time);
    return Math.max(0, Math.min(1, (media.position - line.time) / span));
  });

  let artBroken = $state(false);
  $effect(() => {
    void current.artwork;
    artBroken = false;
  });

  function seekInput(next: number): void {
    dragging = true;
    dragValue = next * duration;
  }

  function seekCommit(next: number): void {
    dragging = false;
    if (duration <= 0) return;
    seekMedia(next * duration);
  }

  function seekBy(seconds: number): void {
    if (!current.active) return;
    seekMedia(media.position + seconds);
  }
</script>

<div class="player" class:empty={!current.active}>
  {#if current.active}
    {#if current.artwork}
      <div class="backdrop" style:background-image={`url(${current.artwork})`} aria-hidden="true"></div>
    {/if}
    {#key `${current.title}|${current.artist}`}
      <div class="top">
        <div class="art" aria-hidden="true">
          {#if current.artwork && !artBroken}
            <img src={current.artwork} alt="" onerror={() => (artBroken = true)} />
          {:else}
            <span class="cover-fallback">
              <span class="cover-disc"></span>
              <Icon name="music" size={26} />
            </span>
          {/if}
          {#if isPlaying()}
            <div class="art-viz">
              <Visualizer variant={store.settings.visualizer_style} compact />
            </div>
          {/if}
        </div>

        <div class="info">
          <span class="title" title={current.title}>{current.title || t("player.untitled")}</span>
          <span class="artist" title={current.artist}>
            {current.artist || t("player.unknown_artist")}
            {#if current.album}<span class="album"> · {current.album}</span>{/if}
          </span>
        </div>

        <div class="controls">
          <button class="btn icon-only" aria-label={t("player.prev")} title={t("player.prev")} onclick={() => mediaControl("previous")}>
            <Icon name="skip-back" size={16} />
          </button>
          <button class="btn icon-only seek-jump" aria-label={t("player.rewind")} title={t("player.rewind")} onclick={() => seekBy(-15)}>
            <Icon name="rewind" size={15} />
          </button>
          <button
            class="btn icon-only play"
            aria-label={isPlaying() ? t("player.pause") : t("player.play")}
            title={isPlaying() ? t("player.pause") : t("player.play")}
            onclick={() => mediaControl("play_pause")}
          >
            <Icon name={isPlaying() ? "pause" : "play"} size={16} />
          </button>
          <button class="btn icon-only seek-jump" aria-label={t("player.forward")} title={t("player.forward")} onclick={() => seekBy(15)}>
            <Icon name="forward" size={15} />
          </button>
          <button class="btn icon-only" aria-label={t("player.next")} title={t("player.next")} onclick={() => mediaControl("next")}>
            <Icon name="skip-forward" size={16} />
          </button>
        </div>
      </div>
    {/key}

    <div class="timeline">
      <span class="time">{formatTrackTime(shownPosition)}</span>
      <SmoothSlider
        value={progress / 100}
        label={t("player.progress")}
        disabled={duration <= 0}
        onInput={seekInput}
        onCommit={seekCommit}
      />
      <span class="time">{formatTrackTime(duration)}</span>
    </div>

    <div class="mixer" class:muted={volume.muted || volume.level === 0}>
      <button
        class="btn icon-only volume-btn"
        aria-label={volume.muted ? t("player.unmute") : t("player.mute")}
        title={volume.muted ? t("player.unmute") : t("player.mute")}
        aria-pressed={volume.muted}
        onclick={() => void toggleMute()}
      >
        <Icon name={volume.muted || volume.level === 0 ? "volume-off" : "volume"} size={14} />
      </button>
      <SmoothSlider
        value={volume.level}
        label={t("player.volume")}
        onInput={volumeInput}
        onCommit={volumeCommit}
      />
      <span class="volume-value">{volumePercent}%</span>
    </div>

    {#if isPlaying()}
      <div class="viz-strip">
        <Visualizer variant={store.settings.visualizer_style} />
      </div>
    {/if}

    <div
      class="lyrics"
      class:style-pop={lyricsStyle === "pop"}
      class:style-blur={lyricsStyle === "blur"}
      class:style-karaoke={lyricsStyle === "karaoke"}
      aria-live="off"
    >
      {#if media.lyricsState === "loading"}
        <span class="lyrics-state">{t("player.lyrics_loading")}</span>
      {:else if media.instrumental}
        <span class="lyrics-state">{t("player.instrumental")}</span>
      {:else if synced.length}
        <div class="lyrics-track" style:transform={`translateY(${lyricsShift}px)`}>
          {#each synced as line, index (line.time + "-" + index)}
            <span
              class="line"
              class:active={index === activeIndex}
              class:past={index < activeIndex}
              style:--lyric-progress={index === activeIndex ? karaokeProgress : 0}
            >
              {line.text}
            </span>
          {/each}
        </div>
      {:else if media.lyricsPlain}
        <p class="plain">{media.lyricsPlain}</p>
      {:else if media.lyricsState === "error"}
        <span class="lyrics-state">{t("player.lyrics_error")}</span>
      {:else if media.lyricsState === "none"}
        <span class="lyrics-state">{t("player.lyrics_none")}</span>
      {:else}
        <span class="lyrics-state">{t("player.lyrics_hint")}</span>
      {/if}
    </div>
  {:else}
    <div class="art empty-art" aria-hidden="true">
      <div class="disc"></div>
      <Icon name="music" size={30} />
    </div>
    <div class="body">
      <span class="title">{t("player.no_music")}</span>
      <p class="hint">{t("player.no_music_hint")}</p>
    </div>
    <div class="empty-volume" class:muted={volume.muted || volume.level === 0}>
      <button
        class="btn icon-only volume-btn"
        aria-label={volume.muted ? t("player.unmute") : t("player.mute")}
        title={volume.muted ? t("player.unmute") : t("player.mute")}
        aria-pressed={volume.muted}
        onclick={() => void toggleMute()}
      >
        <Icon name={volume.muted || volume.level === 0 ? "volume-off" : "volume"} size={15} />
      </button>
      <SmoothSlider
        value={volume.level}
        label={t("player.volume")}
        onInput={volumeInput}
        onCommit={volumeCommit}
      />
      <span class="volume-value">{volumePercent}%</span>
    </div>
  {/if}
</div>

<style>
  .player {
    height: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px 14px 4px;
    position: relative;
    isolation: isolate;
    overflow: hidden;
  }

  .backdrop {
    position: absolute;
    inset: -24px;
    z-index: -1;
    background-size: cover;
    background-position: center;
    filter: blur(30px) saturate(1.35);
    opacity: 0.24;
    transform: scale(1.15);
    pointer-events: none;
    mask-image: linear-gradient(to bottom, #000, transparent 92%);
  }

  .player.empty {
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: 12px;
  }

  .top {
    display: flex;
    align-items: center;
    gap: 11px;
    flex: none;
    animation: track-in 0.4s cubic-bezier(0.22, 1, 0.36, 1);
  }

  @keyframes track-in {
    from {
      opacity: 0;
      transform: translateY(4px) scale(0.99);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .art {
    width: 72px;
    height: 72px;
    border-radius: 14px;
    background: linear-gradient(140deg, rgba(255, 255, 255, 0.14), rgba(255, 255, 255, 0.03));
    display: grid;
    place-items: center;
    color: var(--fg);
    overflow: hidden;
    flex: none;
    position: relative;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
    border: 1px solid color-mix(in srgb, var(--fg) 8%, transparent);
  }

  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .cover-fallback {
    position: relative;
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    background:
      radial-gradient(circle at 30% 25%, color-mix(in srgb, var(--live-accent) 55%, transparent), transparent 58%),
      linear-gradient(145deg, color-mix(in srgb, var(--live-accent) 26%, #111827), #0b1220);
    color: color-mix(in srgb, var(--fg) 82%, transparent);
    overflow: hidden;
  }

  .cover-disc {
    position: absolute;
    width: 82%;
    height: 82%;
    border-radius: 50%;
    border: 1px dashed color-mix(in srgb, var(--fg) 28%, transparent);
    box-shadow: inset 0 0 0 8px color-mix(in srgb, var(--bg) 55%, transparent);
  }

  :global(:root[data-reduced-motion="true"]) .cover-disc {
    animation: none;
  }

  .art-viz {
    position: absolute;
    left: 4px;
    right: 4px;
    bottom: 3px;
    height: 20px;
    padding: 2px 3px;
    border-radius: 5px;
    background: rgba(0, 0, 0, 0.38);
    backdrop-filter: blur(3px);
    pointer-events: none;
    opacity: 0.9;
  }

  .viz-strip {
    flex: none;
    height: 46px;
    padding: 0 2px;
    opacity: 0.95;
  }

  .empty-art {
    width: 84px;
    height: 84px;
    position: relative;
    border-radius: 18px;
  }

  .disc {
    position: absolute;
    inset: 11px;
    border-radius: 50%;
    border: 1px dashed rgba(255, 255, 255, 0.18);
  }

  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .title {
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.01em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .artist {
    font-size: 11.5px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .album {
    opacity: 0.8;
  }

  .controls {
    display: flex;
    gap: 4px;
    flex: none;
  }

  .play {
    background: var(--accent);
    color: #fff;
  }

  .seek-jump {
    color: var(--muted);
  }

  .seek-jump :global(svg) {
    opacity: 0.9;
  }

  .volume-btn {
    color: var(--muted);
    flex: none;
  }

  .mixer {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: none;
    padding: 2px 8px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: color-mix(in srgb, var(--surface) 70%, transparent);
    transition: border-color 0.2s ease, background 0.2s ease;
  }

  .mixer:hover,
  .mixer:focus-within {
    border-color: color-mix(in srgb, var(--live-accent) 30%, var(--border));
    background: var(--surface);
  }

  .mixer.muted {
    opacity: 0.78;
  }

  .mixer.muted .volume-btn {
    color: var(--danger);
  }

  .volume-value {
    min-width: 26px;
    font-size: 8.5px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    text-align: right;
    flex: none;
  }

  .timeline {
    display: flex;
    align-items: center;
    gap: 9px;
    flex: none;
    font-size: 10.5px;
    font-weight: 550;
    color: color-mix(in srgb, var(--fg) 74%, transparent);
    font-variant-numeric: tabular-nums;
    letter-spacing: 0;
  }

  .time {
    min-width: 34px;
    text-align: center;
    line-height: 1;
  }

  .lyrics {
    flex: 1;
    min-height: 0;
    position: relative;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
    mask-image: linear-gradient(to bottom, transparent, #000 12%, #000 88%, transparent);
  }

  .lyrics-track {
    position: absolute;
    left: 0;
    right: 0;
    top: 50%;
    display: flex;
    flex-direction: column;
    transition: transform 0.5s cubic-bezier(0.22, 1, 0.36, 1);
    will-change: transform;
  }

  .line {
    height: 26px;
    display: flex;
    align-items: center;
    font-size: 14px;
    line-height: 1.25;
    color: var(--muted);
    transition: color 0.3s ease, transform 0.3s ease, opacity 0.3s ease, filter 0.3s ease;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    opacity: 0.58;
    transform-origin: left center;
  }

  .line.active {
    color: var(--fg);
    font-size: 16px;
    font-weight: 650;
    transform: translateX(5px) scale(1.03);
    opacity: 1;
  }

  /* Pop: the active line springs forward, neighbours recede. */
  .style-pop .line {
    transform-origin: left center;
    transition: color 0.25s ease, transform 0.42s cubic-bezier(0.22, 1.5, 0.36, 1),
      opacity 0.3s ease, filter 0.3s ease;
  }

  .style-pop .line.active {
    transform: translateX(7px) scale(1.09);
    letter-spacing: 0.1px;
  }

  .style-pop .line:not(.active) {
    transform: scale(0.96);
  }

  /* Blur: unfocused lines lose definition, the active one sharpens. */
  .style-blur .line:not(.active) {
    filter: blur(1.4px);
    opacity: 0.42;
  }

  .style-blur .line.past {
    opacity: 0.22;
  }

  .style-blur .line.active {
    filter: blur(0);
  }

  /* Karaoke: the active line fills with colour as it is sung. */
  .style-karaoke .line.active {
    background-image: linear-gradient(
      90deg,
      color-mix(in srgb, var(--live-accent) 78%, #fff) calc(var(--lyric-progress, 0) * 100%),
      color-mix(in srgb, var(--fg) 34%, transparent) 0
    );
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
    text-shadow: none;
  }

  .line.past {
    opacity: 0.32;
  }

  .plain {
    margin: 0;
    font-size: 11px;
    line-height: 1.45;
    color: var(--muted);
    white-space: pre-wrap;
    overflow-y: auto;
    max-height: 100%;
    scrollbar-width: none;
  }

  .plain::-webkit-scrollbar {
    display: none;
  }

  .lyrics-state {
    font-size: 11px;
    color: var(--muted);
    text-align: center;
  }

  .empty-volume {
    display: flex;
    align-items: center;
    gap: 8px;
    width: min(300px, 90%);
    padding: 7px 10px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    animation: track-in 0.45s cubic-bezier(0.22, 1, 0.36, 1);
  }

  .empty-volume .volume-btn {
    color: var(--muted);
    flex: none;
  }

  .empty-volume:hover .volume-btn,
  .empty-volume:focus-within .volume-btn {
    color: var(--fg);
  }

  .empty-volume.muted .volume-btn {
    color: var(--danger);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-width: 380px;
  }

  .hint {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.45;
    color: var(--muted);
  }

  :global(:root[data-reduced-motion="true"]) .line {
    transition: none;
  }
</style>
