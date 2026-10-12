import { strict as assert } from "node:assert";
import { test } from "node:test";
import {
  formatMoney,
  formatQuantity,
  formatRate,
} from "../src/formatting/index.ts";
import type { ExactValue } from "../src/formatting/index.ts";
test("USD thresholds use unrounded amounts, preserve sign and distinguish tiny nonzero", () => {
  for (const [value, display] of [
    ["0", "$0.00"],
    ["0.004", "<$0.01"],
    ["-0.004", "−<$0.01"],
    ["0.01", "$0.01"],
    ["12.34", "$12.34"],
    ["99.999", "$100.00"],
    ["100.25", "$100"],
    ["12345.67", "$12,346"],
    ["-12345.67", "−$12,346"],
    ["-0.00", "$0.00"],
  ])
    assert.equal(formatMoney(value), display);
});
test("huge amounts and rationals never pass through a JS number", () => {
  assert.equal(formatMoney("9007199254740993.50"), "$9,007,199,254,740,994");
  assert.equal(formatMoney({ numerator: "1", denominator: "3" }), "$0.33");
  assert.equal(
    formatMoney({
      numerator: "1000000000000000000000000000000000001",
      denominator: "1000000000000000000",
    }),
    "$1,000,000,000,000,000,000",
  );
});
test("currency minor units follow Intl metadata, without foreign USD symbols", () => {
  assert.equal(formatMoney("12.6", "JPY"), "¥13");
  assert.equal(formatMoney("0.1", "JPY"), "<¥1");
  assert.match(formatMoney("12.3456", "KWD"), /12\.346$/);
  assert.equal(formatMoney("12.34", "EUR"), "€12.34");
});
test("scaled privacy selects precision after scaling and preserves signed proportions", () => {
  const p = { mode: "scaled", scale: "0.1" } as const;
  assert.equal(formatMoney("1000.50", "USD", p), "$100");
  assert.equal(formatMoney("500", "USD", p), "$50.00");
  assert.equal(formatMoney("-10", "USD", p), "−$1.00");
  assert.equal(formatQuantity("50000000000000000000", 18, p), "5");
  assert.equal(formatMoney("1", "USD", { mode: "scaled" }), "$0.10");
  assert.equal(
    formatMoney("40", "USD", { mode: "scaled", scale: "0.25" }),
    "$10.00",
  );
  for (const scale of ["0", "1", "-1", "2", "NaN", "0.0"])
    assert.throws(() => formatMoney("1", "USD", { mode: "scaled", scale }));
});
test("hidden presentation masks amounts and exact quantities", () => {
  assert.equal(formatMoney("123456789", "USD", { mode: "hidden" }), "••••");
  assert.equal(formatQuantity("123456789", 18, { mode: "hidden" }), "••••");
});
test("quantity trimming retains smallest units and full exact large balances", () => {
  assert.equal(formatQuantity("1", 18), "0.000000000000000001");
  assert.equal(formatQuantity("123000", 5), "1.23");
  assert.equal(formatQuantity("-123000", 5), "−1.23");
  assert.equal(
    formatQuantity("123456789012345678901234567890000000000000000001", 18),
    "123456789012345678901234567890.000000000000000001",
  );
  assert.equal(formatQuantity("0", 18), "0");
  assert.equal(
    formatQuantity("1", 18, { mode: "scaled", scale: "0.1" }),
    "0.0000000000000000001",
  );
});
test("rates retain exact ratios and small nonzero values, independent of privacy", () => {
  assert.equal(formatRate("0.04"), "4%");
  assert.equal(formatRate("0"), "0%");
  assert.equal(formatRate({ numerator: "1", denominator: "3" }), "33.33%");
  assert.equal(formatRate("0.00001"), "<0.01%");
  assert.equal(formatRate("-0.00001"), "−<0.01%");
});
test("reject lossy inputs and invalid scales/ratios rather than guessing", () => {
  for (const value of [
    "1e3",
    "NaN",
    " .5",
    "1.2.3",
    0.05,
    { numerator: 1, denominator: "3" },
    { numerator: "1", denominator: "0" },
    { numerator: "1", denominator: "-2" },
  ])
    assert.throws(() => formatMoney(value as ExactValue));
  for (const scale of [-1, 256, 0.5])
    assert.throws(() => formatQuantity("1", scale));
  assert.throws(() => formatQuantity("1.0", 18));
  assert.throws(() => formatQuantity(123 as unknown as string, 18));
});

test("non-ISO reporting assets retain their denomination and privacy", () => {
  assert.equal(formatMoney("1.25", "USDT"), "1.25 USDT");
  assert.equal(formatMoney("1000", "crypto:asset"), "1,000 crypto:asset");
  assert.equal(formatMoney("1000", "USDT", { mode: "hidden" }), "••••");
});
