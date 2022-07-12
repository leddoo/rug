todo:
- use Image in rasterizer.
- default effect:
    - i guess that's just `Tile::execute`.
    - but also fill visible.
    - thinking make `Effect` trait.
- shadow effect.

- allocators.

- aot strokes in main
- rayon mt in main.

- boundary fragment rasterizer:
    - use trait for shared impl.
    - use 4x dda.
    - and have fn in trait for 4x boundary fragments + masks.

- renderer state: transform, tolerances, etc.

- high level api:
    - (range, sync closure)?
    - well, how do we get mut refs into vecs?
    - maybe just have a rayon feature. i don't really care about not using rayon anymore.



- try fixing `Path<A>` nonsense.
    - i guess let's just not make `Path: Send`.
    - could have a `SendPath`, which wraps a `Path`, just for sending it.
    - could think about the bool approach.
        - but that's pretty messy tbh.
        - idk, maybe it's fine.
        - `type Path<'a> = BasePath<'a, false>` ~ default is not Send.
- stroke -> Path

- command buffer.
    - consider stages (for manually chaining command buffers).
    - "config":
        - initial transform.
        - clear color.
        - tile size (optional tiling?).
        - allocator.
    - thread pool interface.
- clean up:
    - color module.
    - use logfiler: bring back drawing metrics & other diagnostics.


command buffer.
- state:
    - approximation parameters.
    - composition function.
    - clip rect/path.
- fill params:
    - shape.
    - shader.
        - variable length, commonly shared -> external.
        - put shader type into command, separate arrays.
- stroke params:
    - shape.
    - shader.
    - cap/join style.
        - 2 + 2 bits.
        - or 3 if combined.
    - width.
        - only 4 bytes, so indexing doesn't make sense.
        - put into command.
        - maybe external when dashed to only need 4 bytes.
        - left/right? ignore for now. this would be rare -> special command.
    - dashing.
        - rare, variable length, shared -> external.

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
- how to deal with open paths fills?
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
