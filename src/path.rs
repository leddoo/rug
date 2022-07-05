use core::ptr::NonNull;
use core::sync::atomic::{AtomicU32, Ordering};

use sti::simd::*;
use crate::alloc::*;
use crate::geometry::*;


/// Path syntax:
///  Path ::= SubPath*
///  SubPath ::= (BeginOpen | BeginClosed) Curve* End
///  Curve ::= Segment | Quadratic | Cubic
/// 
/// Number of points:
///  Begin*:    1
///  Segment:   1
///  Quadratic: 2
///  Cubic:     3
///  End:       0
/// 
/// The first point of any curve is the last point of the previous verb.
/// Closed paths must have *equal* start/end points.
/// 

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verb {
    BeginOpen,
    BeginClosed,
    Segment,
    Quadratic,
    Cubic,
    End,
}


pub struct Path<A: Alloc> {
    header: NonNull<PathHeader<A>>,
}

// safety:
// Path itself is thread safe (immutable, arc).
// the allocator may not be.
// only path creation and dropping can access the allocator.
// so sending is only safe when sending the allocator is safe,
// but sending references is always safe.
unsafe impl<A: Alloc> Send for Path<A> where A: Send {}
unsafe impl<A: Alloc> Sync for Path<A> {}


impl Path<GlobalAlloc> {
    /// safety:
    /// - verbs/points must have the path syntax (see above).
    /// - aabb must be valid (min <= max) and include all points.
    pub unsafe fn new(verbs: &[Verb], points: &[F32x2], aabb: Rect) -> Self {
        Path::new_in(verbs, points, aabb, GlobalAlloc)
    }
}


pub struct PathHeader<A: Alloc> {
    alloc: A,
    references: AtomicU32,
    verb_count: u32,
    point_count: u32,
    aabb: Rect,
}


pub enum IterEvent {
    Begin     (F32x2, bool), // first-point, closed
    Segment   (Segment),
    Quadratic (Quadratic),
    Cubic     (Cubic),
    End       (F32x2), // last-point
}

pub struct Iter<'p> {
    verbs:  &'p [Verb],
    points: &'p [F32x2],
    verb:  usize,
    point: usize,
    p0:    F32x2,
}

impl<'p> Iter<'p> {
    #[inline(always)]
    pub fn new<A: Alloc>(path: &'p Path<A>) -> Self {
        Iter {
            verbs:  path.verbs(),
            points: path.points(),
            verb:  0,
            point: 0,
            p0:    F32x2::ZERO,
        }
    }

    pub fn has_next(&self) -> bool {
        self.verb < self.verbs.len()
    }

    pub fn next(&mut self) -> Option<IterEvent> {
        if !self.has_next() {
            debug_assert!(self.verb  == self.verbs.len());
            debug_assert!(self.point == self.points.len());
            return None;
        }

        let verb = self.verbs[self.verb];
        let result = match verb {
            Verb::BeginOpen | Verb::BeginClosed => {
                let p0 = self.points[self.point];
                self.point += 1;
                self.p0 = p0;
                IterEvent::Begin(p0, verb == Verb::BeginClosed)
            },

            Verb::Segment => {
                let p0 = self.p0;
                let p1 = self.points[self.point];
                self.point += 1;
                self.p0 = p1;
                IterEvent::Segment(segment(p0, p1))
            },

            Verb::Quadratic => {
                let p0 = self.p0;
                let p1 = self.points[self.point + 0];
                let p2 = self.points[self.point + 1];
                self.point += 2;
                self.p0 = p2;
                IterEvent::Quadratic(quadratic(p0, p1, p2))
            },

            Verb::Cubic => {
                let p0 = self.p0;
                let p1 = self.points[self.point + 0];
                let p2 = self.points[self.point + 1];
                let p3 = self.points[self.point + 2];
                self.point += 3;
                self.p0 = p3;
                IterEvent::Cubic(cubic(p0, p1, p2, p3))
            },

            Verb::End => IterEvent::End(self.p0),
        };
        self.verb += 1;

        Some(result)
    }
}

impl<'p> Iterator for Iter<'p> {
    type Item = IterEvent;

    #[inline(always)]
    fn next(&mut self) -> Option<IterEvent> {
        self.next()
    }
}


pub struct PathBuilder<A: CopyAlloc> {
    verbs:  Vec<Verb, A>,
    points: Vec<F32x2, A>,
    aabb:   Rect,
    in_path:     bool,
    begin_point: F32x2,
    begin_verb:  usize,
}

impl PathBuilder<GlobalAlloc> {
    pub fn new() -> Self {
        PathBuilder::new_in(GlobalAlloc)
    }
}

impl<A: CopyAlloc> PathBuilder<A> {
    pub fn new_in(alloc: A) -> Self {
        PathBuilder {
            verbs:       Vec::new_in(alloc),
            points:      Vec::new_in(alloc),
            aabb:        Rect::MAX_MIN,
            in_path:     false,
            begin_point: F32x2::ZERO,
            begin_verb:  usize::MAX,
        }
    }

    pub fn in_path(&self) -> bool {
        self.in_path
    }

    pub fn last_point(&self) -> F32x2 {
        assert!(self.in_path);
        *self.points.last().unwrap()
    }

    pub fn clear(&mut self) {
        self.verbs.clear();
        self.points.clear();
        self.aabb        = Rect::MAX_MIN;
        self.in_path     = false;
        self.begin_point = F32x2::ZERO;
        self.begin_verb  = usize::MAX;
    }


    pub fn build(&mut self) -> Path<GlobalAlloc> {
        self.build_in(GlobalAlloc)
    }

    pub fn build_in<B: Alloc>(&mut self, alloc: B) -> Path<B> {
        if self.in_path {
            self._end_path();
        }

        // ensure aabb is valid.
        let aabb =
            if self.verbs.len() > 0 { self.aabb }
            else { Rect::ZERO };

        // verbs/points are valid by construction.
        unsafe { Path::new_in(&self.verbs, &self.points, aabb, alloc) }
    }

    pub fn move_to(&mut self, p0: F32x2) {
        if self.in_path {
            self._end_path();
        }

        self.verbs.push(Verb::BeginOpen);
        self.points.push(p0);
        self.aabb.include(p0);
        self.in_path     = true;
        self.begin_point = p0;
        self.begin_verb  = self.verbs.len() - 1;
    }

    pub fn segment_to(&mut self, p1: F32x2) {
        assert!(self.in_path);
        self.verbs.push(Verb::Segment);
        self.points.push(p1);
        self.aabb.include(p1);
    }

    pub fn quadratic_to(&mut self, p1: F32x2, p2: F32x2) {
        assert!(self.in_path);
        self.verbs.push(Verb::Quadratic);
        self.points.push(p1);
        self.points.push(p2);
        self.aabb.include(p1);
        self.aabb.include(p2);
    }

    pub fn cubic_to(&mut self, p1: F32x2, p2: F32x2, p3: F32x2) {
        assert!(self.in_path);
        self.verbs.push(Verb::Cubic);
        self.points.push(p1);
        self.points.push(p2);
        self.points.push(p3);
        self.aabb.include(p1);
        self.aabb.include(p2);
        self.aabb.include(p3);
    }

    pub fn close(&mut self) {
        assert!(self.in_path);
        // ensure start/end points are equal.
        if *self.points.last().unwrap() != self.begin_point {
            self.segment_to(self.begin_point);
        }

        self.verbs[self.begin_verb] = Verb::BeginClosed;
        self._end_path();
    }

    #[inline(always)]
    fn _end_path(&mut self) {
        self.verbs.push(Verb::End);
        self.in_path    = false;
        self.begin_verb = usize::MAX;
    }
}


#[derive(Clone)]
pub struct SoaPath<'a> {
    pub lines:  Box<[Segment],   &'a dyn Alloc>,
    pub quads:  Box<[Quadratic], &'a dyn Alloc>,
    pub cubics: Box<[Cubic],     &'a dyn Alloc>,
    pub aabb:   Rect,
}

impl<'a> SoaPath<'a> {
    pub fn transform(&mut self, tfx: Transform) {
        let mut aabb = Rect::MAX_MIN;

        for line in self.lines.iter_mut() {
            *line = tfx * (*line);
            aabb.include(line.p0);
            aabb.include(line.p1);
        }

        for quad in self.quads.iter_mut() {
            *quad = tfx * (*quad);
            aabb.include(quad.p0);
            aabb.include(quad.p1);
            aabb.include(quad.p2);
        }

        for cubic in self.cubics.iter_mut() {
            *cubic = tfx * (*cubic);
            aabb.include(cubic.p0);
            aabb.include(cubic.p1);
            aabb.include(cubic.p2);
            aabb.include(cubic.p3);
        }

        self.aabb = aabb;
    }
}



/// Path memory layout:
///     header: PathHeader
///     verbs:  [Verb; header.verb_count]
///     points: [F32x2; header.point_count]

impl<A: Alloc> Path<A> {
    // see Path::new for safety information.
    pub unsafe fn new_in(verbs: &[Verb], points: &[F32x2], aabb: Rect, alloc: A) -> Self {
        let verb_count  = verbs.len().try_into().unwrap();
        let point_count = points.len().try_into().unwrap();

        let layout = <PathHeader<A>>::allocation_layout(verbs.len(), points.len());
        let data = alloc.allocate(layout).unwrap();

        // header
        let header = data.as_ptr() as *mut PathHeader<A>;
        header.write(PathHeader {
            alloc,
            references: AtomicU32::new(1),
            verb_count, point_count, aabb,
        });

        // verbs
        let vs =
            core::slice::from_raw_parts_mut(
                PathHeader::verbs_begin(header) as *mut Verb,
                verb_count as usize);
        vs.copy_from_slice(verbs);

        // points
        let ps =
            core::slice::from_raw_parts_mut(
                PathHeader::points_begin(header, verb_count as usize) as *mut F32x2,
                point_count as usize);
        ps.copy_from_slice(points);

        Self { header: NonNull::new(header).unwrap() }
    }


    #[inline(always)]
    pub fn iter(&self) -> Iter {
        Iter::new(self)
    }

    #[inline(always)]
    pub fn aabb(&self) -> Rect {
        unsafe { (&*self.header.as_ptr()).aabb }
    }

    #[inline(always)]
    pub fn verbs(&self) -> &[Verb] {
        unsafe { (&*self.header.as_ptr()).verbs() }
    }

    #[inline(always)]
    pub fn points(&self) -> &[F32x2] {
        unsafe { (&*self.header.as_ptr()).points() }
    }
}

impl<A: Alloc> Clone for Path<A> {
    fn clone(&self) -> Self {
        let header = unsafe { &*self.header.as_ptr() };
        let old_refs = header.references.fetch_add(1, Ordering::Relaxed);
        assert!(old_refs > 0 && old_refs < u32::MAX);

        Self { header: self.header }
    }
}

impl<A: Alloc> Drop for Path<A> {
    fn drop(&mut self) {

        let header = unsafe { &*self.header.as_ptr() };
        let old_refs = header.references.fetch_sub(1, Ordering::Relaxed);
        assert!(old_refs > 0);

        if old_refs == 1 {
            let header = unsafe { self.header.as_ptr().read() };
            let alloc = &header.alloc;

            let data = NonNull::new(self.header.as_ptr() as *mut u8).unwrap();
            let layout = <PathHeader<A>>::allocation_layout(header.verb_count as usize, header.point_count as usize);
            unsafe { alloc.deallocate(data, layout) };

            drop(header);
        }
    }
}


impl<A: Alloc> PathHeader<A> {
    #[inline(always)]
    fn verbs_begin(header: *const Self) -> *const Verb {
        let header_end = unsafe { header.add(1) };
        align_pointer(header_end as *const Verb)
    }

    #[inline(always)]
    fn points_begin(header: *const Self, verb_count: usize) -> *const F32x2 {
        let verbs_begin = Self::verbs_begin(header);
        let verbs_end = unsafe { verbs_begin.add(verb_count) };
        align_pointer(verbs_end as *const F32x2)
    }

    #[inline(always)]
    fn allocation_alignment() -> usize {
        use core::mem::align_of;
        align_of::<Self>()
        .max(align_of::<Verb>())
        .max(align_of::<F32x2>())
    }

    #[inline(always)]
    fn allocation_size(verb_count: usize, point_count: usize) -> usize {
        let points_begin = Self::points_begin(0 as *const Self, verb_count);
        let points_end = unsafe { points_begin.add(point_count) };
        align_pointer_to(points_end, Self::allocation_alignment()) as usize
    }

    #[inline(always)]
    fn allocation_layout(verb_count: usize, point_count: usize) -> AllocLayout {
        AllocLayout::from_size_align(
            Self::allocation_size(verb_count, point_count),
            Self::allocation_alignment()
        ).unwrap()
    }


    #[inline(always)]
    pub fn verbs(&self) -> &[Verb] {
        unsafe {
            core::slice::from_raw_parts(
                Self::verbs_begin(self),
                self.verb_count as usize)
        }
    }

    #[inline(always)]
    pub fn points(&self) -> &[F32x2] {
        unsafe {
            core::slice::from_raw_parts(
                Self::points_begin(self, self.verb_count as usize),
                self.point_count as usize)
        }
    }
}

