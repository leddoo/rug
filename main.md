
- next steps: complete rendering of vg-inputs.
    - gradients.
    - advanced stroking: caps, joins, dashing.
    - arcs.
    - clipping (rect & path).


- gradients.
    - position specification:
        - either absolute, relative to the (untransformed) origin.
            - the same coordinate system as that of the points in a path.
        - or percentage, relative to the (untransformed) bounding box.
            - the axis aligned bounding box in the path point coordinate system.


- todo:
    - linear gradients.
        - impl `fill_mask_linear_gradient_2`.
            - compute vectors.
            - compute t.
            - lerp.
        - parse svg.
        - impl `fill_mask_linear_gradient_n`.
        - consider removing `GradientStops2`.
            - the indirection is cheap, compared to the actual fill.
            - storage doesn't really matter. it's the specialized algo that matters.

    - command metadata for debugging.

    - sti:
        - `Vec::extend` for path builders.
        - Vec drop tests & fix truncate.
        - thread local temp arena (dynamic stack enforcement).

    - fix image u32/usize nonsense. ~ prob use usize everywhere.
    - `FixedVec` for segment buffer ~ uninit.

    - optimization:
        - stroker:
            - allocations.
            - merge left pass into offset pass?

    - spall tracing.
        - global comp time disable switch.
        - separate repo.
        - thread local temp buffer for arg formatting.
            - prob use unsafe to gatekeep access.
        - record & log drop/trunc/write-fail events.


    - support uninit for image.
        - users must write using pointer methods.
        - `render` supports uninit images.


- backlog:
    - shapes.
    - groups (shared opacity).
    - text.
    - effects (shadows, etc).
    - transforms.
    - image sources.
    - compositing.
    - winding rule.
    - tiling.
    - multi-threading.
    - large path rasterizer.
    - canvas api.
    - oom api.
        - ipgui needs that to some extent (where dom nodes own allocations,
          not really for the renderer, we're sol if that panics due to oom).


