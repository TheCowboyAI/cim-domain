<!-- Copyright (c) 2025 - Cowboy AI, LLC. -->

# Dialog DAG Tools

Utilities for maintaining `dialog-dag.json` outside the pure `cim-domain` library.

- `log_dialog_event`: Append a dialog event to an existing `dialog-dag.json` using a CID computed from the event content (Blake3 -> Multihash 0x1e -> CIDv1 0x55), matching the tooling in `cim-ipld`.
- `merge_dialog_dag`: Merge a continuation file into a main `dialog-dag.json`, de-duplicating by `cid` and preserving time order.

These are provided as a standalone Cargo crate under `tools/` to keep the core library pure (no I/O). Build and use from this subdirectory:

```
cd tools/dialog_dag
cargo run --bin log_dialog_event -- \
  ../..//dialog-dag.json assistant \
  "Short summary of my message" \
  "What I understood" \
  "action1;action2;action3"

cargo run --bin merge_dialog_dag -- \
  ../../dialog-dag.json path/to/continuation.json
```

Notes
- The top-level JSON layout expected is `{ "events": [ ... ], "total_events": N, ... }`.
- `cid` is computed from the serialized event content; do not hand-edit content after logging or the content-derived CID will no longer match.
- The main repo’s `dialog-dag.json` should be committed alongside code to preserve conversation provenance.
