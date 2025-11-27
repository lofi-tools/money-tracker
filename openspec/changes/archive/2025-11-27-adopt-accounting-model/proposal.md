# Adopt Accounting Transaction Model

## Context
The project currently has a basic transaction model defined in the specs. We want to adopt a more robust double-entry accounting model inspired by `nmrshll/accounting`.

## Problem
The current `Transaction` model with separate inputs and outputs doesn't fully capture the nature of atomic financial movements where money moves from one account to another in a balanced way.

## Solution
Adopt the `Transaction` and `TxEffect` (based on `TxEffect`) data model.
- A `Transaction` consists of a list of `TxEffect`s (effects) and a timestamp.
- A `TxEffect` represents a change in an account's balance (`amount_diff`) at a specific time.
- This allows representing complex transfers (1-to-1, 1-to-many, many-to-many) uniformly.

## Impact
- `libs/core` will be updated to use the new structs.
- Existing code using `Transaction` will need to be refactored.
