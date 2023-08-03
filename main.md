
- next steps: complete rendering of vg-inputs.
    - gradients.
    - advanced stroking: caps, joins, dashing.
    - arcs.
    - shapes.
    - clipping (rect & path).
        - path, thinking render clipped contents to aabb clipped temp buffer.
        - then render clip path with image source.
        - use path for non-aabb rects.
    - effects (blurs, shadows, etc).
    - groups (shared opacity).
    - winding rule.


- gradients.
    - position specification:
        - either absolute, relative to the (untransformed) origin.
            - the same coordinate system as that of the points in a path.
        - or percentage, relative to the (untransformed) bounding box.
            - the axis aligned bounding box in the path point coordinate system.


- todo:
    - radial gradients.
        - inverse transform or solid fill.
        - or line segment gradient for non-pad stretch, ig?
        - ignore degen transforms for now.
    - impl `fill_mask_linear_gradient_n`.
    - impl `fill_mask_radial_gradient_n`.
    - doc comments for the repr.
    - todos for unsupported gradient properties.

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
    - text.
    - transforms.
    - image sources.
    - compositing.
    - tiling.
    - multi-threading.
    - large path rasterizer.
    - canvas api.
    - oom api.
        - ipgui needs that to some extent (where dom nodes own allocations,
          not really for the renderer, we're sol if that panics due to oom).


