/** Display arithmetic only: never use these helpers to reconcile or estimate yield. */
export interface Rational {
  numerator: string;
  denominator: string;
}
export type ExactValue = string | Rational;
export type Privacy = { mode: "normal" | "hidden" | "scaled"; scale?: string };
export const normalPrivacy: Privacy = { mode: "normal" };
export const defaultScale = "0.1";
function rational(value: ExactValue): [bigint, bigint] {
  if (typeof value !== "string") {
    if (
      typeof value.numerator !== "string" ||
      typeof value.denominator !== "string" ||
      !/^-?\d+$/.test(value.numerator) ||
      !/^\d+$/.test(value.denominator)
    )
      throw new Error("Invalid rational");
    const d = BigInt(value.denominator);
    if (d <= 0n) throw new Error("Denominator must be positive");
    return [BigInt(value.numerator), d];
  }
  if (!/^[+-]?\d+(\.\d+)?$/.test(value))
    throw new Error("Plain exact decimal required");
  const scale = value.split(".")[1]?.length ?? 0;
  return [BigInt(value.replace(".", "")), 10n ** BigInt(scale)];
}
function scaled(value: ExactValue, privacy: Privacy): [bigint, bigint] {
  const [n, d] = rational(value);
  if (privacy.mode !== "scaled") return [n, d];
  const [a, b] = rational(privacy.scale ?? defaultScale);
  if (a <= 0n || a >= b)
    throw new Error("Privacy scale must be between zero and one");
  return [n * a, d * b];
}
function abs(n: bigint) {
  return n < 0n ? -n : n;
}
function rounded(n: bigint, d: bigint, digits: number): bigint {
  const p = 10n ** BigInt(digits);
  return (abs(n) * p * 2n + d) / (2n * d);
}
function currencyString(
  units: bigint,
  digits: number,
  currency: string,
  locale: string,
): string {
  const text = units.toString().padStart(digits + 1, "0");
  const whole = digits ? text.slice(0, -digits) : text;
  const fraction = digits ? text.slice(-digits) : "";
  const grouped = new Intl.NumberFormat(locale, {
    maximumFractionDigits: 0,
  }).format(BigInt(whole));
  const formatter = new Intl.NumberFormat(locale, {
    style: "currency",
    currency,
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  });
  return formatter
    .formatToParts(0)
    .map((p) =>
      p.type === "integer"
        ? grouped
        : p.type === "fraction"
          ? fraction
          : p.value,
    )
    .join("");
}
export function formatMoney(
  value: ExactValue,
  currency = "USD",
  privacy: Privacy = normalPrivacy,
  locale = "en-US",
): string {
  if (privacy.mode === "hidden") return "••••";
  const [n, d] = scaled(value, privacy);
  const minor =
    new Intl.NumberFormat(locale, {
      style: "currency",
      currency,
    }).resolvedOptions().maximumFractionDigits ?? 2;
  const sign = n < 0n ? "−" : "";
  const a = abs(n);
  if (a !== 0n && a * 10n ** BigInt(minor) < d)
    return `${sign}<${currencyString(1n, minor, currency, locale)}`;
  const digits = a < 100n * d ? minor : 0;
  return sign + currencyString(rounded(n, d, digits), digits, currency, locale);
}
/** Full exact quantity; trimming zeros never turns a small nonzero into zero. */
export function formatQuantity(
  units: string,
  scale: number,
  privacy: Privacy = normalPrivacy,
): string {
  if (!Number.isInteger(scale) || scale < 0 || scale > 255)
    throw new Error("Invalid asset scale");
  if (typeof units !== "string" || !/^-?\d+$/.test(units))
    throw new Error("Integer units required");
  if (privacy.mode === "hidden") return "••••";
  let [n, d] = scaled(
    { numerator: units, denominator: (10n ** BigInt(scale)).toString() },
    privacy,
  );
  const sign = n < 0n ? "−" : "";
  n = abs(n);
  const whole = n / d;
  let remainder = n % d;
  let fraction = "";
  // Decimal privacy factors and protected quantity scales always terminate.
  while (remainder !== 0n) {
    remainder *= 10n;
    fraction += (remainder / d).toString();
    remainder %= d;
  }
  return sign + whole.toString() + (fraction ? "." + fraction : "");
}
/** Rates remain visible in privacy modes. Input is an annual fraction. */
export function formatRate(value: ExactValue): string {
  const [n, d] = rational(value);
  const percent = n * 100n;
  const sign = percent < 0n ? "−" : "";
  if (percent !== 0n && abs(percent) * 100n < d) return `${sign}<0.01%`;
  const text = rounded(percent, d, 2).toString().padStart(3, "0");
  const decimal = (text.slice(0, -2) + "." + text.slice(-2)).replace(
    /\.?0+$/,
    "",
  );
  return sign + decimal + "%";
}
