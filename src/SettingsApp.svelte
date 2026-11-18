<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import Icon from "./lib/components/Icon.svelte";
  import HotkeyInput from "./lib/components/HotkeyInput.svelte";
  import Clock from "./lib/components/Clock.svelte";
  import Character from "./lib/components/Character.svelte";
  import WeatherBadge from "./lib/components/WeatherBadge.svelte";
  import { MODULES } from "./lib/modules/registry";
  import {
    DEFAULT_SETTINGS,
    store,
    t,
    updateModule,
    updateSettings,
  } from "./lib/store.svelte";
  import { localeName, locales, type Locale } from "./lib/i18n";
  import { ensureWeather } from "./lib/weather.svelte";
  import { checkForUpdates, installUpdate, updates } from "./lib/updates.svelte";
  import {
    DEFAULT_WIDGETS,
    SIZE_CYCLE,
    WIDGET_META,
    widgetId,
    type WidgetConfig,
    type WidgetKind,
  } from "./lib/widgets/types";

  type Section =
    | "general"
    | "appearance"
    | "home"
    | "tray"
    | "modules"
    | "ai"
    | "hotkeys"
    | "about";

  const SECTIONS: Array<{ id: Section; icon: string; label: string }> = [
    { id: "general", icon: "sliders", label: "settings.section.general" },
    { id: "appearance", icon: "sun", label: "settings.section.appearance" },
    { id: "home", icon: "layout", label: "settings.section.home" },
    { id: "tray", icon: "settings", label: "settings.section.tray" },
    { id: "modules", icon: "home", label: "settings.section.modules" },
    { id: "ai", icon: "sparkles", label: "settings.section.ai" },
    { id: "hotkeys", icon: "terminal", label: "settings.section.hotkeys" },
    { id: "about", icon: "eye", label: "settings.section.about" },
  ];

  const FONTS = ["system", "Inter", "Manrope", "Nunito", "Segoe UI", "Arial", "Calibri"];

  const CHARACTERS = [
    { id: "none", label: "settings.char_none" },
    { id: "eyes", label: "settings.char_eyes" },
    { id: "dots", label: "settings.char_dots" },
    { id: "pill", label: "settings.char_pill" },
    { id: "hud", label: "settings.char_hud" },
    { id: "cat", label: "settings.char_cat" },
    { id: "orb", label: "settings.char_orb" },
    { id: "robot", label: "settings.char_robot" },
    { id: "ghost", label: "settings.char_ghost" },
    { id: "bars", label: "settings.char_bars" },
    { id: "spark", label: "settings.char_spark" },
  ];

  const SCENES = [
    { id: "none", label: "settings.scene_none" },
    { id: "stars", label: "settings.scene_stars" },
    { id: "aurora", label: "settings.scene_aurora" },
    { id: "bubbles", label: "settings.scene_bubbles" },
    { id: "matrix", label: "settings.scene_matrix" },
    { id: "gif", label: "settings.scene_gif" },
  ];

  const PROVIDERS = [
    { id: "ollama", label: "Ollama (local)", base: "http://127.0.0.1:11434/v1", model: "llama3.2" },
    { id: "lmstudio", label: "LM Studio (local)", base: "http://127.0.0.1:1234/v1", model: "local-model" },
    { id: "openai", label: "OpenAI", base: "https://api.openai.com/v1", model: "gpt-4o-mini" },
    { id: "openrouter", label: "OpenRouter", base: "https://openrouter.ai/api/v1", model: "openai/gpt-4o-mini" },
    { id: "groq", label: "Groq", base: "https://api.groq.com/openai/v1", model: "llama-3.3-70b-versatile" },
    { id: "custom", label: "Custom", base: "", model: "" },
  ];

  const THEMES = [
    { id: "classic", label: "settings.theme_classic" },
    { id: "night", label: "settings.theme_night" },
    { id: "light", label: "settings.theme_light" },
    { id: "aurora", label: "settings.theme_aurora" },
    { id: "sunset", label: "settings.theme_sunset" },
    { id: "forest", label: "settings.theme_forest" },
    { id: "mono", label: "settings.theme_mono" },
    { id: "rose", label: "settings.theme_rose" },
    { id: "system", label: "settings.theme_system" },
    { id: "custom", label: "settings.theme_custom" },
  ];

  let section = $state<Section>("general");
  let probeBusy = $state(false);
  let probeMessage = $state("");
  let probeOk = $state(false);
  let addKind = $state<WidgetKind>("clock");
  let hotkeyStatus = $state<{
    registered: Array<{ action: string; keys: string }>;
    failed: Array<{ action: string; keys: string; error: string }>;
    actualToggle: string | null;
  }>({ registered: [], failed: [], actualToggle: null });

  const widgetKinds = Object.keys(WIDGET_META) as WidgetKind[];

  function addWidget(): void {
    const meta = WIDGET_META[addKind];
    const next: WidgetConfig[] = [
      ...store.settings.widgets,
      { id: widgetId(), kind: addKind, size: meta.defaultSize },
    ];
    updateSettings({ widgets: next });
  }

  function removeWidget(id: string): void {
    updateSettings({ widgets: store.settings.widgets.filter((widget) => widget.id !== id) });
  }

  function setWidgetSize(id: string, size: string): void {
    updateSettings({
      widgets: store.settings.widgets.map((widget) =>
        widget.id === id ? { ...widget, size: size as WidgetConfig["size"] } : widget,
      ),
    });
  }

  let dragIndex = $state<number | null>(null);
  let dropIndex = $state<number | null>(null);

  function dragStart(event: DragEvent, index: number): void {
    dragIndex = index;
    dropIndex = index;
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = "move";
      event.dataTransfer.setData("text/plain", String(index));
    }
  }

  function dragOver(event: DragEvent, index: number): void {
    if (dragIndex === null) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
    if (dropIndex !== index) dropIndex = index;
  }

  function dragLeave(index: number): void {
    if (dropIndex === index) dropIndex = null;
  }

  function dropWidget(event: DragEvent, index: number): void {
    event.preventDefault();
    const from = dragIndex;
    dragIndex = null;
    dropIndex = null;
    if (from === null || from === index) return;
    const next = [...store.settings.widgets];
    const [moved] = next.splice(from, 1);
    if (!moved) return;
    next.splice(index, 0, moved);
    updateSettings({ widgets: next });
  }

  function dragEnd(): void {
    dragIndex = null;
    dropIndex = null;
  }

  const hasFpsWidget = $derived(store.settings.widgets.some((widget) => widget.kind === "fps"));

  function resetWidgets(): void {
    updateSettings({ widgets: DEFAULT_WIDGETS.map((widget) => ({ ...widget })) });
  }

  function chooseProvider(id: string): void {
    const preset = PROVIDERS.find((provider) => provider.id === id);
    updateSettings({
      chat_provider: id,
      ...(preset ? { chat_base_url: preset.base, chat_model: preset.model } : {}),
    });
    probeMessage = "";
    probeOk = false;
  }

  async function probe(): Promise<void> {
    if (probeBusy) return;
    probeBusy = true;
    probeMessage = t("settings.ai_testing");
    probeOk = false;
    try {
      await invoke("save_settings", { settings: store.settings });
      const result = await invoke<{ models: string[]; model: string }>("chat_probe");
      const count = result.models.length;
      probeMessage = count
        ? t("settings.ai_ok", { count })
        : t("settings.ai_ok_none", { model: result.model });
      probeOk = true;
    } catch (cause) {
      probeMessage = String(cause);
      probeOk = false;
    } finally {
      probeBusy = false;
    }
  }

  function numberValue(event: Event): number {
    const input = event.currentTarget as HTMLInputElement;
    const value = Number(input.value);
    return Number.isFinite(value) ? value : 0;
  }

  onMount(() => {
    void ensureWeather();
    void invoke<typeof hotkeyStatus>("hotkeys_status")
      .then((status) => (hotkeyStatus = status))
      .catch(() => {});
    const off = listen<typeof hotkeyStatus>("hotkeys-status", (event) => {
      hotkeyStatus = event.payload;
    });
    return () => {
      void off.then((unlisten) => unlisten());
    };
  });
</script>

<div class="settings">
  <aside class="sidebar">
    <div class="brand">
      <span class="logo"><Icon name="sparkles" size={16} /></span>
      <div class="brand-text">
        <span class="name">{t("app.name")}</span>
        <span class="tagline">{t("app.tagline")}</span>
      </div>
    </div>

    <nav class="nav" aria-label={t("settings.title")}>
      {#each SECTIONS as item (item.id)}
        <button
          class="nav-item"
          class:active={section === item.id}
          onclick={() => (section = item.id)}
        >
          <Icon name={item.icon} size={15} />
          <span>{t(item.label)}</span>
        </button>
      {/each}
    </nav>

    <div class="sidebar-footer">{t("settings.about_license")}</div>
  </aside>

  <main class="content">
    <div class="preview" aria-label={t("settings.preview")}>
      <div class="preview-island">
        <span class="preview-clock"><Clock /></span>
        <Character />
        <WeatherBadge />
      </div>
      <span class="preview-label">{t("settings.preview")}</span>
    </div>

    <div class="scroll">
      {#if section === "general"}
        <section class="group">
          <h2>{t("settings.section.general")}</h2>

          <label class="row">
            <span class="row-label">{t("settings.language")}</span>
            <select
              value={store.settings.language}
              onchange={(event) =>
                updateSettings({ language: (event.currentTarget as HTMLSelectElement).value })}
            >
              {#each locales as code}
                <option value={code}>{localeName(code as Locale)}</option>
              {/each}
            </select>
          </label>

          <label class="row">
            <span class="row-label">{t("settings.autostart")}</span>
            <input
              type="checkbox"
              class="switch"
              checked={store.settings.autostart}
              onchange={(event) =>
                updateSettings({ autostart: (event.currentTarget as HTMLInputElement).checked })}
            />
          </label>

          <label class="row">
            <span class="row-label">{t("settings.behavior")}</span>
            <select
              value={store.settings.behavior}
              onchange={(event) =>
                updateSettings({ behavior: (event.currentTarget as HTMLSelectElement).value })}
            >
              <option value="hover">{t("settings.behavior_hover")}</option>
              <option value="click">{t("settings.behavior_click")}</option>
              <option value="double">{t("settings.behavior_double")}</option>
              <option value="always">{t("settings.behavior_always")}</option>
            </select>
          </label>

          <label class="row">
            <span class="row-label">{t("settings.dock")}</span>
            <select
              value={store.settings.island_dock}
              onchange={(event) =>
                updateSettings({ island_dock: (event.currentTarget as HTMLSelectElement).value })}
            >
              <option value="top">{t("settings.dock_top")}</option>
              <option value="left">{t("settings.dock_left")}</option>
              <option value="right">{t("settings.dock_right")}</option>
              <option value="bottom">{t("settings.dock_bottom")}</option>
              <option value="free">{t("settings.dock_free")}</option>
            </select>
          </label>

          <label class="row">
            <span class="row-label">{t("settings.island_pinned")}</span>
            <input
              type="checkbox"
              class="switch"
              checked={store.settings.island_pinned}
              onchange={(event) =>
                updateSettings({
                  island_pinned: (event.currentTarget as HTMLInputElement).checked,
                })}
            />
          </label>
          <p class="hint">{t("settings.pin_hint")}</p>

          <label class="row">
            <span class="row-label">{t("settings.hover_delay")}</span>
            <span class="field">
              <input
                type="number"
                min="0"
                max="2000"
                step="10"
                value={store.settings.hover_delay_ms}
                oninput={(event) => updateSettings({ hover_delay_ms: numberValue(event) })}
              />
              <span class="unit">ms</span>
            </span>
          </label>

          <label class="row">
            <span class="row-label">{t("settings.close_delay")}</span>
            <span class="field">
              <input
                type="number"
                min="0"
                max="5000"
                step="50"
                value={store.settings.close_delay_ms}
                oninput={(event) => updateSettings({ close_delay_ms: numberValue(event) })}
              />
              <span class="unit">ms</span>
            </span>
          </label>

          <label class="row">
            <span class="row-label">{t("settings.autohide_fullscreen")}</span>
            <input
              type="checkbox"
              class="switch"
              checked={store.settings.autohide_fullscreen}
              onchange={(event) =>
                updateSettings({
                  autohide_fullscreen: (event.currentTarget as HTMLInputElement).checked,
                })}
            />
          </label>

          <div class="row static">
            <span class="row-label">{t("settings.position")}</span>
            <span class="field">
              <span class="unit">
                {store.settings.island_pos_x === null || store.settings.island_pos_y === null
                  ? t("settings.position_auto")
                  : `${store.settings.island_pos_x}, ${store.settings.island_pos_y}`}
              </span>
              <button class="btn" onclick={() => void invoke("reset_island_position")}>
                <Icon name="refresh" size={12} />
                {t("settings.reset_position")}
              </button>
            </span>
          </div>
          <p class="hint">{t("settings.drag_hint")}</p>

          <label class="row">
            <span class="row-label">{t("settings.show_seconds")}</span>
            <input
              type="checkbox"
              class="switch"
              checked={store.settings.show_seconds}
              onchange={(event) =>
                updateSettings({ show_seconds: (event.currentTarget as HTMLInputElement).checked })}
            />
          </label>

          <label class="row">
            <span class="row-label">{t("settings.show_battery")}</span>
            <input
              type="checkbox"
              class="switch"
              checked={store.settings.show_battery}
              onchange={(event) =>
                updateSettings({ show_battery: (event.currentTarget as HTMLInputElement).checked })}
            />
          </label>

          <label class="row">
            <span class="row-label">{t("settings.show_stats_collapsed")}</span>
            <input
              type="checkbox"
              class="switch"
              checked={store.settings.show_stats_collapsed}
              onchange={(event) =>
                updateSettings({
                  show_stats_collapsed: (event.currentTarget as HTMLInputElement).checked,
                })}
            />
          </label>

          <label class="row">
            <span class="row-label">{t("settings.clipboard_watch")}</span>
            <input
              type="checkbox"
              class="switch"
              checked={store.settings.clipboard_watch}
              onchange={(event) =>
                updateSettings({
                  clipboard_watch: (event.currentTarget as HTMLInputElement).checked,
                })}
            />
          </label>

          <label class="row">
            <span class="row-label">{t("settings.clipboard_images")}</span>
            <input
              type="checkbox"
              class="switch"
              checked={store.settings.clipboard_images}
              onchange={(event) =>
                updateSettings({
                  clipboard_images: (event.currentTarget as HTMLInputElement).checked,
                })}
            />
          </label>

          <label class="row">
            <span class="row-label">{t("settings.clipboard_ignore_secrets")}</span>
            <input
              type="checkbox"
              class="switch"
              checked={store.settings.clipboard_ignore_secrets}
              onchange={(event) =>
                updateSettings({
                  clipboard_ignore_secrets: (event.currentTarget as HTMLInputElement).checked,
                })}
            />
          </label>
        </section>
      {:else if section === "appearance"}
        <section class="group">
          <h2>{t("settings.section.appearance")}</h2>

          <label class="row">
            <span class="row-label">{t("settings.theme")}</span>
            <select
              value={store.settings.theme}
              onchange={(event) =>
                updateSettings({ theme: (event.currentTarget as HTMLSelectElement).value })}
            >
              {#each THEMES as theme}
                <option value={theme.id}>{t(theme.label)}</option>
              {/each}
            </select>
          </label>

          <label class="row">
            <span class="row-label">{t("settings.accent_color")}</span>
            <span class="color-field">
              <input
                type="color"
                value={store.settings.accent_color}
                oninput={(event) =>
                  updateSettings({ accent_color: (event.currentTarget as HTMLInputElement).value })}
              />
              <code>{store.settings.accent_color}</code>
            </span>
          </label>

          <label class="row">
            <span class="row-label">{t("settings.visualizer")}</span>
            <select
              value={store.settings.visualizer_style}
              onchange={(event) =>
                updateSettings({
                  visualizer_style: (event.currentTarget as HTMLSelectElement).value,
                })}
            >
              <option value="auto">{t("settings.visualizer_auto")}</option>
              <option value="bars">{t("settings.visualizer_bars")}</option>
              <option value="wave">{t("settings.visualizer_wave")}</option>
              <option value="pulse">{t("settings.visualizer_pulse")}</option>
              <option value="orbit">{t("settings.visualizer_orbit")}</option>
              <option value="sparks">{t("settings.visualizer_sparks")}</option>
              <option value="aurora">{t("settings.visualizer_aurora")}</option>
              <option value="meteors">{t("settings.visualizer_meteors")}</option>
              <option value="glow">{t("settings.visualizer_glow")}</option>
              <option value="ribbon">{t("settings.visualizer_ribbon")}</option>
            </select>
          </label>

          <label class="row">
            <span class="row-label">{t("settings.motion")}</span>
            <select
              value={store.settings.motion_profile}
              onchange={(event) =>
                updateSettings({
                  motion_profile: (event.currentTarget as HTMLSelectElement).value,
                })}
            >
              <option value="soft">{t("settings.motion_soft")}</option>
              <option value="snappy">{t("settings.motion_snappy")}</option>
              <option value="cinematic">{t("settings.motion_cinematic")}</option>
              <option value="calm">{t("settings.motion_calm")}</option>
            </select>
          </label>

          <label class="row">
            <span class="row-label">{t("settings.scene")}</span>
            <select
              value={store.settings.island_scene}
              onchange={(event) =>
                updateSettings({
                  island_scene: (event.currentTarget as HTMLSelectElement).value,
                })}
            >
              {#each SCENES as scene}
                <option value={scene.id}>{t(scene.label)}</option>
              {/each}
            </select>
          </label>

          {#if store.settings.island_scene === "gif"}
            <div class="row static">
              <span class="row-label">{t("settings.scene_gif_url")}</span>
              <span class="field">
                <input
                  class="wide"
                  type="url"
                  placeholder="https://…/loop.gif"
                  value={store.settings.island_gif_url}
                  oninput={(event) =>
                    updateSettings({
                      island_gif_url: (event.currentTarget as HTMLInputElement).value,
                    })}
                />
              </span>
            </div>
            <p class="hint">{t("settings.scene_gif_hint")}</p>
          {/if}

          <label class="row">
            <span class="row-label">{t("settings.lyrics_style")}</span>
            <select
              value={store.settings.lyrics_style}
              onchange={(event) =>
                updateSettings({
                  lyrics_style: (event.currentTarget as HTMLSelectElement).value,
                })}
            >
              <option value="slide">{t("settings.lyrics_slide")}</option>
              <option value="pop">{t("settings.lyrics_pop")}</option>
              <option value="blur">{t("settings.lyrics_blur")}</option>
              <option value="karaoke">{t("settings.lyrics_karaoke")}</option>
            </select>
          </label>

          {#if store.settings.theme === "custom"}
            <label class="row">
              <span class="row-label">{t("settings.custom_bg")}</span>
              <span class="color-field">
                <input
                  type="color"
                  value={store.settings.custom_bg}
                  oninput={(event) =>
                    updateSettings({ custom_bg: (event.currentTarget as HTMLInputElement).value })}
                />
                <code>{store.settings.custom_bg}</code>
              </span>
            </label>

            <label class="row">
              <span class="row-label">{t("settings.custom_fg")}</span>
              <span class="color-field">
                <input
                  type="color"
                  value={store.settings.custom_fg}
                  oninput={(event) =>
                    updateSettings({ custom_fg: (event.currentTarget as HTMLInputElement).value })}
                />
                <code>{store.settings.custom_fg}</code>
              </span>
            </label>
          {/if}

          <label class="row">
            <span class="row-label">{t("settings.font")}</span>
            <select
              value={store.settings.font}
              onchange={(event) =>
                updateSettings({ font: (event.currentTarget as HTMLSelectElement).value })}
            >
              {#each FONTS as font}
                <option value={font}>{font === "system" ? t("settings.font_system") : font}</option>
              {/each}
            </select>
          </label>

          <label class="row">
            <span class="row-label">{t("settings.radius")}</span>
            <span class="field">
              <input
                type="range"
                min="0"
                max="32"
                value={store.settings.radius}
                oninput={(event) => updateSettings({ radius: numberValue(event) })}
              />
              <span class="unit">{store.settings.radius}px</span>
            </span>
          </label>

          <label class="row">
            <span class="row-label">{t("settings.character")}</span>
            <select
              value={store.settings.character}
              onchange={(event) =>
                updateSettings({ character: (event.currentTarget as HTMLSelectElement).value })}
            >
              {#each CHARACTERS as character}
                <option value={character.id}>{t(character.label)}</option>
              {/each}
            </select>
          </label>

          <label class="row">
            <span class="row-label">{t("settings.character_speed")}</span>
            <span class="field">
              <input
                type="range"
                min="0.2"
                max="3"
                step="0.1"
                value={store.settings.character_speed}
                oninput={(event) => updateSettings({ character_speed: numberValue(event) })}
              />
              <span class="unit">{store.settings.character_speed.toFixed(1)}×</span>
            </span>
          </label>

          <label class="row">
            <span class="row-label">{t("settings.reduced_motion")}</span>
            <input
              type="checkbox"
              class="switch"
              checked={store.settings.reduced_motion}
              onchange={(event) =>
                updateSettings({
                  reduced_motion: (event.currentTarget as HTMLInputElement).checked,
                })}
            />
          </label>
        </section>
      {:else if section === "home"}
        <section class="group">
          <h2>{t("settings.section.home")}</h2>
          <p class="hint">{t("settings.home_hint")}</p>

          <label class="row">
            <span class="row-label">{t("settings.auto_music_panel")}</span>
            <input
              type="checkbox"
              class="switch"
              checked={store.settings.auto_music_panel}
              onchange={(event) =>
                updateSettings({
                  auto_music_panel: (event.currentTarget as HTMLInputElement).checked,
                })}
            />
          </label>

          <label class="row">
            <span class="row-label">{t("settings.karaoke_line")}</span>
            <input
              type="checkbox"
              class="switch"
              checked={store.settings.karaoke_line}
              onchange={(event) =>
                updateSettings({
                  karaoke_line: (event.currentTarget as HTMLInputElement).checked,
                })}
            />
          </label>

          <div class="widget-list" role="list">
            {#each store.settings.widgets as widget, index (widget.id)}
              <div
                class="widget-row"
                role="listitem"
                class:dragging={dragIndex === index}
                class:drop-target={dragIndex !== null && dropIndex === index && dragIndex !== index}
                draggable="true"
                ondragstart={(event) => dragStart(event, index)}
                ondragover={(event) => dragOver(event, index)}
                ondragleave={() => dragLeave(index)}
                ondrop={(event) => dropWidget(event, index)}
                ondragend={dragEnd}
              >
                <span class="widget-grip" title={t("widget.drag_hint")}>
                  <Icon name="grip" size={13} />
                </span>
                <span class="widget-icon">
                  <Icon name={WIDGET_META[widget.kind].icon} size={14} />
                </span>
                <span class="widget-name">{t(WIDGET_META[widget.kind].label)}</span>
                <select
                  class="widget-size-select"
                  value={widget.size}
                  title={t("widget.resize")}
                  aria-label={t("widget.resize")}
                  onchange={(event) => setWidgetSize(widget.id, event.currentTarget.value)}
                >
                  {#each SIZE_CYCLE as size (size)}
                    <option value={size}>{t(`widget.size_${size}`)}</option>
                  {/each}
                </select>
                <button
                  class="btn icon-only"
                  onclick={() => removeWidget(widget.id)}
                  title={t("widget.remove")}
                  aria-label={t("widget.remove")}
                >
                  <Icon name="trash" size={12} />
                </button>
              </div>
            {/each}
            {#if !store.settings.widgets.length}
              <p class="text">{t("widget.empty")}</p>
            {/if}
          </div>
          <p class="hint">{t("widget.drag_hint")}</p>

          {#if hasFpsWidget}
            <label class="row">
              <span class="row-label">{t("settings.fps_metric")}</span>
              <select
                value={store.settings.fps_metric}
                onchange={(event) =>
                  updateSettings({
                    fps_metric: event.currentTarget.value === "gpu" ? "gpu" : "cpu",
                  })}
              >
                <option value="cpu">CPU</option>
                <option value="gpu">GPU</option>
              </select>
            </label>
            <p class="hint">{t("settings.fps_metric_hint")}</p>
          {/if}

          <div class="row static">
            <span class="row-label">{t("widget.add")}</span>
            <span class="field">
              <select
                value={addKind}
                onchange={(event) =>
                  (addKind = (event.currentTarget as HTMLSelectElement).value as WidgetKind)}
              >
                {#each widgetKinds as kind (kind)}
                  <option value={kind}>{t(WIDGET_META[kind].label)}</option>
                {/each}
              </select>
              <button class="btn" onclick={addWidget}>
                <Icon name="plus" size={12} />
                {t("widget.add")}
              </button>
            </span>
          </div>

          <div class="row static">
            <span class="row-label">{t("widget.reset")}</span>
            <button class="btn" onclick={resetWidgets}>
              <Icon name="refresh" size={12} />
              {t("common.reset")}
            </button>
          </div>

          <p class="hint">{t("settings.home_edit_hint")}</p>
        </section>
      {:else if section === "tray"}
        <section class="group">
          <h2>{t("settings.section.tray")}</h2>
          <p class="hint">{t("settings.tray_hint")}</p>

          <label class="row">
            <span class="row-label">{t("settings.tray_enabled")}</span>
            <input
              type="checkbox"
              class="switch"
              checked={store.settings.tray_enabled}
              onchange={(event) =>
                updateSettings({ tray_enabled: (event.currentTarget as HTMLInputElement).checked })}
            />
          </label>

          <label class="row">
            <span class="row-label">{t("settings.autostart")}</span>
            <input
              type="checkbox"
              class="switch"
              checked={store.settings.autostart}
              onchange={(event) =>
                updateSettings({ autostart: (event.currentTarget as HTMLInputElement).checked })}
            />
          </label>
        </section>
      {:else if section === "modules"}
        <section class="group">
          <h2>{t("settings.section.modules")}</h2>
          <p class="hint">{t("settings.modules_hint")}</p>

          <div class="module-grid">
            {#each MODULES as module (module.id)}
              <label class="module" class:disabled={module.id === "home"}>
                <span class="module-icon"><Icon name={module.icon} size={16} /></span>
                <span class="module-body">
                  <span class="module-name">{t(`nav.${module.id}`)}</span>
                </span>
                <input
                  type="checkbox"
                  class="switch"
                  disabled={module.id === "home"}
                  checked={module.id === "home" || store.settings.modules[module.id] !== false}
                  onchange={(event) =>
                    updateModule(module.id, (event.currentTarget as HTMLInputElement).checked)}
                />
              </label>
            {/each}
          </div>
        </section>
      {:else if section === "ai"}
        <section class="group">
          <h2>{t("settings.section.ai")}</h2>
          <p class="hint">{t("settings.ai_hint")}</p>

          <label class="row">
            <span class="row-label">{t("settings.ai_provider")}</span>
            <select
              value={store.settings.chat_provider}
              onchange={(event) => chooseProvider((event.currentTarget as HTMLSelectElement).value)}
            >
              {#each PROVIDERS as provider}
                <option value={provider.id}>{provider.label}</option>
              {/each}
            </select>
          </label>

          <label class="row">
            <span class="row-label">{t("settings.ai_base")}</span>
            <input
              type="text"
              class="wide"
              value={store.settings.chat_base_url}
              placeholder="http://127.0.0.1:11434/v1"
              oninput={(event) =>
                updateSettings({ chat_base_url: (event.currentTarget as HTMLInputElement).value })}
            />
          </label>

          <label class="row">
            <span class="row-label">{t("settings.ai_model")}</span>
            <input
              type="text"
              class="wide"
              value={store.settings.chat_model}
              placeholder="llama3.2"
              oninput={(event) =>
                updateSettings({ chat_model: (event.currentTarget as HTMLInputElement).value })}
            />
          </label>

          <label class="row">
            <span class="row-label">{t("settings.ai_key")}</span>
            <input
              type="password"
              class="wide"
              value={store.settings.chat_api_key}
              autocomplete="off"
              placeholder={t("settings.ai_key_placeholder")}
              oninput={(event) =>
                updateSettings({ chat_api_key: (event.currentTarget as HTMLInputElement).value })}
            />
          </label>

          <p class="text">{t("settings.ai_privacy")}</p>

          <div class="row static">
            <span class="row-label">{t("settings.ai_test")}</span>
            <span class="field">
              <button class="btn" onclick={probe} disabled={probeBusy}>
                <Icon name="wifi" size={13} />
                {probeBusy ? t("settings.ai_testing") : t("settings.ai_test_button")}
              </button>
            </span>
          </div>
          {#if probeMessage}
            <p class="text" class:probe-ok={probeOk} class:probe-error={!probeOk}>{probeMessage}</p>
          {/if}
        </section>
      {:else if section === "hotkeys"}
        <section class="group">
          <h2>{t("settings.section.hotkeys")}</h2>

          <div class="row">
            <span class="row-label">{t("settings.hotkey_toggle")}</span>
            <HotkeyInput
              value={store.settings.hotkey}
              label={t("settings.hotkey_toggle")}
              onchange={(value) => updateSettings({ hotkey: value })}
            />
          </div>

          <div class="row">
            <span class="row-label">{t("settings.hotkey_expand")}</span>
            <HotkeyInput
              value={store.settings.hotkey_expand}
              label={t("settings.hotkey_expand")}
              onchange={(value) => updateSettings({ hotkey_expand: value })}
            />
          </div>

          <div class="row">
            <span class="row-label">{t("settings.hotkey_collapse")}</span>
            <HotkeyInput
              value={store.settings.hotkey_collapse}
              label={t("settings.hotkey_collapse")}
              onchange={(value) => updateSettings({ hotkey_collapse: value })}
            />
          </div>

          <div class="row">
            <span class="row-label">{t("settings.hotkey_open_settings")}</span>
            <HotkeyInput
              value={store.settings.hotkey_settings}
              label={t("settings.hotkey_open_settings")}
              onchange={(value) => updateSettings({ hotkey_settings: value })}
            />
          </div>

          <div class="row">
            <span class="row-label">{t("settings.hotkey_screenshot")}</span>
            <HotkeyInput
              value={store.settings.hotkey_screenshot}
              label={t("settings.hotkey_screenshot")}
              onchange={(value) => updateSettings({ hotkey_screenshot: value })}
            />
          </div>

          <div class="row">
            <span class="row-label">{t("settings.hotkey_play_pause")}</span>
            <HotkeyInput
              value={store.settings.hotkey_play_pause}
              label={t("settings.hotkey_play_pause")}
              onchange={(value) => updateSettings({ hotkey_play_pause: value })}
            />
          </div>

          <div class="row">
            <span class="row-label">{t("settings.hotkey_next")}</span>
            <HotkeyInput
              value={store.settings.hotkey_next}
              label={t("settings.hotkey_next")}
              onchange={(value) => updateSettings({ hotkey_next: value })}
            />
          </div>

          <div class="row">
            <span class="row-label">{t("settings.hotkey_prev")}</span>
            <HotkeyInput
              value={store.settings.hotkey_prev}
              label={t("settings.hotkey_prev")}
              onchange={(value) => updateSettings({ hotkey_prev: value })}
            />
          </div>

          <div class="row static">
            <span class="row-label">{t("settings.hotkeys_reset")}</span>
            <button
              class="btn"
              onclick={() =>
                updateSettings({
                  hotkey: "Alt+Space",
                  hotkey_expand: "",
                  hotkey_collapse: "",
                  hotkey_settings: "",
                  hotkey_screenshot: "",
                  hotkey_play_pause: "",
                  hotkey_next: "",
                  hotkey_prev: "",
                })}
            >
              <Icon name="refresh" size={12} />
              {t("common.reset")}
            </button>
          </div>

          {#if hotkeyStatus.failed.length}
            <div class="error" role="alert">
              {t("settings.hotkeys_conflict")}:
              {hotkeyStatus.failed.map((item) => `${item.action} (${item.keys}): ${item.error}`).join("; ")}
            </div>
          {:else if hotkeyStatus.registered.length}
            <p class="hint">
              {t("settings.hotkeys_active")}:
              {hotkeyStatus.registered.map((item) => `${item.action} — ${item.keys}`).join(" · ")}
            </p>
          {/if}

          <p class="hint">{t("settings.hotkeys_hint")}</p>
        </section>
      {:else}
        <section class="group">
          <h2>{t("settings.section.about")}</h2>

          <div class="about-row">
            <span>{t("settings.about_version", { version: store.version })}</span>
            <button
              class="btn update-btn"
              disabled={updates.phase === "checking" || updates.phase === "downloading"}
              onclick={() => void checkForUpdates()}
            >
              <Icon name="refresh" size={13} />
              {updates.phase === "checking" ? t("settings.update_checking") : t("settings.update_check")}
            </button>
          </div>

          {#if updates.phase === "current"}
            <p class="hint update-hint">{t("settings.update_current")}</p>
          {:else if updates.phase === "available"}
            <div class="update-card">
              <span class="update-title">{t("settings.update_available", { version: updates.version })}</span>
              {#if updates.notes}
                <p class="update-notes">{updates.notes}</p>
              {/if}
              <button class="btn" onclick={() => void installUpdate()}>
                <Icon name="download" size={13} />
                {t("settings.update_install")}
              </button>
            </div>
          {:else if updates.phase === "downloading"}
            <div class="update-card">
              <span class="update-title">{t("settings.update_downloading", { percent: Math.round(updates.progress * 100) })}</span>
              <span class="update-bar"><span style:transform={`scaleX(${updates.progress})`}></span></span>
            </div>
          {:else if updates.phase === "ready"}
            <p class="hint update-hint">{t("settings.update_ready")}</p>
          {:else if updates.phase === "error"}
            <p class="hint update-hint error-hint">{t("settings.update_error")}: {updates.error}</p>
          {/if}
          <div class="about-row muted">{t("settings.about_license")}</div>

          <h3>{t("settings.privacy_title")}</h3>
          <p class="text">{t("settings.privacy_text")}</p>

          <button class="btn" onclick={() => updateSettings({ ...DEFAULT_SETTINGS })}>
            <Icon name="refresh" size={14} />
            {t("common.reset")}
          </button>
        </section>
      {/if}

      {#if store.saveError}
        <div class="error" role="alert">{t("settings.save_error")}: {store.saveError}</div>
      {/if}
    </div>
  </main>
</div>

<style>
  .settings {
    height: 100vh;
    display: grid;
    grid-template-columns: 232px 1fr;
    background: var(--bg);
    color: var(--fg);
    overflow: hidden;
  }

  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 12px;
    border-right: 1px solid var(--border);
    background: var(--bg-elevated);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 2px 6px;
  }

  .logo {
    width: 30px;
    height: 30px;
    border-radius: 9px;
    display: grid;
    place-items: center;
    background: var(--surface-strong);
    border: 1px solid var(--border);
    color: var(--accent);
    flex: none;
  }

  .brand-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .name {
    font-size: 13px;
    font-weight: 650;
  }

  .tagline {
    font-size: 9.5px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--muted);
    border-radius: var(--radius-control);
    padding: 8px 10px;
    min-height: 34px;
    font-size: 12.5px;
    text-align: left;
    transition: background 0.14s ease, color 0.14s ease;
  }

  .nav-item:hover {
    background: var(--surface);
    color: var(--fg);
  }

  .nav-item.active {
    background: var(--surface-strong);
    color: var(--fg);
  }

  .sidebar-footer {
    font-size: 9.5px;
    color: var(--muted);
    padding: 0 6px;
  }

  .content {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }

  .preview {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 22px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elevated);
    flex: none;
  }

  .preview-island {
    display: inline-flex;
    align-items: center;
    gap: 9px;
    height: 34px;
    padding: 0 16px;
    border-radius: 0 0 17px 17px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-top: none;
    font-size: 13px;
    font-weight: 550;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.25);
  }

  .preview-clock {
    font-variant-numeric: tabular-nums;
  }

  .preview-label {
    font-size: 11px;
    color: var(--muted);
  }

  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 24px 28px 32px;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-width: 680px;
  }

  h2 {
    margin: 0 0 16px;
    font-size: 17px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  h3 {
    margin: 20px 0 6px;
    font-size: 13px;
    font-weight: 600;
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 10px 12px;
    border-radius: var(--radius-control);
    font-size: var(--text-md);
    min-height: 44px;
  }

  .row:hover {
    background: var(--surface);
  }

  .row.static {
    justify-content: space-between;
  }

  .row-label {
    color: var(--fg);
  }

  .field {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .field input[type="number"] {
    width: 86px;
    text-align: right;
  }

  .wide {
    width: 300px;
    max-width: 58%;
    text-align: left;
    font-size: 11px;
  }

  .field input[type="range"] {
    width: 150px;
    accent-color: var(--accent);
    background: transparent;
    border: none;
    padding: 0;
  }

  .unit {
    font-size: 10.5px;
    color: var(--muted);
    min-width: 40px;
    text-align: right;
  }

  .color-field {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .color-field input[type="color"] {
    width: 34px;
    height: 26px;
    padding: 2px;
    border-radius: 7px;
    background: var(--surface);
    border: 1px solid var(--border);
    cursor: pointer;
  }

  .color-field code {
    font-size: 11px;
    color: var(--muted);
  }

  .switch {
    appearance: none;
    width: 34px;
    height: 20px;
    border-radius: 999px;
    background: var(--surface-strong);
    border: 1px solid var(--border);
    position: relative;
    cursor: pointer;
    transition: background 0.16s ease;
    flex: none;
    padding: 0;
  }

  .switch::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--fg);
    transition: transform 0.16s cubic-bezier(0.34, 1.3, 0.64, 1);
  }

  .switch:checked {
    background: var(--accent);
    border-color: transparent;
  }

  .switch:checked::after {
    transform: translateX(14px);
    background: #fff;
  }

  .switch:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .hint {
    font-size: 10.5px;
    line-height: 1.5;
    color: var(--muted);
    margin: 6px 4px 10px;
  }

  .text {
    font-size: 11.5px;
    line-height: 1.6;
    color: var(--muted);
    margin: 0 0 14px;
  }

  .probe-ok {
    color: var(--ok);
  }

  .probe-error {
    color: var(--danger);
    word-break: break-word;
  }

  .module-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
    margin-bottom: 8px;
  }

  .module {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
    font-size: 12px;
    cursor: pointer;
  }

  .module.disabled {
    opacity: 0.75;
  }

  .module-icon {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    border-radius: 8px;
    background: var(--surface-strong);
    color: var(--muted);
    flex: none;
  }

  .module-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .module-name {
    font-size: 11.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .widget-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 4px 0 10px;
    max-height: 260px;
    overflow-y: auto;
    padding-right: 2px;
  }

  .widget-row {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 5px 7px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
  }

  .widget-icon {
    width: 24px;
    height: 24px;
    display: grid;
    place-items: center;
    border-radius: 7px;
    background: var(--surface-strong);
    color: var(--accent);
    flex: none;
  }

  .widget-name {
    flex: 1;
    min-width: 0;
    font-size: 11.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .widget-grip {
    display: grid;
    place-items: center;
    color: var(--muted);
    cursor: grab;
    flex: none;
    opacity: 0.75;
  }

  .widget-row.dragging {
    opacity: 0.45;
  }

  .widget-row.drop-target {
    border-color: color-mix(in srgb, var(--accent) 65%, var(--border));
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent) 35%, transparent);
  }

  .widget-size-select {
    width: 74px;
    padding: 2px 4px;
    font-size: 9.5px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--surface-strong);
    color: var(--fg);
    flex: none;
  }

  .widget-row .btn {
    padding: 3px;
    min-width: 22px;
    min-height: 22px;
    border-color: transparent;
    background: transparent;
  }

  .about-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    font-size: 12px;
    padding: 3px 0;
  }

  .update-btn {
    padding: 4px 9px;
    font-size: 10.5px;
  }

  .update-card {
    display: flex;
    flex-direction: column;
    gap: 7px;
    align-items: flex-start;
    margin-top: 10px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
  }

  .update-title {
    font-size: 12px;
    font-weight: 600;
  }

  .update-notes {
    margin: 0;
    font-size: 10.5px;
    color: var(--muted);
    white-space: pre-line;
    max-height: 96px;
    overflow-y: auto;
  }

  .update-hint {
    margin-top: 6px;
  }

  .error-hint {
    color: var(--danger);
  }

  .update-bar {
    display: block;
    width: 100%;
    height: 4px;
    border-radius: 999px;
    background: var(--surface-strong);
    overflow: hidden;
  }

  .update-bar span {
    display: block;
    width: 100%;
    height: 100%;
    transform-origin: left center;
    background: var(--accent);
    transition: transform 0.15s linear;
  }

  .error {
    margin-top: 14px;
    font-size: 11px;
    color: var(--danger);
  }
</style>
