
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
        - spall tracing.
            - write to file.
                - set path in setup func.
                - writer checks whether has file in each iter.
                    - if not, checks global context for path.
                    - if still no file, drop buffer and record that.
                - write header.
                    - setup function.
            - thread local temp buffer for arg formatting.
                - prob use unsafe to gatekeep access.
            - record & log drop/trunc events.
        - add spall tracing to rug.
        - port dynamic svg parser using xmlparser.
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


