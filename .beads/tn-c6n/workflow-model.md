# Workflow model — tn-c6n

## Legal transitions

1. Raw input (caller string / JSON text / timestamps / counts) → smart
   constructor → `Ok(value)` or `Err(typed variant)`.
2. Assembled `ReceiptEnvelope` → JSON text (`Serialize`, total) → JSON text
   back into `ReceiptEnvelope` (`Deserialize`, fallible, schema-first).
3. No other transitions exist: values cannot be mutated after construction
   (all fields private; only total accessors exposed).

## Guards

Each constructor applies its guard before storing anything: non-empty,
NUL-free, `passed <= scanned`, `finished >= started`, absolute path, schema
equality. Guards are pure predicates over the inputs.

## Terminal states

`Ok(value)` (frozen valid value) or `Err(ReceiptError)` (typed rejection with
all offending inputs carried in the variant). There is no partial-value state.

## Forbidden transitions

- JSON text → value without passing the constructor (both `Deserialize`
  impls funnel through `new`; the envelope checks schema first).
- Value → mutated value (no `&mut` APIs on any receipt type).
