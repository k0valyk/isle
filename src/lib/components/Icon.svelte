<script lang="ts">
  interface Props {
    name: string;
    size?: number;
    stroke?: number;
  }

  let { name, size = 18, stroke = 1.7 }: Props = $props();

  const ICONS: Record<string, string> = {
    home: '<path d="M3 10.6 12 3l9 7.6"/><path d="M5.2 9.3V21h13.6V9.3"/><path d="M9.6 21v-6.2h4.8V21"/>',
    music: '<path d="M9 18V5.5l11-2v12.3"/><circle cx="6.4" cy="18" r="2.6"/><circle cx="17.4" cy="15.8" r="2.6"/>',
    clipboard: '<rect x="5" y="4" width="14" height="17" rx="2.5"/><path d="M9 4.5V3.2h6v1.3"/><path d="M9 10h6M9 14h6M9 18h4"/>',
    type: '<path d="M4 6.5V4h16v2.5"/><path d="M12 4v16"/><path d="M8.5 20h7"/>',
    notes: '<rect x="4.5" y="3.5" width="15" height="17" rx="2.5"/><path d="M8 8h8M8 12h8M8 16h5"/>',
    timer: '<circle cx="12" cy="13.5" r="7.5"/><path d="M12 9.5v4.2l2.6 1.6"/><path d="M9 2.5h6"/><path d="M12 2.5v1.8"/>',
    cloud: '<path d="M7.2 18.5h10.1a3.7 3.7 0 0 0 .4-7.4A5.6 5.6 0 0 0 7 9.9a4.3 4.3 0 0 0 .2 8.6Z"/>',
    camera: '<rect x="3" y="6.5" width="18" height="13" rx="2.5"/><path d="M8.5 6.5 10 4h4l1.5 2.5"/><circle cx="12" cy="13" r="3.4"/>',
    sparkles: '<path d="M12 3.5 13.6 8l4.4 1.6L13.6 11 12 15.5 10.4 11 6 9.6 10.4 8Z"/><path d="M18.5 15.5l.8 2.2 2.2.8-2.2.8-.8 2.2-.8-2.2-2.2-.8 2.2-.8Z"/><path d="M5 16l.6 1.6 1.6.6-1.6.6L5 20.4l-.6-1.6L2.8 18.2l1.6-.6Z"/>',
    message: '<path d="M20.5 11.6a8.2 8.2 0 0 1-8.5 8.2 8.7 8.7 0 0 1-3.5-.7L3.5 20.5l1.4-4.7a8.2 8.2 0 1 1 15.6-4.2Z"/><path d="M8.5 11.5h7M8.5 14.5h4.5"/>',
    calendar: '<rect x="3.5" y="5" width="17" height="15.5" rx="2.5"/><path d="M3.5 10h17"/><path d="M7.5 3v4M16.5 3v4"/>',
    sliders: '<path d="M4 7h7M15 7h5M4 12h3M11 12h9M4 17h9M17 17h3"/><circle cx="13" cy="7" r="2"/><circle cx="9" cy="12" r="2"/><circle cx="15" cy="17" r="2"/>',
    phone: '<rect x="7" y="2.5" width="10" height="19" rx="2.6"/><path d="M10.5 5h3"/><path d="M11.5 18.5h1"/>',
    terminal: '<rect x="3" y="4.5" width="18" height="15" rx="2.5"/><path d="M7 9.5 10 12l-3 2.5"/><path d="M12.5 15h4"/>',
    settings: '<circle cx="12" cy="12" r="3.1"/><path d="M12 2.4v2.4M12 19.2v2.4M2.4 12h2.4M19.2 12h2.4M5.2 5.2l1.7 1.7M17.1 17.1l1.7 1.7M18.8 5.2l-1.7 1.7M6.9 17.1l-1.7 1.7"/>',
    x: '<path d="M6 6l12 12M18 6 6 18"/>',
    plus: '<path d="M12 5v14M5 12h14"/>',
    trash: '<path d="M4 7h16"/><path d="M9.5 7V4.5h5V7"/><path d="M6.5 7l1 13h9l1-13"/><path d="M10.5 11v5M13.5 11v5"/>',
    copy: '<rect x="9" y="9" width="11.5" height="11.5" rx="2.2"/><path d="M15 9V6.5A2.5 2.5 0 0 0 12.5 4H6.5A2.5 2.5 0 0 0 4 6.5v6A2.5 2.5 0 0 0 6.5 15H9"/>',
    check: '<path d="M5 12.5 10 17.5 19.5 6.5"/>',
    search: '<circle cx="11" cy="11" r="7"/><path d="M16.5 16.5 21 21"/>',
    refresh: '<path d="M20 11.5A8 8 0 0 0 6.3 6.3L4 8.5"/><path d="M4 4v4.5h4.5"/><path d="M4 12.5A8 8 0 0 0 17.7 17.7L20 15.5"/><path d="M20 20v-4.5h-4.5"/>',
    play: '<path d="M8 5.2v13.6L19 12Z" fill="currentColor" stroke="none"/>',
    pause: '<rect x="7" y="5" width="3.6" height="14" rx="1.2" fill="currentColor" stroke="none"/><rect x="13.4" y="5" width="3.6" height="14" rx="1.2" fill="currentColor" stroke="none"/>',
    'skip-back': '<path d="M18.5 5.5v13L8.5 12Z"/><path d="M5.5 5.5v13"/>',
    'skip-forward': '<path d="M5.5 5.5v13L15.5 12Z"/><path d="M18.5 5.5v13"/>',
    rewind: '<path d="M3 12a9 9 0 1 0 3-6.7L3 8"/><path d="M3 3v5h5"/>',
    forward: '<path d="M21 12a9 9 0 1 1-3-6.7L21 8"/><path d="M21 3v5h-5"/>',
    'chevron-down': '<path d="m6 9 6 6 6-6"/>',
    volume: '<path d="M4 9.5h3.2L12 5.5v13L7.2 14.5H4Z"/><path d="M15.5 9.2a4 4 0 0 1 0 5.6"/><path d="M18 6.8a7.2 7.2 0 0 1 0 10.4"/>',
    'volume-off': '<path d="M4 9.5h3.2L12 5.5v13L7.2 14.5H4Z"/><path d="m16 9.5 5 5M21 9.5l-5 5"/>',
    'chevron-left': '<path d="m14.5 6-6 6 6 6"/>',
    'chevron-right': '<path d="m9.5 6 6 6-6 6"/>',
    pin: '<path d="M9 3.5h6l-.8 5.6 3.3 3.4H6.5l3.3-3.4Z"/><path d="M12 12.5V21"/>',
    sun: '<circle cx="12" cy="12" r="4.2"/><path d="M12 2.5v2.2M12 19.3v2.2M2.5 12h2.2M19.3 12h2.2M5.2 5.2l1.6 1.6M17.2 17.2l1.6 1.6M18.8 5.2l-1.6 1.6M6.8 17.2l-1.6 1.6"/>',
    moon: '<path d="M20 14.5A8 8 0 0 1 9.5 4 8 8 0 1 0 20 14.5Z"/>',
    'cloud-sun': '<circle cx="8" cy="8" r="3"/><path d="M8 2.5v1.5M2.5 8H4M3.9 3.9l1.1 1.1M12.1 3.9 11 5"/><path d="M9 19h9.4a3.4 3.4 0 0 0 .4-6.8A5 5 0 0 0 9 11.2 4 4 0 0 0 9 19Z"/>',
    'cloud-moon': '<path d="M16.5 3.5a5 5 0 0 0 3.4 6.4 5.2 5.2 0 1 1-3.4-6.4Z"/><path d="M9 19h9.4a3.4 3.4 0 0 0 .4-6.8A5 5 0 0 0 9 11.2 4 4 0 0 0 9 19Z"/>',
    'cloud-drizzle': '<path d="M7.2 15.5h10.1a3.7 3.7 0 0 0 .4-7.4A5.6 5.6 0 0 0 7 6.9a4.3 4.3 0 0 0 .2 8.6Z"/><path d="M8.5 18.3l-.7 2M12.3 18.3l-.7 2M16.1 18.3l-.7 2"/>',
    'cloud-rain': '<path d="M7.2 14.5h10.1a3.7 3.7 0 0 0 .4-7.4A5.6 5.6 0 0 0 7 5.9a4.3 4.3 0 0 0 .2 8.6Z"/><path d="M8.5 17.4 7.4 21M12.3 17.4 11.2 21M16.1 17.4 15 21"/>',
    'cloud-snow': '<path d="M7.2 14.5h10.1a3.7 3.7 0 0 0 .4-7.4A5.6 5.6 0 0 0 7 5.9a4.3 4.3 0 0 0 .2 8.6Z"/><path d="M8.4 18.2h.01M12 18.2h.01M15.6 18.2h.01M10.2 20.6h.01M13.8 20.6h.01"/>',
    'cloud-lightning': '<path d="M7.2 14.5h10.1a3.7 3.7 0 0 0 .4-7.4A5.6 5.6 0 0 0 7 5.9a4.3 4.3 0 0 0 .2 8.6Z"/><path d="m12.6 16.6-2 4h3.2l-1.4 3.4"/>',
    fog: '<path d="M6 13.5h12M4.5 17h15M7 20.5h10"/><path d="M8 10h8.5a2.9 2.9 0 0 0 .3-5.8A4.6 4.6 0 0 0 8 6.2 2.8 2.8 0 0 0 8 10Z"/>',
    wind: '<path d="M3 8.5h10a2.6 2.6 0 1 0-2.5-3.3"/><path d="M3 12.5h15a2.5 2.5 0 1 1-2.4 3.2"/><path d="M3 16.5h7"/>',
    droplet: '<path d="M12 3.2S6.5 9.4 6.5 13.6a5.5 5.5 0 0 0 11 0C17.5 9.4 12 3.2 12 3.2Z"/>',
    gauge: '<path d="M4 18a8.5 8.5 0 1 1 16 0"/><path d="m12 13.5 4-4"/><circle cx="12" cy="14" r="1.3" fill="currentColor" stroke="none"/>',
    sunrise: '<path d="M12 3v5M9.4 5.6 12 8.2l2.6-2.6"/><path d="M4 17h16"/><path d="M6.5 17a5.5 5.5 0 0 1 11 0"/><path d="M3 21h18"/>',
    sunset: '<path d="M12 8V3M9.4 5.4 12 3l2.6 2.4"/><path d="M4 17h16"/><path d="M6.5 17a5.5 5.5 0 0 1 11 0"/><path d="M3 21h18"/>',
    battery: '<rect x="3" y="7.5" width="16.5" height="9" rx="2.4"/><path d="M21.5 10.5v3"/><rect x="5.5" y="10" width="8" height="4" rx="1" fill="currentColor" stroke="none"/>',
    wifi: '<path d="M3.5 9.2a13 13 0 0 1 17 0"/><path d="M6.4 12.4a8.6 8.6 0 0 1 11.2 0"/><path d="M9.3 15.6a4.2 4.2 0 0 1 5.4 0"/><circle cx="12" cy="19" r="1.2" fill="currentColor" stroke="none"/>',
    eye: '<path d="M2.5 12S6 6 12 6s9.5 6 9.5 6-3.5 6-9.5 6-9.5-6-9.5-6Z"/><circle cx="12" cy="12" r="2.8"/>',
    qr: '<rect x="3.5" y="3.5" width="6" height="6" rx="1"/><rect x="14.5" y="3.5" width="6" height="6" rx="1"/><rect x="3.5" y="14.5" width="6" height="6" rx="1"/><path d="M14.5 14.5h2.5v2.5h-2.5zM20.5 14.5v2M14.5 20.5h2M18.5 18.5h2v2"/>',
    scan: '<path d="M4 8V5.5A1.5 1.5 0 0 1 5.5 4H8M16 4h2.5A1.5 1.5 0 0 1 20 5.5V8M20 16v2.5a1.5 1.5 0 0 1-1.5 1.5H16M8 20H5.5A1.5 1.5 0 0 1 4 18.5V16"/><path d="M4 12h16"/>',
    folder: '<path d="M3.5 7.5A1.5 1.5 0 0 1 5 6h4l2 2.5h8a1.5 1.5 0 0 1 1.5 1.5v8A1.5 1.5 0 0 1 19 19.5H5A1.5 1.5 0 0 1 3.5 18Z"/>',
    download: '<path d="M12 3.5v11"/><path d="m7.5 10.5 4.5 4.5 4.5-4.5"/><path d="M4 18.5h16"/>',
    send: '<path d="M20.5 3.5 3.5 10l6.5 2.5 2.5 7Z"/><path d="m10 12.5 5-5"/>',
    bell: '<path d="M6.5 10a5.5 5.5 0 0 1 11 0c0 4 1.5 5.5 1.5 5.5H5s1.5-1.5 1.5-5.5Z"/><path d="M9.5 19a2.5 2.5 0 0 0 5 0"/>',
    bot: '<rect x="4" y="7" width="16" height="12" rx="3"/><path d="M12 3.5V7"/><circle cx="9" cy="12.5" r="1" fill="currentColor" stroke="none"/><circle cx="15" cy="12.5" r="1" fill="currentColor" stroke="none"/><path d="M9.5 15.5h5"/>',
    globe: '<circle cx="12" cy="12" r="8.5"/><path d="M3.5 12h17"/><path d="M12 3.5c2.4 2.3 3.6 5.2 3.6 8.5s-1.2 6.2-3.6 8.5c-2.4-2.3-3.6-5.2-3.6-8.5S9.6 5.8 12 3.5Z"/>',
    link: '<path d="M10 13.5a4 4 0 0 0 5.7 0l2.8-2.8a4 4 0 0 0-5.7-5.7l-1.2 1.2"/><path d="M14 10.5a4 4 0 0 0-5.7 0l-2.8 2.8a4 4 0 0 0 5.7 5.7l1.2-1.2"/>',
    'arrow-up-right': '<path d="M7 17 17 7"/><path d="M8 7h9v9"/>',
    crop: '<path d="M7 3v14h14"/><path d="M3 7h14v14"/>',
    square: '<rect x="4" y="4" width="16" height="16" rx="2"/>',
    arrow: '<path d="M5 19 19 5"/><path d="M9 5h10v10"/>',
    undo: '<path d="M4 10h10a5 5 0 0 1 0 10h-4"/><path d="m8 6-4 4 4 4"/>',
    grip: '<circle cx="9" cy="7" r="1.2" fill="currentColor" stroke="none"/><circle cx="15" cy="7" r="1.2" fill="currentColor" stroke="none"/><circle cx="9" cy="12" r="1.2" fill="currentColor" stroke="none"/><circle cx="15" cy="12" r="1.2" fill="currentColor" stroke="none"/><circle cx="9" cy="17" r="1.2" fill="currentColor" stroke="none"/><circle cx="15" cy="17" r="1.2" fill="currentColor" stroke="none"/>',
    external: '<path d="M14 4h6v6"/><path d="M20 4 11 13"/><path d="M18 14v4.5A1.5 1.5 0 0 1 16.5 20h-11A1.5 1.5 0 0 1 4 18.5v-11A1.5 1.5 0 0 1 5.5 6H10"/>',
    cpu: '<rect x="7" y="7" width="10" height="10" rx="2"/><rect x="10" y="10" width="4" height="4" rx="0.8"/><path d="M10 3v4M14 3v4M10 17v4M14 17v4M3 10h4M3 14h4M17 10h4M17 14h4"/>',
    memory: '<rect x="3" y="7" width="18" height="9" rx="1.8"/><path d="M6 16v3M10 16v3M14 16v3M18 16v3"/><path d="M7 10h2M11 10h2M15 10h2"/>',
    thermometer: '<path d="M12 14.5V5a2.5 2.5 0 0 1 5 0v9.5a4.5 4.5 0 1 1-5 0Z"/><circle cx="14.5" cy="17.5" r="1.4" fill="currentColor" stroke="none"/>',
    activity: '<path d="M3 12h4l2.5-6 4 12 2.5-6h5"/>',
    network: '<path d="M8 4v9a3 3 0 0 0 3 3h9"/><path d="m17 13 3 3-3 3"/><path d="M16 20v-9a3 3 0 0 0-3-3H4"/><path d="m7 11-3-3 3-3"/>',
    disk: '<rect x="3.5" y="5" width="17" height="14" rx="2.2"/><circle cx="12" cy="12" r="3.2"/><circle cx="12" cy="12" r="0.7" fill="currentColor" stroke="none"/><path d="M6 5v3"/>',
    film: '<rect x="3.5" y="4.5" width="17" height="15" rx="2"/><path d="M8 4.5v15M16 4.5v15M3.5 9h4.5M3.5 15h4.5M16 9h4.5M16 15h4.5"/>',
    grid: '<rect x="4" y="4" width="7" height="7" rx="1.4"/><rect x="13" y="4" width="7" height="7" rx="1.4"/><rect x="4" y="13" width="7" height="7" rx="1.4"/><rect x="13" y="13" width="7" height="7" rx="1.4"/>',
    pencil: '<path d="M4 20h4.5L20 8.5 15.5 4 4 15.5Z"/><path d="m13.5 6 4.5 4.5"/>',
    layout: '<rect x="3.5" y="4" width="17" height="16" rx="2.2"/><path d="M3.5 9h17M9.5 9v11"/>',
    palette: '<path d="M12 3.5a8.5 8.5 0 1 0 0 17h1.2a1.8 1.8 0 0 0 1.2-3.1 1.7 1.7 0 0 1 1.2-2.9h1.9a3 3 0 0 0 3-3A8.4 8.4 0 0 0 12 3.5Z"/><circle cx="7.8" cy="10" r="1" fill="currentColor" stroke="none"/><circle cx="10.5" cy="7" r="1" fill="currentColor" stroke="none"/><circle cx="14.5" cy="7.5" r="1" fill="currentColor" stroke="none"/>',
    monitor: '<rect x="3" y="4.5" width="18" height="12" rx="2"/><path d="M8 20h8M12 16.5V20"/>',
    keyboard: '<rect x="2.5" y="6.5" width="19" height="11" rx="2"/><path d="M6 10h.01M9 10h.01M12 10h.01M15 10h.01M18 10h.01M6 13h.01M18 13h.01M9 13.5h6"/>',
    brush: '<path d="M14.5 4.5 19.5 9.5 11 18H6v-5Z"/><path d="m13 6 5 5"/><path d="M6 18c-1.5 0-2.5 1-3 3 2 0 3-1 3-3Z"/>',
    eraser: '<path d="m15.5 4.5 4 4a2 2 0 0 1 0 2.8l-7.2 7.2H7l-2.5-2.5a2 2 0 0 1 0-2.8l8.2-8.2a2 2 0 0 1 2.8 0Z"/><path d="m9 10 5 5"/><path d="M7 18.5h13"/>',
    expand: '<path d="M9 4H4v5M15 4h5v5M9 20H4v-5M15 20h5v-5"/>',
    minus: '<path d="M5 12h14"/>',
  };
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
>
  {@html ICONS[name] ?? ICONS.cloud}
</svg>
