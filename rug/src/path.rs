use sti::alloc::*;
use sti::vec::Vec;
use sti::simd::*;

use core::ptr::NonNull;
use core::marker::PhantomData;
use core::mem::{size_of, ManuallyDrop};
use core::sync::atomic::{AtomicU32, Ordering};

use crate::geometry::*;


/// Path syntax:
///  Path    ::= SubPath*
///  SubPath ::= (BeginOpen | BeginClosed) Curve* End
///  Curve   ::= Line | Quad | Cubic
/// 
/// Number of points:
///  Begin*: 1
///  Line:   1
///  Quad:   2
///  Cubic:  3
///  End:    0
/// 
/// The first point of any curve is the last point of the previous verb.
/// Closed paths must have *equal* start/end points.
/// 
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verb {
    BeginOpen,
    BeginClosed,
    Line,
    Quad,
    Cubic,
    End,
}



pub struct PathBuilder<A: Alloc = GlobalAlloc> {
    verbs:  Vec<Verb,  A>,
    points: Vec<F32x2, A>,
    aabb:        Rect,
    in_path:     bool,
    begin_point: F32x2,
    begin_verb:  usize,
}

impl<A: Alloc> PathBuilder<A> {
    pub fn new_in(alloc: A) -> Self  where A: Clone{
        PathBuilder {
            verbs:       Vec::new_in(alloc.clone()),
            points:      Vec::new_in(alloc),
            aabb:        Rect::MAX_MIN,
            in_path:     false,
            begin_point: F32x2::ZERO,
            begin_verb:  usize::MAX,
        }
    }
}

impl PathBuilder<GlobalAlloc> {
    pub fn new() -> Self {
        PathBuilder::new_in(GlobalAlloc)
    }
}

impl<A: Alloc> PathBuilder<A> {
    pub fn in_path(&self) -> bool {
        self.in_path
    }

    pub fn last_point(&self) -> F32x2 {
        assert!(self.in_path);
        *self.points.last().unwrap()
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

    pub fn line_to(&mut self, p1: F32x2) {
        assert!(self.in_path);
        self.verbs.push(Verb::Line);
        self.points.push(p1);
        self.aabb.include(p1);
    }

    pub fn quad_to(&mut self, p1: F32x2, p2: F32x2) {
        assert!(self.in_path);
        self.verbs.push(Verb::Quad);
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

    pub fn close_path(&mut self) {
        assert!(self.in_path);
        // ensure start/end points are equal.
        if *self.points.last().unwrap() != self.begin_point {
            self.line_to(self.begin_point);
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


    pub fn clear(&mut self) {
        self.verbs.clear();
        self.points.clear();
        self.aabb        = Rect::MAX_MIN;
        self.in_path     = false;
        self.begin_point = F32x2::ZERO;
        self.begin_verb  = usize::MAX;
    }

    pub fn build_in<B: Alloc>(&mut self, alloc: B) -> PathBuf<B> {
        if self.in_path {
            self._end_path();
        }

        // ensure aabb is valid.
        let aabb =
            if self.verbs.len() > 0 { self.aabb }
            else { Rect::ZERO };

        // verbs/points are valid by construction.
        unsafe { PathBuf::new_in(&self.verbs, &self.points, aabb, alloc) }
    }

    pub fn build(&mut self) -> PathBuf<GlobalAlloc> {
        self.build_in(GlobalAlloc)
    }
}


/// Path memory layout:
/// ```rust
///     header: PathHeader
///     verbs:  [Verb; header.verb_count]
///     points: [F32x2; header.point_count]
/// ```
pub struct PathBuf<A: Alloc = GlobalAlloc> {
    data: NonNull<PathData>,
    alloc: A,
}

impl<A: Alloc> PathBuf<A> {
    #[inline(always)]
    pub unsafe fn new_in(verbs: &[Verb], points: &[F32x2], aabb: Rect, alloc: A) -> Self {
        let num_verbs  = verbs.len().try_into().unwrap();  // @temp
        let num_points = points.len().try_into().unwrap(); // @temp

        let layout = PathData::layout(verbs.len(), points.len()).unwrap(); // @temp
        let data: NonNull<PathData> = alloc.alloc(layout).unwrap().cast(); // @temp

        unsafe {
            data.as_ptr().write(PathData {
                refs: AtomicU32::new(1),
                aabb,
                num_verbs, num_points,
            });

            let vp: *mut Verb  = cat_next_mut(data.as_ptr(), size_of::<PathData>());
            let pp: *mut F32x2 = cat_next_mut(vp, verbs.len()*size_of::<Verb>());

            let end: *mut u8 = cat_next_mut(pp, points.len()*size_of::<F32x2>());
            debug_assert_eq!(end as usize - data.as_ptr() as usize, layout.size());

            core::ptr::copy(verbs.as_ptr(),  vp, verbs.len());
            core::ptr::copy(points.as_ptr(), pp, points.len());
        }

        Self { data, alloc }
    }


    #[inline(always)]
    fn data(&self) -> &PathData { unsafe { self.data.as_ref() } }

    #[inline(always)]
    pub fn path(&self) -> Path {
        Path { data: self.data, phantom: PhantomData }
    }


    #[inline(always)]
    pub fn leak<'a>(self) -> Path<'a>  where A: 'a {
        let mut this = ManuallyDrop::new(self);
        unsafe { core::ptr::drop_in_place(&mut this.alloc) }
        Path { data: this.data, phantom: PhantomData }
    }
}

impl<A: Alloc + Clone> Clone for PathBuf<A> {
    fn clone(&self) -> Self {
        let old_refs = self.data().refs.fetch_add(1, Ordering::Relaxed);
        debug_assert!(old_refs > 0);

        Self {
            data: self.data,
            alloc: self.alloc.clone(),
        }
    }
}

impl<A: Alloc> Drop for PathBuf<A> {
    fn drop(&mut self) {
        unsafe {
            let old_refs = self.data().refs.fetch_sub(1, Ordering::Relaxed);
            debug_assert!(old_refs > 0);

            if old_refs == 1 {
                let layout = PathData::layout(
                    self.data().num_verbs  as usize,
                    self.data().num_points as usize).unwrap();
                self.alloc.free(self.data.cast(), layout);
            }
        }
    }
}


struct PathData {
    refs: AtomicU32,
    aabb: Rect,
    num_verbs:  u32,
    num_points: u32,
}

impl PathData {
    #[inline(always)]
    fn layout(verb_count: usize, point_count: usize) -> Option<Layout> {
        cat_join(
            cat_join(
                Layout::new::<PathData>(),
                Layout::array::<Verb>(verb_count).ok()?)?,
            Layout::array::<F32x2>(point_count).ok()?)
    }
}


#[derive(Clone, Copy)]
pub struct Path<'a> {
    data: NonNull<PathData>,
    phantom: PhantomData<&'a ()>,
}

impl<'a> Path<'a> {
    #[inline(always)]
    fn data(&self) -> &PathData { unsafe { self.data.as_ref() } }

    #[inline(always)]
    pub fn verbs(&self) -> &[Verb] { unsafe {
        let ptr: *const Verb = cat_next(self.data.as_ptr(), size_of::<PathData>());
        core::slice::from_raw_parts(ptr, self.data().num_verbs as usize)
    }}

    #[inline(always)]
    pub fn points(&self) -> &[F32x2] { unsafe {
        let verbs: *const Verb  = cat_next(self.data.as_ptr(), size_of::<PathData>());
        let ptr:   *const F32x2 = cat_next(verbs, self.data().num_verbs as usize * size_of::<Verb>());
        core::slice::from_raw_parts(ptr, self.data().num_points as usize)
    }}

    #[inline(always)]
    pub fn aabb(&self) -> Rect {
        self.data().aabb
    }


    #[inline(always)]
    pub fn iter(&self) -> Iter { Iter::new(self) }
}

impl<'a> core::fmt::Debug for Path<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "Path(")?;
        f.debug_list().entries(self.iter()).finish()?;
        write!(f, ")")
    }
}


#[derive(Debug)]
pub enum IterEvent {
    Begin (F32x2, bool), // first-point, closed
    Line  (Line),
    Quad  (Quad),
    Cubic (Cubic),
    End   (F32x2), // last-point
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
    pub fn new(path: &'p Path) -> Self {
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

            Verb::Line => {
                let p0 = self.p0;
                let p1 = self.points[self.point];
                self.point += 1;
                self.p0 = p1;
                IterEvent::Line(line(p0, p1))
            },

            Verb::Quad => {
                let p0 = self.p0;
                let p1 = self.points[self.point + 0];
                let p2 = self.points[self.point + 1];
                self.point += 2;
                self.p0 = p2;
                IterEvent::Quad(quad(p0, p1, p2))
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


