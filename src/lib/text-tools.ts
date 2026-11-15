// Pure text transformations used by the Text module. No DOM, no Tauri.

const EN_LOWER = "qwertyuiopasdfghjklzxcvbnm[];',./";
const UK_LOWER = "йцукенгшщзхфівапролджячсмитьхїжєюб";
const RU_LOWER = "йцукенгшщзхфывапролджячсмитьхъжэюб";

// The two strings above are position-aligned: index maps EN key -> local key.
// EN row order here is qwertyuiop / asdfghjkl / zxcvbnm followed by [] ; ' , . /
const EN_KEYS = "qwertyuiop" + "asdfghjkl" + "zxcvbnm" + "[];',./";
const UK_KEYS = "йцукенгшщзх" + "фівапролдж" + "ячсмить" + "хїжєюб";
const RU_KEYS = "йцукенгшщзх" + "фывапролдж" + "ячсмить" + "хъжэюб";

function buildMap(from: string, to: string): Map<string, string> {
  const map = new Map<string, string>();
  for (let i = 0; i < from.length && i < to.length; i += 1) {
    map.set(from[i], to[i]);
    map.set(from[i].toUpperCase(), to[i].toUpperCase());
  }
  return map;
}

const EN_TO_UK = buildMap(EN_KEYS, UK_KEYS);
const EN_TO_RU = buildMap(EN_KEYS, RU_KEYS);
const UK_TO_EN = buildMap(UK_KEYS, EN_KEYS);
const RU_TO_EN = buildMap(RU_KEYS, EN_KEYS);

function applyMap(text: string, map: Map<string, string>): string {
  let out = "";
  for (const char of text) {
    out += map.get(char) ?? char;
  }
  return out;
}

export type LayoutDirection = "auto" | "en-uk" | "uk-en" | "en-ru" | "ru-en";

export function switchLayout(text: string, direction: LayoutDirection): string {
  const hasCyrillic = /[\u0400-\u04FF]/.test(text);
  if (direction === "auto") {
    return hasCyrillic ? applyMap(text, UK_TO_EN) : applyMap(text, EN_TO_UK);
  }
  switch (direction) {
    case "en-uk":
      return applyMap(text, EN_TO_UK);
    case "uk-en":
      return applyMap(text, UK_TO_EN);
    case "en-ru":
      return applyMap(text, EN_TO_RU);
    case "ru-en":
      return applyMap(text, RU_TO_EN);
    default:
      return text;
  }
}

export function toUpper(text: string): string {
  return text.toLocaleUpperCase();
}

export function toLower(text: string): string {
  return text.toLocaleLowerCase();
}

export function toTitle(text: string): string {
  return text.replace(/(^|[\s\-—–(])(\p{L})/gu, (_m, sep: string, ch: string) => sep + ch.toLocaleUpperCase());
}

export function toSentence(text: string): string {
  const lower = text.toLocaleLowerCase();
  return lower.replace(/(^\s*|[.!?]\s+|\n\s*)(\p{L})/gu, (_m, sep: string, ch: string) => sep + ch.toLocaleUpperCase());
}

const TRANSLIT: Record<string, string> = {
  а: "a", б: "b", в: "v", г: "h", ґ: "g", д: "d", е: "e", є: "ie", ж: "zh",
  з: "z", и: "y", і: "i", ї: "i", й: "i", к: "k", л: "l", м: "m", н: "n",
  о: "o", п: "p", р: "r", с: "s", т: "t", у: "u", ф: "f", х: "kh", ц: "ts",
  ч: "ch", ш: "sh", щ: "shch", ь: "", ю: "iu", я: "ia", "'": "", "’": "",
};

const INITIAL: Record<string, string> = {
  є: "Ye", ї: "Yi", й: "Y", ю: "Yu", я: "Ya",
};

export function transliterate(text: string): string {
  let result = "";
  let atWordStart = true;
  for (const char of text) {
    const lower = char.toLocaleLowerCase();
    const mapped = atWordStart ? (INITIAL[lower] ?? TRANSLIT[lower]) : TRANSLIT[lower];
    if (mapped === undefined) {
      result += char;
      atWordStart = /[\s.,!?;:()\[\]"«»—–\-]/.test(char);
      continue;
    }
    const isUpper = char !== lower;
    const out = isUpper ? mapped.charAt(0).toUpperCase() + mapped.slice(1) : mapped;
    result += out;
    atWordStart = false;
  }
  return result;
}

const REVERSE_TRANSLIT: Array<[string, string]> = [
  ["shch", "щ"], ["zh", "ж"], ["kh", "х"], ["ts", "ц"], ["ch", "ч"],
  ["sh", "ш"], ["iu", "ю"], ["ia", "я"], ["ie", "є"], ["yi", "ї"],
  ["ye", "є"], ["yu", "ю"], ["ya", "я"], ["a", "а"], ["b", "б"],
  ["v", "в"], ["h", "г"], ["g", "ґ"], ["d", "д"], ["e", "е"],
  ["z", "з"], ["y", "и"], ["i", "і"], ["k", "к"], ["l", "л"],
  ["m", "м"], ["n", "н"], ["o", "о"], ["p", "п"], ["r", "р"],
  ["s", "с"], ["t", "т"], ["u", "у"], ["f", "ф"], ["c", "к"],
  ["q", "к"], ["w", "в"], ["x", "кс"], ["j", "й"],
];

export function reverseTransliterate(text: string): string {
  let result = "";
  let i = 0;
  const lower = text.toLocaleLowerCase();
  while (i < text.length) {
    let matched = false;
    for (const [latin, cyr] of REVERSE_TRANSLIT) {
      if (lower.startsWith(latin, i)) {
        const original = text.slice(i, i + latin.length);
        const isUpper = original[0] === original[0].toLocaleUpperCase() && original[0] !== original[0].toLocaleLowerCase();
        result += isUpper ? cyr.toLocaleUpperCase() : cyr;
        i += latin.length;
        matched = true;
        break;
      }
    }
    if (!matched) {
      result += text[i];
      i += 1;
    }
  }
  return result;
}

export function collapseWhitespace(text: string): string {
  return text
    .replace(/[ \t]+/g, " ")
    .replace(/ *\n */g, "\n")
    .replace(/\n{3,}/g, "\n\n")
    .trim();
}

export function removeEmptyLines(text: string): string {
  return text
    .split(/\r?\n/)
    .filter((line) => line.trim().length > 0)
    .join("\n");
}

export function formatJson(text: string, minify = false): string {
  const parsed = JSON.parse(text);
  return JSON.stringify(parsed, null, minify ? 0 : 2);
}

function utf8ToBase64(text: string): string {
  const bytes = new TextEncoder().encode(text);
  let binary = "";
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return btoa(binary);
}

function base64ToUtf8(text: string): string {
  const binary = atob(text.trim());
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
  return new TextDecoder().decode(bytes);
}

export function encodeBase64(text: string): string {
  return utf8ToBase64(text);
}

export function decodeBase64(text: string): string {
  return base64ToUtf8(text);
}

export function sortLines(text: string): string {
  return text
    .split(/\r?\n/)
    .sort((a, b) => a.localeCompare(b, undefined, { numeric: true, sensitivity: "base" }))
    .join("\n");
}

export function dedupeLines(text: string): string {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const line of text.split(/\r?\n/)) {
    if (!seen.has(line)) {
      seen.add(line);
      out.push(line);
    }
  }
  return out.join("\n");
}

export function generatePassword(length = 20): string {
  const alphabet = "ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789!@#$%^&*-_=+";
  const random = new Uint32Array(length);
  crypto.getRandomValues(random);
  let out = "";
  for (let i = 0; i < length; i += 1) {
    out += alphabet[random[i] % alphabet.length];
  }
  return out;
}

export function generateUuid(): string {
  if (typeof crypto.randomUUID === "function") return crypto.randomUUID();
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}

export function countWords(text: string): number {
  const trimmed = text.trim();
  return trimmed ? trimmed.split(/\s+/).length : 0;
}
