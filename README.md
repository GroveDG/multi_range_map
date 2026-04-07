# Multi-Range Map

`multi_range_map` is a crate providing the `MultiRangeMap` and a set of related bimaps. Unlike other range map crates, `MultiRangeMap` supports overlapping ranges meaning that one index can map to multiple values.

`MultiRangeMap` is implemented by keeping track of all the bounds of ranges and storing the value of all ranges which overlap those bounds. These values are stored in [`SmallVec`s](https://docs.rs/smallvec/latest/smallvec/struct.SmallVec.html) and the default size of these `SmallVec`s is controlled by the const generic `N` with a default of 4.

There are also a set of bimaps which map values back to ranges.
- `MultiRangeHashMap` uses `HashMap`.
- `MultiRangeBTreeMap` uses `BTreeMap`.
- `MultiRangeSlotMap` uses `SlotMap` and is enabled with the `slotmap` feature.