<script lang="ts">
  import Icon from "../../components/Icon.svelte";
  import { locale, t } from "../../store.svelte";
  import { writeClipboard } from "../../ipc";
  import { invoke } from "@tauri-apps/api/core";
  import {
    collapseWhitespace,
    countWords,
    decodeBase64,
    dedupeLines,
    encodeBase64,
    formatJson,
    generatePassword,
    generateUuid,
    removeEmptyLines,
    reverseTransliterate,
    sortLines,
    switchLayout,
    toLower,
    toSentence,
    toTitle,
    toUpper,
    transliterate,
    type LayoutDirection,
  } from "../../text-tools";

  let input = $state("");
  let output = $state("");
  let error = $state<string | null>(null);
  let copied = $state(false);
  let from = $state("auto");
  let to = $state<string>(locale() === "en" ? "en" : locale());
  let translating = $state(false);
  let detected = $state(false);
  let detectedSource = $state("");
  let note = $state("");

  const LANGUAGES = [
    "auto",
    "uk",
    "en",
    "ru",
    "pl",
    "de",
    "fr",
    "es",
    "it",
    "pt",
    "tr",
    "zh",
    "ja",
  ];

  const LANGUAGE_LABELS: Record<string, string> = {
    auto: "Auto",
    uk: "Українська",
    en: "English",
    ru: "Русский",
    pl: "Polski",
    de: "Deutsch",
    fr: "Français",
    es: "Español",
    it: "Italiano",
    pt: "Português",
    tr: "Türkçe",
    zh: "中文",
    ja: "日本語",
  };

  function detectLocal(text: string): string {
    const cyrillic = (text.match(/[\u0400-\u04FF]/g) ?? []).length;
    const latin = (text.match(/[A-Za-z]/g) ?? []).length;
    if (cyrillic > latin) {
      if (/[іїєґ]/i.test(text)) return "uk";
      if (/[ыэъ]/i.test(text)) return "ru";
      return "uk";
    }
    return "en";
  }

  const chars = $derived(input.length);
  const words = $derived(countWords(input));
  const lines = $derived(input ? input.split(/\r?\n/).length : 0);

  function run(fn: (text: string) => string): void {
    error = null;
    try {
      output = fn(input);
    } catch (err) {
      error = err instanceof Error && err.message ? err.message : t("text.error");
    }
  }

  function layout(direction: LayoutDirection): void {
    run((text) => switchLayout(text, direction));
  }

  function copy(): void {
    const value = output || input;
    if (!value) return;
    void writeClipboard(value).then(() => {
      copied = true;
      setTimeout(() => (copied = false), 1400);
    });
  }

  function replace(): void {
    if (!output) return;
    input = output;
    output = "";
  }

  async function translate(): Promise<void> {
    if (translating) return;
    if (!input.trim()) {
      error = t("text.translate_empty");
      return;
    }
    error = null;
    note = "";
    translating = true;
    try {
      let target = to;
      const guessed = detectLocal(input);
      if (from === "auto" && guessed === target) {
        target = target === "en" ? "uk" : "en";
        to = target;
        note = t("text.target_auto_switched", {
          language: LANGUAGE_LABELS[target] ?? target.toUpperCase(),
        });
      }
      const result = await invoke<{
        text: string;
        from: string;
        to: string;
        detected: boolean;
      }>("translate_text", { text: input, from, to: target });
      output = result.text;
      detected = result.detected;
      detectedSource = result.from;
      if (result.detected) from = result.from;
      if (result.text.trim() === input.trim() && result.from === result.to) {
        note = t("text.same_language");
      }
    } catch (cause) {
      error = String(cause);
    } finally {
      translating = false;
    }
  }

  function swapLanguages(): void {
    note = "";
    if (from === "auto") {
      from = to;
      to = to === "en" ? "uk" : "en";
      return;
    }
    const previous = from;
    from = to;
    to = previous;
  }

  const toolGroups: Array<{
    label: string;
    tools: Array<{ label: string; action: () => void; icon?: string }>;
  }> = [
    {
      label: "text.layout",
      tools: [
        { label: "text.layout_auto", action: () => layout("auto") },
        { label: "text.layout_en_uk", action: () => layout("en-uk") },
        { label: "text.layout_uk_en", action: () => layout("uk-en") },
        { label: "text.layout_en_ru", action: () => layout("en-ru") },
        { label: "text.layout_ru_en", action: () => layout("ru-en") },
      ],
    },
    {
      label: "text.tools",
      tools: [
        { label: "text.upper", action: () => run(toUpper) },
        { label: "text.lower", action: () => run(toLower) },
        { label: "text.title", action: () => run(toTitle) },
        { label: "text.sentence", action: () => run(toSentence) },
        { label: "text.translit", action: () => run(transliterate) },
        { label: "text.translit_reverse", action: () => run(reverseTransliterate) },
        { label: "text.whitespace", action: () => run(collapseWhitespace) },
        { label: "text.empty_lines", action: () => run(removeEmptyLines) },
      ],
    },
    {
      label: "text.result",
      tools: [
        { label: "text.json_format", action: () => run((text) => formatJson(text, false)) },
        { label: "text.json_minify", action: () => run((text) => formatJson(text, true)) },
        { label: "text.base64_encode", action: () => run(encodeBase64) },
        { label: "text.base64_decode", action: () => run(decodeBase64) },
        { label: "text.url_encode", action: () => run(encodeURIComponent) },
        { label: "text.url_decode", action: () => run(decodeURIComponent) },
        { label: "text.sort_lines", action: () => run(sortLines) },
        { label: "text.dedupe", action: () => run(dedupeLines) },
        { label: "text.password", action: () => run(() => generatePassword(20)) },
        { label: "text.uuid", action: () => run(() => generateUuid()) },
      ],
    },
  ];
</script>

<div class="text-tools">
  <div class="translator" class:busy={translating}>
    <span class="translate-label">{t("text.translate")}</span>
    <select
      value={from}
      aria-label={t("text.translate_from")}
      onchange={(event) => {
        from = (event.currentTarget as HTMLSelectElement).value;
        note = "";
      }}
    >
      {#each LANGUAGES as code (code)}
        <option value={code}>
          {code === "auto" ? t("text.auto") : LANGUAGE_LABELS[code] ?? code.toUpperCase()}
        </option>
      {/each}
    </select>
    <button class="swap" onclick={swapLanguages} title={t("text.swap")} aria-label={t("text.swap")}>
      <Icon name="refresh" size={12} />
    </button>
    <span class="arrow">→</span>
    <select
      value={to}
      aria-label={t("text.translate_to")}
      onchange={(event) => {
        to = (event.currentTarget as HTMLSelectElement).value;
        note = "";
      }}
    >
      {#each LANGUAGES.filter((code) => code !== "auto") as code (code)}
        <option value={code}>{LANGUAGE_LABELS[code] ?? code.toUpperCase()}</option>
      {/each}
    </select>
    <button class="btn translate" onclick={translate} disabled={translating || !input}>
      <Icon name={translating ? "refresh" : "globe"} size={13} />
      {translating ? t("text.translating") : t("text.translate")}
    </button>
    {#if detected && detectedSource}
      <span class="detected-source">{LANGUAGE_LABELS[detectedSource] ?? detectedSource.toUpperCase()}</span>
    {/if}
    {#if translating}
      <span class="progress"><span></span></span>
    {/if}
  </div>

  <div class="workspace">
    <div class="editor">
      <textarea
        bind:value={input}
        placeholder={t("text.placeholder")}
        aria-label={t("text.placeholder")}
        spellcheck="false"
      ></textarea>
      <div class="counters">
        <span>{t("text.chars", { count: chars })}</span>
        <span>{t("text.words", { count: words })}</span>
        <span>{t("text.lines", { count: lines })}</span>
      </div>
    </div>

    <aside class="panel">
      <div class="groups">
      {#each toolGroups as group}
        <div class="group">
          <div class="group-label">{t(group.label)}</div>
          <div class="buttons">
            {#each group.tools as tool}
              <button class="tool" onclick={tool.action} disabled={!input}>
                {t(tool.label)}
              </button>
            {/each}
          </div>
        </div>
      {/each}
    </div>

    {#if output}
      <div class="result">
        <div class="result-head">
          <span>
            {t("text.result")}
            {#if detected}
              <span class="detected">
                {detectedSource ? LANGUAGE_LABELS[detectedSource] ?? detectedSource.toUpperCase() : t("text.auto")}
              </span>
            {/if}
          </span>
          <div class="result-actions">
            <button class="btn ghost" onclick={replace}>{t("common.save")}</button>
            <button class="btn ghost" onclick={copy}>
              <Icon name={copied ? "check" : "copy"} size={13} />
              {copied ? t("common.copied") : t("common.copy")}
            </button>
          </div>
        </div>
        <pre class="mono">{output}</pre>
      </div>
    {/if}

    {#if note}
      <div class="note">{note}</div>
    {/if}

    {#if error}
      <div class="error" role="alert">{error}</div>
    {/if}
  </aside>
  </div>
</div>

<style>
  .text-tools {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
  }

  .workspace {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .editor {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-height: 0;
  }

  textarea {
    flex: 1;
    min-height: 0;
    resize: none;
    font-size: 12px;
    line-height: 1.5;
    padding: 10px;
    border-radius: 11px;
    background: var(--input);
    border: 1px solid var(--border);
  }

  .counters {
    display: flex;
    gap: 12px;
    font-size: 10px;
    color: var(--muted);
    flex: none;
    padding: 0 2px;
  }

  .panel {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
  }

  .groups {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 7px;
    overflow-y: auto;
    max-height: 40%;
    padding-right: 2px;
  }

  .translator {
    flex: none;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 5px 6px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
    position: relative;
    overflow: hidden;
  }

  .translator.busy {
    border-color: color-mix(in srgb, var(--accent) 40%, var(--border));
  }

  .translate-label {
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
    padding: 0 4px;
    white-space: nowrap;
  }

  .translator select {
    min-width: 0;
    flex: 1;
    height: 25px;
    font-size: 10.5px;
    padding: 0 5px;
    border-radius: 6px;
    background: var(--input);
    border: 1px solid var(--border);
    color: var(--fg);
  }

  .arrow {
    color: var(--muted);
    font-size: 12px;
    flex: none;
  }

  .detected-source {
    font-size: 8px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--muted);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 1px 5px;
    white-space: nowrap;
  }

  .progress {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 2px;
    background: var(--surface-strong);
    overflow: hidden;
  }

  .progress span {
    display: block;
    width: 38%;
    height: 100%;
    border-radius: 999px;
    background: var(--accent);
    animation: translate-progress 1s ease-in-out infinite;
  }

  @keyframes translate-progress {
    from { transform: translateX(-110%); }
    to { transform: translateX(300%); }
  }

  .translator.busy .translate :global(svg) {
    animation: translate-spin 0.9s linear infinite;
  }

  @keyframes translate-spin {
    to { transform: rotate(360deg); }
  }

  .swap {
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
    padding: 0;
    flex: none;
  }

  .swap:hover {
    color: var(--fg);
    background: var(--surface-strong);
  }

  .translate {
    flex: none;
    padding: 0 8px;
    min-height: 24px;
    font-size: 10px;
    white-space: nowrap;
  }

  .translate:disabled {
    opacity: 0.5;
  }

  .detected {
    margin-left: 5px;
    font-size: 8px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0 4px;
  }

  .group-label {
    font-size: 9.5px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
    padding: 0 2px 4px;
  }

  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .tool {
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--fg);
    border-radius: 7px;
    padding: 4px 8px;
    font-size: 10.5px;
    min-height: 24px;
    transition: background 0.14s ease, transform 0.1s ease;
  }

  .tool:hover:not(:disabled) {
    background: var(--surface-strong);
  }

  .tool:active:not(:disabled) {
    transform: scale(0.96);
  }

  .tool:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .result {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: 11px;
    background: var(--surface);
    overflow: hidden;
  }

  .result-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 5px 8px;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
    border-bottom: 1px solid var(--border);
  }

  .result-actions {
    display: flex;
    gap: 2px;
    text-transform: none;
    letter-spacing: 0;
  }

  .result-actions .btn {
    padding: 3px 6px;
    font-size: 10.5px;
    min-height: 22px;
  }

  pre {
    margin: 0;
    padding: 8px 10px;
    font-size: 11px;
    line-height: 1.45;
    overflow: auto;
    flex: 1;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .error {
    font-size: 11px;
    color: var(--danger);
    padding: 0 2px;
  }

  .note {
    font-size: 10px;
    color: var(--accent);
    padding: 0 2px;
  }
</style>
