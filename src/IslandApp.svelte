<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Icon from "./lib/components/Icon.svelte";
  import Clock from "./lib/components/Clock.svelte";
  import Character from "./lib/components/Character.svelte";
  import WeatherBadge from "./lib/components/WeatherBadge.svelte";
  import BatteryBadge from "./lib/components/BatteryBadge.svelte";
  import MiniSpectrum from "./lib/components/MiniSpectrum.svelte";
  import SceneLayer from "./lib/components/SceneLayer.svelte";
  import Home from "./lib/modules/home/Home.svelte";
  import Player from "./lib/modules/player/Player.svelte";
  import Clipboard from "./lib/modules/clipboard/Clipboard.svelte";
  import TextTools from "./lib/modules/text/Text.svelte";
  import Notes from "./lib/modules/notes/Notes.svelte";
  import Timer from "./lib/modules/timer/Timer.svelte";
  import Weather from "./lib/modules/weather/Weather.svelte";
  import Capture from "./lib/modules/capture/Capture.svelte";
  import Calendar from "./lib/modules/calendar/Calendar.svelte";
  import Chat from "./lib/modules/chat/Chat.svelte";
  import Control from "./lib/modules/control/Control.svelte";
  import Stats from "./lib/modules/stats/Stats.svelte";
  import Agents from "./lib/modules/agents/Agents.svelte";
  import Phone from "./lib/modules/phone/Phone.svelte";
  import { MODULES } from "./lib/modules/registry";
  import { motionDuration, store, t, updateSettings } from "./lib/store.svelte";
  import { ensureWeather } from "./lib/weather.svelte";
  import { initMedia, audio, currentLyricText, media } from "./lib/media.svelte";
  import {
    againTimer,
    markTimerFinished,
    resetTimer,
    timerLabel,
    timerVisible,
  } from "./lib/timer.svelte";
  import { initStats, effectiveFps, system } from "./lib/stats.svelte";
  import { initBattery } from "./lib/battery.svelte";
  import { agentPhase, agents, initAgents } from "./lib/agents.svelte";

  const COLLAPSED_BASE = 142;
  const COLLAPSED_CHARACTER = 32;
  const COLLAPSED_BATTERY = 48;
  const EXPANDED_WIDTH = 560;
  const EXPANDED_HEIGHT = 300;
  const HOME_HEIGHT = 380;
  const PLAYER_HEIGHT = 404;

  let expanded = $state(false);
  let activeTab = $state("home");
  let islandEl: HTMLDivElement | undefined = $state();

  let hoverTimer: ReturnType<typeof setTimeout> | undefined;
  let closeTimer: ReturnType<typeof setTimeout> | undefined;
  let resizeObserver: ResizeObserver | undefined;
  let moveRegionTimer: ReturnType<typeof setTimeout> | undefined;
  let attentionTimer: ReturnType<typeof setTimeout> | undefined;
  let positionTimer: ReturnType<typeof setTimeout> | undefined;
  let attention = $state(false);
  const lyricLine = $derived(currentLyricText());
  let dragOrigin: { x: number; y: number } | null = null;
  let dragStartedAt = 0;
  let dragging = $state(false);
  let suppressClick = false;
  const interactionsReadyAt = performance.now() + 700;

  function isMacPlatform(): boolean {
    return /Macintosh|Mac OS X/.test(navigator.userAgent);
  }
  let positionTrackingReady = false;
  setTimeout(() => {
    positionTrackingReady = true;
  }, 1600);
  let dockOverride = $state<string | null>(null);
  let near = $state(0);
  let ripple = $state<{ x: number; y: number; id: number } | null>(null);
  let musicAutoKey = "";
  let musicSuppressedKey = "";
  let lastTrackKey = "";
  let collapsedArtBroken = $state(false);
  let contentDir = $state(1);
  $effect(() => {
    void media.current.artwork;
    collapsedArtBroken = false;
  });

  const expandedHeight = $derived(
    activeTab === "home"
      ? HOME_HEIGHT
      : activeTab === "player"
        ? PLAYER_HEIGHT
        : activeTab === "timer"
          ? 350
          : activeTab === "capture"
            ? 380
            : activeTab === "text"
              ? 340
              : activeTab === "stats"
                ? 392
                : EXPANDED_HEIGHT,
  );
  const dock = $derived(dockOverride ?? store.settings.island_dock ?? "top");
  const pinned = $derived(store.settings.island_pinned);
  const liveAccent = $derived(media.accent ?? store.settings.accent_color);
  const ambient = $derived(
    ["aurora", "forest", "sunset", "rose"].includes(store.settings.theme),
  );
  const karaoke = $derived(
    store.settings.karaoke_line &&
      media.current.active &&
      media.current.status === "playing" &&
      lyricLine,
  );
  const agentState = $derived(agentPhase(agents.events[0]));
  const fpsNow = $derived(effectiveFps());

  const collapsedWidth = $derived(
    COLLAPSED_BASE +
      (store.settings.character === "hud"
        ? 104
        : store.settings.character && store.settings.character !== "none"
          ? COLLAPSED_CHARACTER
          : 0) +
      (store.settings.show_battery ? COLLAPSED_BATTERY : 0),
  );

  const enabledModules = $derived(
    MODULES.filter(
      (module) => module.id !== "home" && store.settings.modules[module.id] !== false,
    ),
  );

  const tabs = $derived.by(() => {
    const order = store.settings.module_order;
    const byId = new Map(enabledModules.map((module) => [module.id, module]));
    const sorted: Array<(typeof MODULES)[number]> = [];
    for (const id of order) {
      const module = byId.get(id as (typeof MODULES)[number]["id"]);
      if (module) {
        sorted.push(module);
        byId.delete(module.id);
      }
    }
    for (const module of byId.values()) sorted.push(module);
    return sorted;
  });

  let regionTimer: ReturnType<typeof setTimeout> | undefined;
  let resizeTimer: ReturnType<typeof setTimeout> | undefined;
  let lastRegion = "";
  type RegionTarget = "expanded" | "collapsed" | "banner";
  type BannerKind = "reminder" | "download" | "phone" | "agent" | "screenshot" | "timer";
  interface Banner {
    kind: BannerKind;
    title: string;
    detail: string;
  }
  let banner = $state<Banner | null>(null);
  let bannerTimer: ReturnType<typeof setTimeout> | undefined;

  function targetSize(target: RegionTarget): { width: number; height: number } {
    if (target === "expanded") return { width: EXPANDED_WIDTH, height: expandedHeight };
    if (target === "banner") return { width: 380, height: 64 };
    return { width: collapsedWidth, height: 34 };
  }

  function sendRegion(
    x: number,
    y: number,
    w: number,
    h: number,
    r: number,
    layout: { left: number; top: number; width: number; height: number },
  ): void {
    const key = `${x}:${y}:${w}:${h}:${r}:${dock}`;
    if (key === lastRegion) return;
    lastRegion = key;
    void invoke("set_island_rect", { rect: { x, y, w, h, r }, layout }).catch(() => {
      lastRegion = "";
    });
  }

  function applyTargetRegion(target: RegionTarget): void {
    const dpr = window.devicePixelRatio || 1;
    const size = targetSize(target);
    const left =
      dock === "left"
        ? 0
        : dock === "right"
          ? Math.max(0, window.innerWidth - size.width)
          : Math.max(0, (window.innerWidth - size.width) / 2);
    const top =
      dock === "bottom" ? Math.max(0, window.innerHeight - size.height) : 0;
    const radius =
      target === "collapsed"
        ? size.height / 2
        : target === "banner"
          ? 24
          : store.settings.radius;
    const x = Math.round(left * dpr);
    const y = Math.round(top * dpr);
    const w = Math.round(size.width * dpr);
    const h = Math.round(size.height * dpr);
    sendRegion(x, y, w, h, radius * dpr, { left: x, top: y, width: w, height: h });
  }

  function currentTarget(): RegionTarget {
    if (expanded) return "expanded";
    return banner ? "banner" : "collapsed";
  }

  function queueRegion(target: RegionTarget, delay: number): void {
    if (regionTimer) clearTimeout(regionTimer);
    regionTimer = setTimeout(() => applyTargetRegion(target), delay);
  }

  function dismissBanner(): void {
    if (!banner) return;
    if (bannerTimer) clearTimeout(bannerTimer);
    banner = null;
    if (!expanded) queueRegion("collapsed", motionDuration() + 20);
  }

  function showBanner(next: Banner): void {
    if (dragging) return;
    if (expanded) {
      flashAttention();
      return;
    }
    banner = next;
    if (bannerTimer) clearTimeout(bannerTimer);
    applyTargetRegion("banner");
    bannerTimer = setTimeout(() => {
      banner = null;
      if (!expanded) queueRegion("collapsed", motionDuration() + 20);
    }, 4800);
  }

  function clearTimers(): void {
    if (hoverTimer) {
      clearTimeout(hoverTimer);
      hoverTimer = undefined;
    }
    if (closeTimer) {
      clearTimeout(closeTimer);
      closeTimer = undefined;
    }
  }

  function expand(): void {
    if (expanded) return;
    clearTimers();
    expanded = true;
    if (bannerTimer) clearTimeout(bannerTimer);
    banner = null;
    // Grow the hit region immediately so the animation is never clipped.
    if (regionTimer) clearTimeout(regionTimer);
    applyTargetRegion("expanded");
  }

  function collapse(): void {
    if (!expanded) return;
    clearTimers();
    if (pinned) updateSettings({ island_pinned: false });
    if (activeTab === "player" && media.current.active) {
      musicSuppressedKey = `${media.current.title}|${media.current.artist}`;
    }
    expanded = false;
    // Keep the larger region until the spring (per motion profile) finishes,
    // then shrink it.
    queueRegion("collapsed", motionDuration() + 20);
  }

  function toggle(): void {
    clearTimers();
    const target = !expanded;
    expanded = target;
    if (target) {
      if (bannerTimer) clearTimeout(bannerTimer);
      banner = null;
      if (regionTimer) clearTimeout(regionTimer);
      applyTargetRegion("expanded");
    } else {
      if (pinned) updateSettings({ island_pinned: false });
      queueRegion("collapsed", motionDuration() + 20);
    }
  }

  function togglePin(): void {
    const next = !pinned;
    updateSettings({ island_pinned: next });
    if (next) {
      clearTimers();
      expand();
    }
  }

  function onEnter(): void {
    if (dragOrigin || dragging) return;
    if (expanded) {
      if (closeTimer) {
        clearTimeout(closeTimer);
        closeTimer = undefined;
      }
      return;
    }
    if (store.settings.behavior === "click" || store.settings.behavior === "double") return;
    clearTimers();
    if (store.settings.behavior === "always") {
      expand();
      return;
    }
    hoverTimer = setTimeout(expand, Math.max(0, store.settings.hover_delay_ms));
  }

  function onLeave(): void {
    if (dragOrigin || dragging) return;
    if (!expanded) {
      clearTimers();
      return;
    }
    if (store.settings.behavior === "always" || pinned) return;
    if (closeTimer) clearTimeout(closeTimer);
    closeTimer = setTimeout(collapse, Math.max(0, store.settings.close_delay_ms));
  }

  function onIslandClick(): void {
    if (suppressClick) {
      suppressClick = false;
      return;
    }
    if (!expanded) {
      if (store.settings.behavior === "double") return;
      expand();
    }
  }

  function onIslandDoubleClick(event: MouseEvent): void {
    if (suppressClick) {
      suppressClick = false;
      return;
    }
    const target = event.target as HTMLElement | null;
    if (target?.closest("button, input, textarea, select, a, [data-no-drag]")) return;
    // Two taps keep the island open: pin it and expand if it was collapsed.
    clearTimers();
    if (!store.settings.island_pinned) updateSettings({ island_pinned: true });
    expand();
    flashAttention();
  }

  function onPointerDown(event: PointerEvent): void {
    if (event.button !== 0) return;
    if (!event.isTrusted || performance.now() < interactionsReadyAt) return;
    const target = event.target as HTMLElement | null;
    const interactive = target?.closest("button, input, textarea, select, a, [data-no-drag]");
    if (interactive && !event.ctrlKey) return;
    // A pinned island stays exactly where the user put it.
    if (pinned) return;
    if (event.ctrlKey) event.preventDefault();
    clearTimers();
    suppressClick = false;
    dragOrigin = { x: event.screenX, y: event.screenY };
    dragStartedAt = performance.now();
    dragging = false;
    if (islandEl) {
      const rect = islandEl.getBoundingClientRect();
      const id = Date.now();
      ripple = { x: event.clientX - rect.left, y: event.clientY - rect.top, id };
      setTimeout(() => {
        if (ripple?.id === id) ripple = null;
      }, 520);
    }
    // macOS does not expose a global cursor feed to the custom drag loop, so
    // the drag is handed to the native window manager (see startCustomDrag).
    try {
      (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    } catch {
      /* pointer capture is optional */
    }
  }

  function startCustomDrag(): void {
    if (!islandEl) return;
    if (isMacPlatform()) {
      // The native window drag keeps the transparent island attached to the
      // pointer without a global cursor loop.
      dragging = true;
      suppressClick = true;
      dragOrigin = null;
      clearTimers();
      void getCurrentWindow()
        .startDragging()
        .finally(() => {
          dragging = false;
          schedulePositionSave();
        })
        .catch(() => {
          dragging = false;
          suppressClick = false;
        });
      return;
    }
    const rect = islandEl.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    const layout = {
      left: Math.round(rect.left * dpr),
      top: Math.round(rect.top * dpr),
      width: Math.round(rect.width * dpr),
      height: Math.round(rect.height * dpr),
    };
    dragging = true;
    suppressClick = true;
    dragOrigin = null;
    clearTimers();
    lastRegion = "";
    void invoke("begin_island_drag", { layout }).catch(() => {
      dragging = false;
      suppressClick = false;
    });
  }

  function onPointerMove(event: PointerEvent): void {
    if (!dragOrigin || dragging) return;
    if ((event.buttons & 1) === 0 && performance.now() - dragStartedAt > 600) {
      dragOrigin = null;
      return;
    }
    if (Math.hypot(event.screenX - dragOrigin.x, event.screenY - dragOrigin.y) < 4) return;
    startCustomDrag();
  }

  function onPointerUp(): void {
    dragOrigin = null;
  }

  function flashAttention(): void {
    attention = true;
    if (attentionTimer) clearTimeout(attentionTimer);
    attentionTimer = setTimeout(() => (attention = false), 1600);
  }

  function schedulePositionSave(): void {
    if (positionTimer) clearTimeout(positionTimer);
    positionTimer = setTimeout(() => {
      void invoke("save_island_position");
    }, 350);
  }

  function onIslandKeydown(event: KeyboardEvent): void {
    if (pinned) return;
    if (!event.altKey || !["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(event.key)) {
      return;
    }
    event.preventDefault();
    const step = event.shiftKey ? 40 : 10;
    const dx = event.key === "ArrowLeft" ? -step : event.key === "ArrowRight" ? step : 0;
    const dy = event.key === "ArrowUp" ? -step : event.key === "ArrowDown" ? step : 0;
    void invoke("move_island", { dx, dy });
  }

  function go(id: string): void {
    const order = ["home", ...tabs.map((module) => module.id)];
    contentDir = order.indexOf(id) >= order.indexOf(activeTab) ? 1 : -1;
    activeTab = id;
  }

  function openSettings(): void {
    void invoke("open_settings");
  }

  async function setup(): Promise<void> {
    await Promise.resolve();
    if (islandEl) {
      islandEl.addEventListener("mouseenter", onEnter);
      islandEl.addEventListener("mouseleave", onLeave);
      islandEl.addEventListener("click", onIslandClick);
      islandEl.addEventListener("dblclick", onIslandDoubleClick);
      islandEl.addEventListener("pointerdown", onPointerDown);
      islandEl.addEventListener("pointermove", onPointerMove);
      islandEl.addEventListener("pointerup", onPointerUp);
      islandEl.addEventListener("pointercancel", onPointerUp);
    }
    applyTargetRegion("collapsed");
    resizeObserver = new ResizeObserver(() => {
      if (resizeTimer) clearTimeout(resizeTimer);
      resizeTimer = setTimeout(() => applyTargetRegion(currentTarget()), 160);
    });
    if (islandEl) resizeObserver.observe(islandEl);
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape" && expanded) {
      event.preventDefault();
      collapse();
    }
  }

  onMount(() => {
    void ensureWeather();
    void initMedia();
    void initStats();
    void initBattery();
    void initAgents();
    void setup();
    applyTargetRegion("collapsed");
    if (store.settings.behavior === "always" || store.settings.island_pinned) {
      setTimeout(() => expand(), 260);
    }
    window.addEventListener("keydown", onKeydown);
    window.addEventListener("keydown", onIslandKeydown);
    window.addEventListener("blur", () => {
      if (expanded) applyTargetRegion("expanded");
    });

    const unlisten: Array<() => void> = [];
    void listen("island-toggle", () => toggle()).then((off) => unlisten.push(off));
    void listen("island-collapse", () => collapse()).then((off) => unlisten.push(off));
    void listen("island-expand", () => expand()).then((off) => unlisten.push(off));
    void listen("timer-finished", () => {
      markTimerFinished();
      if (expanded) {
        activeTab = "timer";
        flashAttention();
      } else {
        showBanner({
          kind: "timer",
          title: t("banner.timer"),
          detail: t("banner.timer_detail"),
        });
      }
    }).then((off) => unlisten.push(off));
    void listen<{ dock?: string }>("island-dock", (event) => {
      dockOverride = event.payload.dock ?? null;
      requestAnimationFrame(() => applyTargetRegion(currentTarget()));
    }).then((off) => unlisten.push(off));
    void listen<{ dock?: string }>("island-drag-end", (event) => {
      dragging = false;
      dockOverride = event.payload.dock ?? null;
      lastRegion = "";
      setTimeout(() => applyTargetRegion(currentTarget()), 80);
    }).then((off) => unlisten.push(off));
    void listen("reminder-due", (event) => {
      const payload = event.payload as { title?: string };
      if (expanded) {
        activeTab = "calendar";
        flashAttention();
      } else {
        showBanner({
          kind: "reminder",
          title: t("banner.reminder"),
          detail: payload.title ?? "",
        });
      }
    }).then((off) => unlisten.push(off));
    void listen<{ source?: string; event?: string; title?: string; detail?: string }>(
      "agent-event",
      (event) => {
        const phase = agentPhase({ event: event.payload.event ?? "" });
        // "working" events arrive every few seconds while an agent runs. Keep
        // them out of the banner so the island does not stay in banner mode;
        // the collapsed pill shows a small coloured dot instead.
        if (phase === "working" || phase === "idle") return;
        showBanner({
          kind: "agent",
          title: event.payload.source || t("banner.agent"),
          detail: event.payload.title || event.payload.detail || "",
        });
      },
    ).then((off) => unlisten.push(off));
    void listen<{ app?: string; title?: string; text?: string }>(
      "phone-notification",
      (event) => {
        showBanner({
          kind: "phone",
          title: event.payload.app || t("banner.phone"),
          detail: event.payload.title || event.payload.text || "",
        });
      },
    ).then((off) => unlisten.push(off));
    void listen<{ name?: string }>("phone-file", (event) => {
      showBanner({
        kind: "phone",
        title: t("banner.file"),
        detail: event.payload.name ?? "",
      });
    }).then((off) => unlisten.push(off));
    void listen<{ name?: string; done?: boolean; error?: string | null }>(
      "download-progress",
      (event) => {
        if (event.payload.done && !event.payload.error) {
          showBanner({
            kind: "download",
            title: t("banner.download"),
            detail: event.payload.name ?? "",
          });
        }
      },
    ).then((off) => unlisten.push(off));
    void listen<{ name?: string }>("screenshot-taken", (event) => {
      showBanner({
        kind: "screenshot",
        title: t("banner.screenshot"),
        detail: event.payload.name ?? "",
      });
    }).then((off) => unlisten.push(off));
    void getCurrentWindow()
      .onMoved(() => {
        if (dragging) return;
        if (moveRegionTimer) clearTimeout(moveRegionTimer);
        moveRegionTimer = setTimeout(() => applyTargetRegion(currentTarget()), 220);
        if (positionTrackingReady) schedulePositionSave();
      })
      .then((off) => unlisten.push(off));
    void getCurrentWindow()
      .onScaleChanged(() => queueRegion(currentTarget(), 150))
      .then((off) => unlisten.push(off));

    // The island leans a few pixels towards the cursor and glows when it is
    // nearby. The character's eyes already track the same global position.
    let windowPosition = { x: 0, y: 0 };
    void getCurrentWindow()
      .outerPosition()
      .then((position) => (windowPosition = { x: position.x, y: position.y }))
      .catch(() => {});
    void getCurrentWindow()
      .onMoved((event) => {
        windowPosition = { x: event.payload.x, y: event.payload.y };
      })
      .then((off) => unlisten.push(off));
    void listen<{ x: number; y: number }>("cursor-move", (event) => {
      if (!islandEl) return;
      const rect = islandEl.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;
      const centerX = windowPosition.x + (rect.left + rect.width / 2) * dpr;
      const centerY = windowPosition.y + (rect.top + rect.height / 2) * dpr;
      const dx = event.payload.x - centerX;
      const dy = event.payload.y - centerY;
      const distance = Math.hypot(dx, dy);
      const reach = Math.max(0, Math.min(1, 1 - distance / 230));
      near = reach * reach;
      void dy;
    }).then((off) => unlisten.push(off));

    return () => {
      window.removeEventListener("keydown", onKeydown);
      window.removeEventListener("keydown", onIslandKeydown);
      islandEl?.removeEventListener("mouseenter", onEnter);
      islandEl?.removeEventListener("mouseleave", onLeave);
      islandEl?.removeEventListener("click", onIslandClick);
      islandEl?.removeEventListener("dblclick", onIslandDoubleClick);
      islandEl?.removeEventListener("pointerdown", onPointerDown);
      islandEl?.removeEventListener("pointermove", onPointerMove);
      islandEl?.removeEventListener("pointerup", onPointerUp);
      islandEl?.removeEventListener("pointercancel", onPointerUp);
      if (attentionTimer) clearTimeout(attentionTimer);
      if (positionTimer) clearTimeout(positionTimer);
      if (regionTimer) clearTimeout(regionTimer);
      if (resizeTimer) clearTimeout(resizeTimer);
      if (moveRegionTimer) clearTimeout(moveRegionTimer);
      if (bannerTimer) clearTimeout(bannerTimer);
      clearTimers();
      resizeObserver?.disconnect();
      for (const off of unlisten) off();
    };
  });

  $effect(() => {
    // Follow module list changes so the active tab always exists.
    if (expanded && activeTab !== "home" && !tabs.some((module) => module.id === activeTab)) {
      activeTab = "home";
    }
  });

  let wasPinned = false;
  $effect(() => {
    const isPinned = store.settings.island_pinned;
    const previous = wasPinned;
    wasPinned = isPinned;
    if (!store.loaded) return;
    if (isPinned) {
      // Toggling the pin in Settings should open the island right away.
      if (!expanded && !dragging) {
        clearTimers();
        expand();
      }
      return;
    }
    // Just unpinned: collapse unless the pointer is still over the island
    // (for example when the pin button itself was clicked).
    if (!previous || !expanded || store.settings.behavior === "always") return;
    const timer = setTimeout(() => {
      if (!islandEl?.matches(":hover")) collapse();
    }, Math.max(250, store.settings.close_delay_ms));
    return () => clearTimeout(timer);
  });

  $effect(() => {
    const nextDock = store.settings.island_dock;
    if (dockOverride && nextDock === dockOverride) {
      dockOverride = null;
    }
    lastRegion = "";
    queueRegion(currentTarget(), 60);
  });

  $effect(() => {
    // A different tab can change the expanded height (home widgets and the
    // lyrics panel get more room). Re-apply the hit region after the CSS
    // transition so the window never clips the content.
    void expandedHeight;
    if (expanded) {
      lastRegion = "";
      queueRegion("expanded", 40);
    }
  });

  $effect(() => {
    const active = media.current.active;
    const status = media.current.status;
    const title = media.current.title;
    const artist = media.current.artist;
    const lyricsReady =
      media.lyricsState === "ready" && (media.lyrics.length > 0 || media.lyricsPlain.length > 0);
    if (!active || status !== "playing" || !title) {
      lastTrackKey = "";
      musicAutoKey = "";
      musicSuppressedKey = "";
      return;
    }
    const key = `${title}|${artist}`;
    if (key !== lastTrackKey) {
      lastTrackKey = key;
      musicAutoKey = "";
      musicSuppressedKey = "";
    }
    if (!store.settings.auto_music_panel || !lyricsReady) return;
    if (expanded || dragging || key === musicSuppressedKey || key === musicAutoKey) return;
    musicAutoKey = key;
    activeTab = "player";
    expand();
  });

  $effect(() => {
    const lat = store.settings.weather_lat;
    const lon = store.settings.weather_lon;
    const units = store.settings.weather_units;
    if (!store.loaded) return;
    void lat;
    void lon;
    void units;
    void ensureWeather(true);
  });
</script>

<div class="stage" class:dock-left={dock === "left"} class:dock-right={dock === "right"} class:dock-bottom={dock === "bottom"} class:dock-top={dock === "top"} class:dock-free={dock === "free"}>
  <div
    class="island"
    class:expanded
    class:banner={banner !== null && !expanded}
    class:attention
    class:dragging
    class:ambient
    class:pinned
    bind:this={islandEl}
    style:--island-width={`${expanded ? EXPANDED_WIDTH : banner ? 380 : collapsedWidth}px`}
    style:--island-height={`${expanded ? expandedHeight : banner ? 64 : 34}px`}
    style:--island-radius={`${expanded ? `${store.settings.radius}px` : banner ? "24px" : "999px"}`}
    style:--audio-level={audio.level}
    style:--live-accent={liveAccent}
    style:--near={near}
    role="region"
    aria-label={t("app.name")}
  >
    <SceneLayer
      scene={store.settings.island_scene}
      gifUrl={store.settings.island_gif_url}
      expanded={expanded}
    />
    {#if ambient}
      <div class="ambient-layer" aria-hidden="true">
        {#each Array.from({ length: 10 }, (_, index) => index) as index (index)}
          <span
            style:--i={index}
            style:left={`${(index * 37 + 11) % 96}%`}
            style:animation-delay={`${(index * 1.7) % 7}s`}
            style:animation-duration={`${6 + (index % 4) * 1.8}s`}
          ></span>
        {/each}
      </div>
    {/if}
    {#if ripple}
      {#key ripple.id}
        <span
          class="ripple"
          style:left={`${ripple.x}px`}
          style:top={`${ripple.y}px`}
          aria-hidden="true"
        ></span>
      {/key}
    {/if}
    {#if expanded}
      <div class="chrome">
        <nav class="tabs" aria-label={t("home.quick_actions")}>
          <button
            class="tab"
            class:active={activeTab === "home"}
            onclick={(event) => {
              event.stopPropagation();
              go("home");
            }}
            title={t("nav.home")}
            aria-label={t("nav.home")}
          >
            <Icon name="home" size={15} />
          </button>
          {#each tabs as module (module.id)}
            <button
              class="tab"
              class:active={activeTab === module.id}
              onclick={(event) => {
                event.stopPropagation();
                go(module.id);
              }}
              title={t(`nav.${module.id}`)}
              aria-label={t(`nav.${module.id}`)}
            >
              <Icon name={module.icon} size={15} />
            </button>
          {/each}
        </nav>

        <span class="chrome-perf" aria-hidden="true">
          <span class="chrome-fps">{fpsNow === null ? "—" : Math.round(fpsNow)} FPS</span>
          {#if system.data?.cpuTemp != null}
            <span class="chrome-temp">{Math.round(system.data.cpuTemp)}°</span>
          {/if}
        </span>

        <div class="chrome-actions">
          <button
            class="tab action pin"
            class:active={pinned}
            onclick={(event) => {
              event.stopPropagation();
              togglePin();
            }}
            title={pinned ? t("island.unpin") : t("island.pin")}
            aria-label={pinned ? t("island.unpin") : t("island.pin")}
            aria-pressed={pinned}
          >
            <Icon name="pin" size={15} />
          </button>
          <button
            class="tab action"
            onclick={(event) => {
              event.stopPropagation();
              openSettings();
            }}
            title={t("settings.title")}
            aria-label={t("settings.title")}
          >
            <Icon name="settings" size={15} />
          </button>
          <button
            class="tab action"
            onclick={(event) => {
              event.stopPropagation();
              collapse();
            }}
            title={t("common.close")}
            aria-label={t("common.close")}
          >
            <Icon name="chevron-down" size={15} />
          </button>
        </div>
      </div>

      {#key activeTab}
        <div class="body" data-no-drag style:--dir={contentDir}>
          {#if activeTab === "home"}
          <Home go={go} />
        {:else if activeTab === "player"}
          <Player />
        {:else if activeTab === "clipboard"}
          <Clipboard />
        {:else if activeTab === "text"}
          <TextTools />
        {:else if activeTab === "notes"}
          <Notes />
        {:else if activeTab === "timer"}
          <Timer />
        {:else if activeTab === "weather"}
          <Weather />
        {:else if activeTab === "capture"}
          <Capture />
        {:else if activeTab === "calendar"}
          <Calendar />
        {:else if activeTab === "chat"}
          <Chat />
        {:else if activeTab === "control"}
          <Control />
        {:else if activeTab === "stats"}
          <Stats />
        {:else if activeTab === "agents"}
          <Agents />
        {:else if activeTab === "phone"}
          <Phone />
        {:else if activeTab === "terminal"}
          {#await import("./lib/modules/terminal/Terminal.svelte")}
            <span class="body-loading">{t("common.loading")}</span>
          {:then module}
            {@const TerminalView = module.default}
            <TerminalView />
          {/await}
        {:else}
          <Home go={go} />
        {/if}
        </div>
      {/key}
    {:else if banner}
      <div class="banner-card" role="group" aria-label={`${banner.title}: ${banner.detail}`}>
        <button
          class="banner-open"
          onclick={(event) => {
            event.stopPropagation();
            const kind = banner?.kind;
            dismissBanner();
            if (kind === "reminder") {
              activeTab = "calendar";
              expand();
            } else if (kind === "download") {
              activeTab = "control";
              expand();
            } else if (kind === "screenshot") {
              activeTab = "capture";
              expand();
            } else if (kind === "timer") {
              activeTab = "timer";
              expand();
            }
          }}
        >
          <span
            class="banner-icon"
            class:reminder={banner.kind === "reminder"}
            class:download={banner.kind === "download"}
            class:phone={banner.kind === "phone"}
            class:screenshot={banner.kind === "screenshot"}
            class:timer={banner.kind === "timer"}
          >
            <Icon
              name={banner.kind === "reminder"
                ? "bell"
                : banner.kind === "download"
                  ? "download"
                  : banner.kind === "phone"
                    ? "phone"
                    : banner.kind === "screenshot"
                      ? "camera"
                      : banner.kind === "timer"
                        ? "timer"
                        : "terminal"}
              size={16}
            />
          </span>
          <span class="banner-text">
            <strong>{banner.title}</strong>
            <span>{banner.detail || t("app.name")}</span>
          </span>
        </button>
        {#if banner.kind === "timer"}
          <span class="banner-actions">
            <button
              class="banner-action"
              onclick={(event) => {
                event.stopPropagation();
                resetTimer();
                dismissBanner();
              }}
            >
              {t("timer.stop")}
            </button>
            <button
              class="banner-action primary"
              onclick={(event) => {
                event.stopPropagation();
                againTimer();
                dismissBanner();
              }}
            >
              {t("timer.again")}
            </button>
          </span>
        {/if}
      </div>
    {:else}
      <div class="collapsed">
        {#if timerVisible()}
          <span class="collapsed-timer">
            <Icon name="timer" size={13} />
            <span>{timerLabel()}</span>
          </span>
          {#if media.current.active && media.current.status === "playing"}
            <MiniSpectrum variant={store.settings.visualizer_style} />
          {:else}
            <WeatherBadge />
          {/if}
        {:else if store.settings.show_stats_collapsed && system.data}
          <span class="compact-stats">
            <Icon name="gauge" size={11} />
            <span>
              CPU {Math.round(system.data.cpu)} · GPU {system.data.gpu === null ? "—" : Math.round(system.data.gpu)} · RAM {Math.round(system.data.ram)}
              {#if system.data.cpuTemp != null}· {Math.round(system.data.cpuTemp)}°{/if}
              {#if fpsNow != null}· {Math.round(fpsNow)} FPS{/if}
            </span>
          </span>
        {:else if media.current.active && media.current.status === "playing"}
          <Character />
          {#if media.current.artwork && !collapsedArtBroken}
            <img class="collapsed-art" src={media.current.artwork} alt="" onerror={() => (collapsedArtBroken = true)} />
          {/if}
          {#if karaoke}
            {#key lyricLine}
              <span class="collapsed-lyric" title={`${media.current.title} — ${lyricLine}`}>{lyricLine}</span>
            {/key}
          {:else}
            <div class="collapsed-clock">
              <Clock />
              {#if store.settings.character !== "hud"}
                <span class="collapsed-fps">{fpsNow === null ? "—" : `${Math.round(fpsNow)} fps`}</span>
              {/if}
            </div>
          {/if}
          <MiniSpectrum />
        {:else}
          <Character />
          <div class="collapsed-clock">
            <Clock />
            {#if store.settings.character !== "hud"}
              <span class="collapsed-fps">{fpsNow === null ? "—" : `${Math.round(fpsNow)} fps`}</span>
            {/if}
          </div>
          <WeatherBadge />
        {/if}
        {#if agentState === "working" || agentState === "attention" || agentState === "error"}
          <span
            class="agent-indicator"
            class:working={agentState === "working"}
            class:attention={agentState === "attention"}
            class:error={agentState === "error"}
            title={t(`agents.phase_${agentState}`)}
            aria-label={t(`agents.phase_${agentState}`)}
          ></span>
        {/if}
        {#if store.settings.show_battery}
          <BatteryBadge />
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .stage {
    width: 100vw;
    height: 100vh;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    pointer-events: none;
    overflow: hidden;
  }

  .stage.dock-left {
    justify-content: flex-start;
    align-items: flex-start;
  }

  .stage.dock-right {
    justify-content: flex-end;
    align-items: flex-start;
  }

  .stage.dock-bottom {
    justify-content: center;
    align-items: flex-end;
  }

  .island {
    pointer-events: auto;
    width: var(--island-width);
    height: var(--island-height);
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--fg) 4%, var(--bg)), var(--bg) 42%),
      var(--bg);
    color: var(--fg);
    border: 1px solid var(--border);
    border-radius: var(--island-radius);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    position: relative;
    transition: width var(--motion-duration, 320ms) var(--motion-ease, cubic-bezier(0.34, 1.2, 0.64, 1)),
      height var(--motion-duration, 320ms) var(--motion-ease, cubic-bezier(0.34, 1.2, 0.64, 1)),
      border-radius var(--motion-fast, 180ms) ease,
      box-shadow var(--motion-fast, 180ms) ease;
    user-select: none;
    isolation: isolate;
  }

  /* The signature: one hairline of light along the top edge, like a matte bezel. */
  .island::before {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: inherit;
    pointer-events: none;
    z-index: 2;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.07);
    opacity: calc(0.55 + var(--audio-level, 0) * 0.45 + var(--near, 0) * 0.2);
    transition: opacity 0.2s linear;
  }

  .island.dragging {
    cursor: grabbing;
  }

  .island.dragging::after {
    box-shadow: inset 0 0 0 1.5px color-mix(in srgb, var(--accent) 70%, transparent),
      inset 0 0 22px color-mix(in srgb, var(--accent) 28%, transparent);
  }

  .island.pinned {
    box-shadow:
      0 12px 38px rgba(0, 0, 0, 0.38),
      0 0 0 1px color-mix(in srgb, var(--live-accent) 34%, transparent);
  }

  .island.pinned::after {
    box-shadow:
      inset 0 0 0 1px color-mix(in srgb, var(--live-accent) 38%, transparent),
      inset 0 0 18px color-mix(in srgb, var(--live-accent) 12%, transparent);
  }

  .tab.pin.active :global(svg) {
    transform: rotate(-18deg);
    color: var(--live-accent);
  }

  .island.attention {
    animation: attention 1.5s ease;
  }

  .island.banner {
    animation: banner-in var(--motion-duration, 300ms) var(--motion-ease, cubic-bezier(0.34, 1.3, 0.64, 1));
  }

  @keyframes banner-in {
    from {
      opacity: 0.4;
      transform: translateY(-4px) scale(0.96);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  @keyframes attention {
    0%, 100% {
      box-shadow: none;
    }
    25%, 60% {
      box-shadow: inset 0 0 0 1.5px color-mix(in srgb, var(--accent) 55%, transparent);
    }
  }

  .island::after {
    content: "";
    position: absolute;
    inset: 0;
    pointer-events: none;
    border-radius: inherit;
    box-shadow: inset 0 0 0 1px var(--border);
    z-index: 3;
  }

  .ambient-layer {
    position: absolute;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
    z-index: 0;
    opacity: 0.22;
  }

  .ambient-layer span {
    position: absolute;
    top: -8px;
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: var(--accent);
    opacity: 0;
    animation-name: ambient-fall;
    animation-timing-function: linear;
    animation-iteration-count: infinite;
  }

  :global(:root[data-theme="forest"]) .ambient-layer span {
    width: 5px;
    height: 3px;
    border-radius: 2px 0 2px 0;
    background: #86efac;
  }

  :global(:root[data-theme="sunset"]) .ambient-layer span {
    background: #fdba74;
    box-shadow: 0 0 5px #fb923c;
  }

  :global(:root[data-theme="rose"]) .ambient-layer span {
    background: #f9a8d4;
    width: 4px;
    height: 4px;
  }

  @keyframes ambient-fall {
    0% {
      transform: translate3d(0, 0, 0) rotate(0deg);
      opacity: 0;
    }
    12% {
      opacity: 0.75;
    }
    100% {
      transform: translate3d(calc((var(--i) - 5) * 4px), 110vh, 0) rotate(240deg);
      opacity: 0;
    }
  }

  .ripple {
    position: absolute;
    width: 6px;
    height: 6px;
    margin: -3px 0 0 -3px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--live-accent) 55%, transparent);
    pointer-events: none;
    z-index: 4;
    animation: ripple-spread 0.5s ease-out forwards;
  }

  @keyframes ripple-spread {
    from {
      transform: scale(1);
      opacity: 0.55;
    }
    to {
      transform: scale(34);
      opacity: 0;
    }
  }

  .collapsed {
    position: relative;
    z-index: 1;
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 9px;
    padding: 0 14px;
    font-size: 13px;
    font-weight: 550;
    cursor: grab;
  }

  .collapsed:active {
    cursor: grabbing;
  }

  .collapsed-clock {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
    line-height: 1;
    letter-spacing: -0.2px;
  }

  .collapsed-fps {
    font-size: 7px;
    font-weight: 600;
    letter-spacing: 0.04em;
    line-height: 1;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    opacity: 0.85;
  }

  .collapsed-art {
    width: 26px;
    height: 26px;
    border-radius: 6px;
    object-fit: cover;
    flex: none;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.35);
  }

  .collapsed-timer {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--fg);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    flex: none;
  }

  .collapsed-lyric {
    max-width: 132px;
    font-size: 10px;
    line-height: 1.25;
    color: var(--fg);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    animation: lyric-in 0.28s ease;
  }

  @keyframes lyric-in {
    from {
      opacity: 0;
      transform: translateY(3px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .collapsed-timer :global(svg) {
    color: var(--accent);
  }

  .compact-stats {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 8.5px;
    font-variant-numeric: tabular-nums;
    color: var(--muted);
    white-space: nowrap;
    flex: none;
  }

  .compact-stats :global(svg) {
    color: var(--accent);
  }

  .agent-indicator {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
    flex: none;
    box-shadow: 0 0 7px color-mix(in srgb, var(--accent) 70%, transparent);
  }

  .agent-indicator.working {
    background: #f5b301;
    box-shadow: 0 0 8px #f5b301;
    animation: agent-blink 1.2s ease-in-out infinite;
  }

  .agent-indicator.error {
    background: var(--danger);
    box-shadow: 0 0 8px var(--danger);
  }

  .agent-indicator.attention {
    background: var(--accent);
    animation: agent-blink 0.8s ease-in-out infinite;
  }

  @keyframes agent-blink {
    0%, 100% { opacity: 0.4; }
    50% { opacity: 1; }
  }

  .banner-card {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 10px 8px 14px;
    border: none;
    background: transparent;
    color: var(--fg);
    text-align: left;
    width: 100%;
  }

  .banner-open {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    border: none;
    background: transparent;
    color: inherit;
    text-align: left;
    padding: 0;
    cursor: pointer;
  }

  .banner-actions {
    display: flex;
    gap: 3px;
    flex: none;
  }

  .banner-action {
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--fg);
    border-radius: 7px;
    padding: 4px 7px;
    font-size: 9.5px;
    min-height: 24px;
    white-space: nowrap;
    transition: background 0.14s ease, border-color 0.14s ease;
  }

  .banner-action:hover {
    background: var(--surface-strong);
  }

  .banner-action.primary {
    background: var(--accent);
    border-color: transparent;
    color: #fff;
  }

  .banner-icon {
    width: 34px;
    height: 34px;
    border-radius: 50%;
    background: var(--surface-strong);
    display: grid;
    place-items: center;
    color: var(--accent);
    flex: none;
  }

  .banner-icon.reminder {
    background: #f5b301;
    color: #1a1200;
  }

  .banner-icon.download {
    background: var(--accent);
    color: #fff;
  }

  .banner-icon.phone {
    background: #22c55e;
    color: #04120a;
  }

  .banner-icon.screenshot {
    background: #8b5cf6;
    color: #fff;
  }

  .banner-icon.timer {
    background: var(--accent);
    color: #fff;
  }

  .banner-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .banner-text strong {
    font-size: 12px;
    font-weight: 650;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .banner-text span {
    font-size: 10px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .chrome {
    flex: none;
    height: 34px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 8px;
    gap: 6px;
    border-bottom: 1px solid var(--border);
    cursor: grab;
    z-index: 1;
  }

  .chrome:active {
    cursor: grabbing;
  }

  .tabs {
    display: flex;
    align-items: center;
    gap: 1px;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .tabs::-webkit-scrollbar {
    display: none;
  }

  .chrome-actions {
    display: flex;
    align-items: center;
    gap: 1px;
    flex: none;
  }

  .chrome-perf {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    flex: none;
    padding: 3px 7px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: color-mix(in srgb, var(--surface) 72%, transparent);
    font-size: 8.5px;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.01em;
    color: var(--muted);
    white-space: nowrap;
  }

  .chrome-fps {
    color: color-mix(in srgb, var(--live-accent) 78%, var(--fg));
  }

  .chrome-temp {
    color: var(--muted);
  }

  .tab {
    width: 28px;
    height: 26px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    transition: background 0.14s ease, color 0.14s ease;
    flex: none;
  }

  .tab :global(svg) {
    transition: transform 0.18s cubic-bezier(0.34, 1.4, 0.64, 1), color 0.18s ease;
  }

  .tab:hover :global(svg) {
    transform: translateY(-1px) scale(1.12);
  }

  .tab.active :global(svg) {
    color: var(--accent);
  }

  .tab.active {
    background: var(--surface-strong);
  }

  .tab:hover {
    background: var(--surface);
    color: var(--fg);
  }

  .tab.active {
    background: var(--surface-strong);
    color: var(--fg);
  }

  .tab.action {
    color: var(--muted);
  }

  .body {
    flex: 1;
    min-height: 0;
    padding: 10px;
    overflow: hidden;
    animation: body-slide var(--motion-duration, 380ms) var(--motion-ease, ease);
    z-index: 1;
  }

  :global(:root[data-motion="cinematic"]) .body {
    animation: body-rise var(--motion-duration, 480ms) var(--motion-ease, ease);
  }

  :global(:root[data-motion="snappy"]) .body {
    animation: body-pop var(--motion-fast, 180ms) cubic-bezier(0.22, 1.5, 0.36, 1);
  }

  @keyframes body-slide {
    from {
      opacity: 0;
      transform: translateX(calc(var(--dir, 1) * 9px));
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }

  @keyframes body-rise {
    from {
      opacity: 0;
      transform: translateY(9px) translateX(calc(var(--dir, 1) * 7px));
    }
    to {
      opacity: 1;
      transform: translateY(0) translateX(0);
    }
  }

  @keyframes body-pop {
    from {
      opacity: 0;
      transform: translateX(calc(var(--dir, 1) * 5px)) scale(0.985);
    }
    to {
      opacity: 1;
      transform: translateX(0) scale(1);
    }
  }

  .body-loading {
    display: grid;
    place-items: center;
    height: 100%;
    color: var(--muted);
    font-size: 11px;
  }

  :global(:root[data-reduced-motion="true"]) .island {
    transition-duration: 1ms;
  }

  :global(:root[data-reduced-motion="true"]) .island.attention {
    animation: none;
  }

  :global(:root[data-reduced-motion="true"]) .collapsed-lyric {
    animation: none;
  }

  :global(:root[data-reduced-motion="true"]) .body {
    animation: none;
  }

  :global(:root[data-reduced-motion="true"]) .ambient-layer,
  :global(:root[data-reduced-motion="true"]) .ripple {
    display: none;
  }

  :global(:root[data-reduced-motion="true"]) .collapsed,
  :global(:root[data-reduced-motion="true"]) .chrome,
  :global(:root[data-reduced-motion="true"]) .body {
    transform: none;
  }

  :global(:root[data-reduced-motion="true"]) .agent-indicator {
    animation: none;
  }
</style>
