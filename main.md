todo:
- command buffer.
    - command buffer data structure.
- configurable stroker tolerances.
    - zero tolerance is too tight for "sloppy" paths.

stuff:
- need arena before can continue optimization.
- consider `Path` for stroker:
    - just write to two paths.
    - then reverse append.
    - for segment only paths, the SOA path is almost twice as large.
- consider `Image<channels, simd_width>`.
- mt:
    - `Sync + Send` shouldn't actually be an allocator requirement.
    - guess we need `Image<A: Allocator>`.

command buffer:
- initial api:
    - paths.
    - solid color fill/stroke.
    - stroke width.
    - transforms.
- what we need impl wise:
    - some repr of the command buffer.
    - exec:
        - stroke.
        - transform.
        - tile command masks.
- granularity:
    - might want to cache strokes.
    - best would be to do all exec steps in one pass.
        - so tile masks should be optional.


optimization:
- large paths.
    - what does blend2d do?
    - try global boundary fragment rasterization with per-tile binning/sorting and delta_out/winding_in.
    - fill_mask: remove branches.
- not accumulate_runs.
    - ~90% of fragments come from runs >= 4.
    - but when skipping large fills/strokes (w or h > 30), only 5% do.
    - this suggests trying the boundary fragment binning thing first.
    - but that will require quite a bit of restructuring (stroker).
    - so let's make the thing usable first.

stuff:
- put `shift` into simd.rs as shift_lanes_up.
    - try `match amount` + zero shuffle. maybe isel gets it.
- logging utils.
    - scoping.
    - hashmap counter util. (key, number-of-occurences)
- get rid of `crate::float`? or at least make it fast.
- try 4 wide fill_mask.
    - maybe not.
    - 2 vectors should help with ipc.
    - but maybe 4x2 for better work efficiency.
        - first align masks.
        - then determine work efficiency.
        - and estimate gains from going 4x2.
- circular & elliptic arcs.
- stroke: round, miter, square.
- rasterizer interface?

stuff:
- multi-threading.
- path transforming.
    - stroking?
- virtual arena.
- quality of life:
    - fix mouse position (y is up).
    - pan/zoom.
        - matrix.
        - path transform.
        - maths.
    - drag & drop.
    - simple benchmark:
        - render at different resolutions.
        - on press `b`.
        - render returns stats instead of printing.
- dashing.
- more shaders.
- drops.svg
- dynamic pipeline.


stuff:
- stroking:
    - adjust flatten tolerance based on stroke width.
        - reject based on ZERO_TOLERANCE.
        - might want to increase recursion?
    - could a "hairline stroker" improve performance for very thin strokes?
    - more caps & joins: o' = c' + d R ((c'' sqrt(c' c')) - (c' 1/sqrt(c' c') (c'' c'))) / (c' c')
- consider "line, quad, cubic" & "path segment".
- robustness:
    - inf/nan.
    - scale.
    - consistent rule for tolerance & inclusion (lt vs le).
- safer window abstraction.
- static assert sizes & alignments.
- optimized out-of-bounds rasterization:
    - skip non-left curves.
    - approximate monotonic parts as segments.
    - project segments onto left edge.
    - simple fill loop.
- pipeline: fill-rect.
- command buffer.
- text.


invariants:
- rect: min <= max
- curve approx functions: in-order output.
- paths finite.



rust rules:
- ref cannot outlive referent.
- &mut cannot be aliased (refer to same memory).
- "unsafe":
    - deref raw pointers.
    - access mut/extern statics.
    - read union field.
    - call unsafe fn.
    - implement unsafe trait.
- unsafe fn/trait means: read the docs.
    - in general, when using a type, function, or trait in an unsafe way, refer to the corresponding safety docs.
- `PhantomData<T>`: use when struct "contains" a `T`, but fields don't reflect that fact.
    - seems to be about variance & the drop check.
    - any strictly positive occurence of `T` seems to add a lifetime constraint for `T`.
- interior mutability must always use `UnsafeCell`.
- soundness in rust:
    - program has no UB.
    - safe code cannot cause unsafe code to trigger UB.
- niche: invalid bit patterns of a type.
- `Unique<*T>` wrapper:
    - covariant over T.
    - may own a T (for drop check).
    - send/sync iff T send/sync.
    - pointer is non-null.
    - `NonNull<T>` with `PhantomData<T>` and impls for send/sync has the same effects.
