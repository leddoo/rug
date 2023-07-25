
- todo: restore usable v1.
    - paths.
        - segment -> line.
        - iteration.
    - command buffer.
        - with owned paths in arena,
          but built using `PathBuilder<GlobalAlloc>`, cause who cares.
          leak the path buffers. give user `Path<'a>`.

    - maybe use `extend` for path builder point pushes.
    - `Drop` tests for sti vec. truncate does not look right...
    - oom api - ipgui needs that to some extent (where dom nodes own allocations,
      not really for the renderer, we're sol if that panics due to oom).

    - fix image u32/usize nonsense. ~ prob use usize everywhere.
    - compositing (solid color fill using mask Img).
    - renderer abstraction.

- stuff:
    - `FixedVec` for segment buffer ~ uninit.
    - paths.
    - stroking.

- opt vid:
    - tiling.
    - multi-threading.
    - traditional active list rasterizer.



- backlog:
    - dashing.
    - effects.
    - a proper api.
    - multi-threading.


