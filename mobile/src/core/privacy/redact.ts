// Log ve hata kayıtlarına kişisel veri yazılmasını önler (KVKK m.12, gereksinim 5 "Gizlilik").

const TEXT_PATTERNS: readonly [RegExp, string][] = [
  [/\bBearer\s+[\w.~+/-]+=*/gi, 'Bearer [anahtar]'],
  [/\b[\w-]{10,}\.[\w-]{10,}\.[\w-]{10,}\b/g, '[anahtar]'],
  [/[\w.%+-]+@[\w.-]+\.[a-z]{2,}/gi, '[e-posta]'],
  [/(?:\+?90[\s-]?)?\(?0?5\d{2}\)?[\s-]?\d{3}[\s-]?\d{2}[\s-]?\d{2}\b/g, '[telefon]'],
  [/\b[1-9]\d{10}\b/g, '[kimlik-no]'],
];

const SENSITIVE_KEYS = new Set([
  'accesstoken',
  'refreshtoken',
  'token',
  'authorization',
  'password',
  'pin',
  'code',
  'phone',
  'email',
  'allergens',
  'description',
  'note',
  'fulltext',
]);

const MAX_DEPTH = 4;

export function redact(text: string): string {
  return TEXT_PATTERNS.reduce((result, [pattern, replacement]) => result.replace(pattern, replacement), text);
}

export function redactValue(value: unknown, depth = 0): unknown {
  if (typeof value === 'string') return redact(value);
  if (value === null || typeof value !== 'object') return value;
  if (depth >= MAX_DEPTH) return '[…]';
  if (value instanceof Error) return { name: value.name, message: redact(value.message) };
  if (Array.isArray(value)) return value.map((item) => redactValue(item, depth + 1));
  return Object.fromEntries(
    Object.entries(value).map(([key, item]) => [
      key,
      SENSITIVE_KEYS.has(key.toLowerCase()) ? '[gizli]' : redactValue(item, depth + 1),
    ]),
  );
}
