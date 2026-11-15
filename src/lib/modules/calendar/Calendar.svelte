<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import Icon from "../../components/Icon.svelte";
  import { locale, t } from "../../store.svelte";

  interface EventItem {
    id: number;
    title: string;
    start: number;
    end: number;
    all_day: boolean;
    notes: string;
  }

  interface Reminder {
    id: number;
    title: string;
    due: number;
    done: boolean;
    notified: boolean;
    created: number;
  }

  type Mode = "calendar" | "reminders";

  let mode = $state<Mode>("calendar");
  let view = $state(new Date());
  let selected = $state(startOfDay(new Date()));
  let events = $state<EventItem[]>([]);
  let reminders = $state<Reminder[]>([]);
  let eventTitle = $state("");
  let eventTime = $state(defaultTime());
  let reminderTitle = $state("");
  let reminderTime = $state(defaultReminderTime());
  let error = $state("");
  let notice = $state("");

  function startOfDay(date: Date): Date {
    const copy = new Date(date);
    copy.setHours(0, 0, 0, 0);
    return copy;
  }

  function addDays(date: Date, days: number): Date {
    const copy = new Date(date);
    copy.setDate(copy.getDate() + days);
    return copy;
  }

  function defaultTime(): string {
    const now = new Date();
    now.setMinutes(0, 0, 0);
    now.setHours(now.getHours() + 1);
    return `${now.getHours().toString().padStart(2, "0")}:00`;
  }

  function defaultReminderTime(): string {
    const now = new Date(Date.now() + 30 * 60_000);
    return `${now.getFullYear()}-${(now.getMonth() + 1).toString().padStart(2, "0")}-${now
      .getDate()
      .toString()
      .padStart(2, "0")}T${now.getHours().toString().padStart(2, "0")}:${now
      .getMinutes()
      .toString()
      .padStart(2, "0")}`;
  }

  function sameDay(a: Date, b: Date): boolean {
    return (
      a.getFullYear() === b.getFullYear() &&
      a.getMonth() === b.getMonth() &&
      a.getDate() === b.getDate()
    );
  }

  function monthStart(date: Date): Date {
    return new Date(date.getFullYear(), date.getMonth(), 1);
  }

  function monthEnd(date: Date): Date {
    return new Date(date.getFullYear(), date.getMonth() + 1, 0, 23, 59, 59, 999);
  }

  const cells = $derived.by(() => {
    const first = monthStart(view);
    const weekday = (first.getDay() + 6) % 7; // Monday-first grid
    const start = addDays(first, -weekday);
    return Array.from({ length: 42 }, (_, index) => addDays(start, index));
  });

  const eventsByDay = $derived.by(() => {
    const map = new Map<string, EventItem[]>();
    for (const event of events) {
      const key = new Date(event.start).toDateString();
      const list = map.get(key) ?? [];
      list.push(event);
      map.set(key, list);
    }
    return map;
  });

  const dayEvents = $derived(
    events
      .filter((event) => {
        const begin = startOfDay(selected).getTime();
        const end = addDays(startOfDay(selected), 1).getTime();
        return event.start < end && event.end > begin;
      })
      .sort((a, b) => a.start - b.start),
  );

  const pending = $derived(reminders.filter((reminder) => !reminder.done));
  const finished = $derived(reminders.filter((reminder) => reminder.done));

  function formatTime(timestamp: number): string {
    return new Date(timestamp).toLocaleTimeString(locale(), {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    });
  }

  function formatReminder(timestamp: number): string {
    const date = new Date(timestamp);
    return date.toLocaleString(locale(), {
      day: "numeric",
      month: "short",
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    });
  }

  function monthLabel(): string {
    return view.toLocaleDateString(locale(), { month: "long", year: "numeric" });
  }

  function shiftMonth(delta: number): void {
    view = new Date(view.getFullYear(), view.getMonth() + delta, 1);
    void loadEvents();
  }

  async function loadEvents(): Promise<void> {
    try {
      const from = addDays(monthStart(view), -7).getTime();
      const to = addDays(monthEnd(view), 7).getTime();
      events = await invoke<EventItem[]>("events_list", { from, to });
    } catch (cause) {
      error = String(cause);
    }
  }

  async function loadReminders(): Promise<void> {
    try {
      reminders = await invoke<Reminder[]>("reminders_list");
    } catch (cause) {
      error = String(cause);
    }
  }

  async function addEvent(): Promise<void> {
    const title = eventTitle.trim();
    if (!title) return;
    const [hours, minutes] = eventTime.split(":").map(Number);
    const start = new Date(selected);
    start.setHours(hours || 0, minutes || 0, 0, 0);
    try {
      await invoke("events_save", {
        event: {
          id: null,
          title,
          start: start.getTime(),
          end: start.getTime() + 3600_000,
          all_day: false,
          notes: "",
        },
      });
      eventTitle = "";
      await loadEvents();
    } catch (cause) {
      error = String(cause);
    }
  }

  async function removeEvent(id: number): Promise<void> {
    try {
      await invoke("events_delete", { id });
      await loadEvents();
    } catch (cause) {
      error = String(cause);
    }
  }

  async function addReminder(): Promise<void> {
    const title = reminderTitle.trim();
    if (!title) return;
    const due = new Date(reminderTime).getTime();
    if (!Number.isFinite(due)) {
      error = t("reminders.invalid_time");
      return;
    }
    try {
      await invoke("reminders_save", { reminder: { id: null, title, due } });
      reminderTitle = "";
      reminderTime = defaultReminderTime();
      await loadReminders();
    } catch (cause) {
      error = String(cause);
    }
  }

  async function toggleReminder(reminder: Reminder): Promise<void> {
    try {
      await invoke("reminders_toggle", { id: reminder.id, done: !reminder.done });
      await loadReminders();
    } catch (cause) {
      error = String(cause);
    }
  }

  async function removeReminder(id: number): Promise<void> {
    try {
      await invoke("reminders_delete", { id });
      await loadReminders();
    } catch (cause) {
      error = String(cause);
    }
  }

  async function importIcs(): Promise<void> {
    try {
      const path = await open({
        multiple: false,
        filters: [{ name: "iCalendar", extensions: ["ics"] }],
      });
      if (!path) return;
      const count = await invoke<number>("calendar_import_ics", { path });
      notice = t("calendar.imported", { count });
      await loadEvents();
      setTimeout(() => (notice = ""), 2500);
    } catch (cause) {
      error = String(cause);
    }
  }

  async function exportIcs(): Promise<void> {
    try {
      const path = await save({
        defaultPath: "isle-calendar.ics",
        filters: [{ name: "iCalendar", extensions: ["ics"] }],
      });
      if (!path) return;
      const saved = await invoke<string>("calendar_export_ics", { path });
      notice = saved;
      setTimeout(() => (notice = ""), 3000);
    } catch (cause) {
      error = String(cause);
    }
  }

  onMount(() => {
    void loadEvents();
    void loadReminders();
    const off = listen("reminder-due", () => {
      void loadReminders();
    });
    return () => {
      void off.then((unlisten) => unlisten());
    };
  });
</script>

<div class="calendar">
  <div class="topbar">
    <button class:active={mode === "calendar"} onclick={() => (mode = "calendar")}>
      <Icon name="calendar" size={12} />{t("calendar.tab")}
    </button>
    <button class:active={mode === "reminders"} onclick={() => (mode = "reminders")}>
      <Icon name="bell" size={12} />{t("reminders.tab")}
      {#if pending.length}<span class="badge">{pending.length}</span>{/if}
    </button>
    <span class="spacer"></span>
    {#if mode === "calendar"}
      <button class="mini" onclick={importIcs} title={t("calendar.import")}>
        <Icon name="download" size={12} />
      </button>
      <button class="mini" onclick={exportIcs} title={t("calendar.export")}>
        <Icon name="external" size={12} />
      </button>
    {/if}
  </div>

  {#if mode === "calendar"}
    <div class="month-head">
      <button class="nav" aria-label={t("calendar.prev")} onclick={() => shiftMonth(-1)}>
        <Icon name="chevron-left" size={13} />
      </button>
      <span class="month">{monthLabel()}</span>
      <button class="nav" aria-label={t("calendar.next")} onclick={() => shiftMonth(1)}>
        <Icon name="chevron-right" size={13} />
      </button>
      <button class="today" onclick={() => { selected = startOfDay(new Date()); view = new Date(); void loadEvents(); }}>
        {t("calendar.today")}
      </button>
    </div>

    <div class="weekdays">
      {#each ["mon", "tue", "wed", "thu", "fri", "sat", "sun"] as day}
        <span>{t(`calendar.wd_${day}`)}</span>
      {/each}
    </div>

    <div class="grid">
      {#each cells as cell, index (index)}
        <button
          class="cell"
          class:outside={cell.getMonth() !== view.getMonth()}
          class:selected={sameDay(cell, selected)}
          class:today={sameDay(cell, new Date())}
          onclick={() => (selected = startOfDay(cell))}
        >
          <span>{cell.getDate()}</span>
          {#if eventsByDay.get(cell.toDateString())?.length}
            <span class="dot"></span>
          {/if}
        </button>
      {/each}
    </div>

    <div class="events">
      <div class="events-head">
        <span>
          {selected.toLocaleDateString(locale(), { day: "numeric", month: "long", weekday: "short" })}
        </span>
        <span class="muted">{t("calendar.events_count", { count: dayEvents.length })}</span>
      </div>
      <div class="event-list">
        {#each dayEvents as event (event.id)}
          <div class="event">
            <span class="time">{event.all_day ? t("calendar.all_day") : formatTime(event.start)}</span>
            <span class="title" title={event.title}>{event.title}</span>
            <button class="trash" aria-label={t("common.delete")} onclick={() => removeEvent(event.id)}>
              <Icon name="trash" size={11} />
            </button>
          </div>
        {:else}
          <span class="muted pad">{t("calendar.no_events")}</span>
        {/each}
      </div>
      <form class="add" onsubmit={(event) => { event.preventDefault(); void addEvent(); }}>
        <input type="time" bind:value={eventTime} aria-label={t("calendar.time")} />
        <input class="grow" bind:value={eventTitle} placeholder={t("calendar.placeholder")} aria-label={t("calendar.add")} />
        <button class="add-btn" type="submit" disabled={!eventTitle.trim()} aria-label={t("common.add")}>
          <Icon name="plus" size={13} />
        </button>
      </form>
    </div>
  {:else}
    <form class="reminder-add" onsubmit={(event) => { event.preventDefault(); void addReminder(); }}>
      <input class="grow" bind:value={reminderTitle} placeholder={t("reminders.placeholder")} aria-label={t("reminders.add")} />
      <input type="datetime-local" bind:value={reminderTime} aria-label={t("reminders.time")} />
      <button class="add-btn" type="submit" disabled={!reminderTitle.trim()} aria-label={t("common.add")}>
        <Icon name="plus" size={13} />
      </button>
    </form>

    <div class="reminder-list">
      {#each pending as reminder (reminder.id)}
        <div class="reminder" class:due={reminder.due <= Date.now()}>
          <button class="check" aria-label={t("reminders.done")} onclick={() => toggleReminder(reminder)}>
            <span></span>
          </button>
          <span class="title" title={reminder.title}>{reminder.title}</span>
          <span class="due-time">{formatReminder(reminder.due)}</span>
          <button class="trash" aria-label={t("common.delete")} onclick={() => removeReminder(reminder.id)}>
            <Icon name="trash" size={11} />
          </button>
        </div>
      {:else}
        <div class="empty">
          <Icon name="bell" size={20} />
          <span>{t("reminders.empty")}</span>
        </div>
      {/each}

      {#if finished.length}
        <div class="finished-label">{t("reminders.finished")}</div>
        {#each finished.slice(0, 6) as reminder (reminder.id)}
          <div class="reminder done">
            <button class="check checked" aria-label={t("reminders.restore")} onclick={() => toggleReminder(reminder)}>
              <Icon name="check" size={9} />
            </button>
            <span class="title">{reminder.title}</span>
            <button class="trash" aria-label={t("common.delete")} onclick={() => removeReminder(reminder.id)}>
              <Icon name="trash" size={11} />
            </button>
          </div>
        {/each}
      {/if}
    </div>
  {/if}

  {#if notice}<div class="notice" role="status">{notice}</div>{/if}
  {#if error}<div class="error" role="alert">{error}</div>{/if}
</div>

<style>
  .calendar {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-height: 0;
  }

  .topbar {
    display: flex;
    align-items: center;
    gap: 3px;
    flex: none;
  }

  .topbar > button {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--muted);
    border-radius: 6px;
    padding: 3px 7px;
    font-size: 10.5px;
  }

  .topbar > button.active {
    background: var(--surface-strong);
    color: var(--fg);
    border-color: var(--border);
  }

  .badge {
    background: var(--accent);
    color: #fff;
    border-radius: 999px;
    font-size: 8px;
    padding: 0 4px;
  }

  .spacer {
    flex: 1;
  }

  .mini {
    width: 22px;
    height: 20px;
    display: grid;
    place-items: center;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--muted);
    border-radius: 5px;
    padding: 0;
  }

  .mini:hover {
    color: var(--fg);
  }

  .month-head {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: none;
  }

  .nav {
    width: 20px;
    height: 18px;
    display: grid;
    place-items: center;
    border: none;
    background: transparent;
    color: var(--muted);
    padding: 0;
  }

  .month {
    font-size: 11px;
    font-weight: 600;
    text-transform: capitalize;
    flex: 1;
  }

  .today {
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--muted);
    border-radius: 5px;
    font-size: 9px;
    padding: 2px 6px;
  }

  .weekdays {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 1px;
    text-align: center;
    font-size: 8px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted);
    flex: none;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    grid-template-rows: repeat(6, 1fr);
    gap: 1px;
    flex: 1;
    min-height: 92px;
  }

  .cell {
    position: relative;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: var(--fg);
    font-size: 9.5px;
    padding: 0;
    display: grid;
    place-items: center;
    min-height: 14px;
  }

  .cell:hover {
    background: var(--surface);
  }

  .cell.outside {
    color: var(--muted);
    opacity: 0.45;
  }

  .cell.today span:first-child {
    color: var(--accent);
    font-weight: 700;
  }

  .cell.selected {
    background: var(--surface-strong);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .dot {
    position: absolute;
    bottom: 1px;
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: var(--accent);
  }

  .events {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .events-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 9.5px;
    color: var(--muted);
    text-transform: capitalize;
    padding: 0 2px;
  }

  .event-list {
    max-height: 48px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1px;
    scrollbar-width: none;
  }

  .event {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 10px;
    padding: 1px 2px;
  }

  .event .time {
    color: var(--accent);
    font-variant-numeric: tabular-nums;
    min-width: 42px;
  }

  .event .title {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .trash {
    border: none;
    background: transparent;
    color: var(--muted);
    padding: 1px;
    display: grid;
    place-items: center;
    opacity: 0.6;
  }

  .trash:hover {
    color: var(--danger);
    opacity: 1;
  }

  .add,
  .reminder-add {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: none;
  }

  .add input,
  .reminder-add input {
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--fg);
    border-radius: 6px;
    padding: 3px 6px;
    font-size: 10px;
    font-family: inherit;
    min-width: 0;
  }

  .add input[type="time"],
  .reminder-add input[type="datetime-local"] {
    width: 76px;
    color-scheme: dark;
  }

  .reminder-add input[type="datetime-local"] {
    width: 138px;
  }

  .grow {
    flex: 1;
  }

  .add-btn {
    width: 24px;
    height: 22px;
    border: none;
    border-radius: 6px;
    background: var(--accent);
    color: #fff;
    display: grid;
    place-items: center;
    padding: 0;
  }

  .add-btn:disabled {
    opacity: 0.4;
  }

  .reminder-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
    scrollbar-width: thin;
  }

  .reminder {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 10.5px;
    padding: 2px 3px;
    border-radius: 5px;
  }

  .reminder:hover {
    background: var(--surface);
  }

  .reminder.due .due-time {
    color: var(--accent);
  }

  .reminder.done {
    opacity: 0.5;
  }

  .reminder .title {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .due-time {
    font-size: 9px;
    color: var(--muted);
    white-space: nowrap;
  }

  .check {
    width: 14px;
    height: 14px;
    border-radius: 4px;
    border: 1px solid var(--border);
    background: transparent;
    padding: 0;
    display: grid;
    place-items: center;
    color: #fff;
    flex: none;
  }

  .check.checked {
    background: var(--accent);
    border-color: transparent;
  }

  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    flex: 1;
    color: var(--muted);
    font-size: 10.5px;
    padding: 14px;
  }

  .finished-label {
    font-size: 8.5px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
    padding: 4px 3px 1px;
  }

  .muted {
    color: var(--muted);
    font-size: 9.5px;
  }

  .pad {
    padding: 8px 2px;
  }

  .notice {
    font-size: 9px;
    color: var(--accent);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: none;
  }

  .error {
    font-size: 9px;
    color: var(--danger);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: none;
  }
</style>
