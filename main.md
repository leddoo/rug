todo:
- visible paths bit vectors.
- rasterizer optimization:
    - aabb rejection, unless path is mostly contained.
    - add_segment: assume clamped, F32x2, branchless loop.
    - clamp segments, unless path is fully contained.
    - simd accumulate.
    - buffered simd segment clamping.
- multi-threading.
- path transforming.
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
- simd-pre-process rasterizer.
- perf:
    - 4x2.
    - floor/ceil: clamp first -> positive.
        - can use unchecked cast for flooring.
        - ceil: floor(x + (1 - 1ulp))
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




optimization ideas:
- rasterizer:
    - skip horizontal segments.
    - special case for vertical segments.



stuff:

```rust
#![feature(portable_simd)]
    use std::simd::*;

    let a = u8x16::from_array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    let b = u8x16::from_array([3, 2, 1, 3, 2, 1, 3, 2, 1,  3,  2,  1,  3,  2,  1,  3]);
    let mask: u8x16 = mask8x16::from_bitmask(0b1010_0011_1100_0101).to_int().cast();

    let mut foo = Vec::<u8x16>::with_capacity(1024*1024*1024);
    unsafe { foo.set_len(foo.capacity()); }
    let foo = foo.into_boxed_slice();
    //let foo = vec![u8x16::splat(42); 1024*1024*1024].into_boxed_slice();
```


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
