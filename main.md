
- next steps:
    - renderer.
    - strokes.
    - shapes.
    - advanced stroking: caps, joins, dashing.
    - effects.
    - tiling.
    - multi-threading.
    - large path rasterizer.



- todo: restore usable v1.
    - renderer:
        - copy-expand.
        - port dynamic svg parser.
            - is there a non-allocating xml parser?
        - write tiger as image in example.

    - maybe use `extend` for path builder point pushes.
    - `Drop` tests for sti vec. truncate does not look right...
    - oom api - ipgui needs that to some extent (where dom nodes own allocations,
      not really for the renderer, we're sol if that panics due to oom).

    - fix image u32/usize nonsense. ~ prob use usize everywhere.
    - `FixedVec` for segment buffer ~ uninit.

    - support uninit for image.
        - users must write using pointer methods.
        - `render` supports uninit images.


