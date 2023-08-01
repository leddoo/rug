
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
    - command metadata for debugging.

    - `Vec::extend` for path builders.

    - stroker:
        - configurable joins & caps.
        - dashing.
        - optimization.
            - allocations.
            - merge left pass into offset pass?

    - spall tracing.
        - global comp time disable switch.
        - separate repo.
        - thread local temp buffer for arg formatting.
            - prob use unsafe to gatekeep access.
        - record & log drop/trunc/write-fail events.

    - maybe use `extend` for path builder point pushes.
    - `Drop` tests for sti vec. truncate does not look right...
    - oom api - ipgui needs that to some extent (where dom nodes own allocations,
      not really for the renderer, we're sol if that panics due to oom).

    - fix image u32/usize nonsense. ~ prob use usize everywhere.
    - `FixedVec` for segment buffer ~ uninit.

    - support uninit for image.
        - users must write using pointer methods.
        - `render` supports uninit images.


