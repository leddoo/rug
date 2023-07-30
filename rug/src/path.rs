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
///  SubPath ::= (BeginOpen | BeginClosed) Curve* (EndOpen | EndClosed)
///  Curve   ::= Line | Quad | Cubic
/// 
/// Number of points:
///  Begin*: 1
///  Line:   1
///  Quad:   2
///  Cubic:  3
///  End*:   0
/// 
/// The first point of any curve is the last point of the previous verb.
/// Closed paths must have *equal* start/end points.
/// Begin/End Open/Closed must match for each sub path.
/// 
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verb {
    BeginOpen,
    BeginClosed,
    Line,
    Quad,
    Cubic,
    EndOpen,
    EndClosed,
}



pub struct PathBuilder<A: Alloc = GlobalAlloc> {
    verbs:  Vec<Verb,  A>,
    points: Vec<F32x2, A>,
    aabb:        Rect,
    in_path:     bool,
    begin_point: F32x2,
    begin_verb:  usize,
}

impl PathBuilder<GlobalAlloc> {
    pub fn new() -> Self {
        PathBuilder::new_in(GlobalAlloc)
    }
}

impl<A: Alloc> PathBuilder<A> {
    pub fn new_in(alloc: A) -> Self  where A: Clone {
        PathBuilder {
            verbs:       Vec::new_in(alloc.clone()),
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


    pub fn move_to(&mut self, p0: F32x2) {
        if self.in_path {
            self._end_path(Verb::EndOpen);
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
        self._end_path(Verb::EndClosed);
    }

    #[inline(always)]
    fn _end_path(&mut self, verb: Verb) {
        self.verbs.push(verb);
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
            self._end_path(Verb::EndOpen);
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



pub struct RawPathBuilder<A: Alloc = GlobalAlloc> {
    pub verbs:  Vec<Verb,  A>,
    pub points: Vec<F32x2, A>,
}

impl RawPathBuilder<GlobalAlloc> {
    pub fn new() -> Self {
        RawPathBuilder::new_in(GlobalAlloc)
    }
}

impl<A: Alloc> RawPathBuilder<A> {
    pub fn new_in(alloc: A) -> Self  where A: Clone {
        RawPathBuilder {
            verbs:  Vec::new_in(alloc.clone()),
            points: Vec::new_in(alloc),
        }
    }

    pub fn clear(&mut self) {
        self.verbs.clear();
        self.points.clear();
    }
}



/// Path memory layout:
///  header: PathHeader
///  verbs:  [Verb; header.verb_count]
///  points: [F32x2; header.point_count]
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


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IterEvent {
    /// first point, closed.
    Begin (F32x2, bool),
    Line  (Line),
    Quad  (Quad),
    Cubic (Cubic),
    /// last point, closed.
    End   (F32x2, bool),
}

#[derive(Clone)]
pub struct Iter<'p> {
    verbs:  &'p [Verb],
    points: &'p [F32x2],
    verb:  usize,
    point: usize,
}

impl<'p> Iter<'p> {
    #[inline(always)]
    pub fn new(path: &'p Path) -> Self {
        Iter {
            verbs:  path.verbs(),
            points: path.points(),
            verb:  0,
            point: 0,
        }
    }


    #[cfg(test)]
    fn test_equal(&self, other: &Self) -> bool {
        println!("{} {} =?= {} {}",
            self.verb,  self.point,
            other.verb, other.point);
           self.verb  == other.verb
        && self.point == other.point
    }


    #[inline(always)]
    pub fn has_next(&self) -> bool {
        self.verb < self.verbs.len()
    }

    pub fn next(&mut self) -> Option<IterEvent> {
        if !self.has_next() {
            debug_assert_eq!(self.verb,  self.verbs.len());
            debug_assert_eq!(self.point, self.points.len());
            return None;
        }

        let verb = self.verbs[self.verb];
        self.verb += 1;

        Some(match verb {
            Verb::BeginOpen | Verb::BeginClosed => {
                let p0 = self.points[self.point];
                IterEvent::Begin(p0, verb == Verb::BeginClosed)
            },

            Verb::Line => {
                assert!(self.point + 1 < self.points.len());
                let p0 = self.points[self.point + 0];
                let p1 = self.points[self.point + 1];
                self.point += 1;
                IterEvent::Line(line(p0, p1))
            },

            Verb::Quad => {
                assert!(self.point + 2 < self.points.len());
                let p0 = self.points[self.point + 0];
                let p1 = self.points[self.point + 1];
                let p2 = self.points[self.point + 2];
                self.point += 2;
                IterEvent::Quad(quad(p0, p1, p2))
            },

            Verb::Cubic => {
                assert!(self.point + 3 < self.points.len());
                let p0 = self.points[self.point + 0];
                let p1 = self.points[self.point + 1];
                let p2 = self.points[self.point + 2];
                let p3 = self.points[self.point + 3];
                self.point += 3;
                IterEvent::Cubic(cubic(p0, p1, p2, p3))
            },

            Verb::EndOpen | Verb::EndClosed => {
                let p0 = self.points[self.point];
                self.point += 1;
                IterEvent::End(p0, verb == Verb::EndClosed)
            }
        })
    }


    #[inline(always)]
    pub fn has_prev(&self) -> bool {
        self.verb > 0
    }

    pub fn prev_rev(&mut self) -> Option<IterEvent> {
        if !self.has_prev() {
            debug_assert_eq!(self.verb,  0);
            debug_assert_eq!(self.point, 0);
            return None;
        }

        let verb = self.verbs[self.verb - 1];
        self.verb -= 1;

        Some(match verb {
            Verb::BeginOpen | Verb::BeginClosed => {
                let p0 = self.points[self.point];
                IterEvent::End(p0, verb == Verb::BeginClosed)
            },

            Verb::Line => {
                self.point -= 1;
                let p0 = self.points[self.point + 1];
                let p1 = self.points[self.point + 0];
                IterEvent::Line(line(p0, p1))
            },

            Verb::Quad => {
                self.point -= 2;
                let p0 = self.points[self.point + 2];
                let p1 = self.points[self.point + 1];
                let p2 = self.points[self.point + 0];
                IterEvent::Quad(quad(p0, p1, p2))
            },

            Verb::Cubic => {
                self.point -= 3;
                let p0 = self.points[self.point + 3];
                let p1 = self.points[self.point + 2];
                let p2 = self.points[self.point + 1];
                let p3 = self.points[self.point + 0];
                IterEvent::Cubic(cubic(p0, p1, p2, p3))
            },

            Verb::EndOpen | Verb::EndClosed => {
                self.point -= 1;
                let p0 = self.points[self.point];
                IterEvent::Begin(p0, verb == Verb::EndClosed)
            }
        })
    }
}

impl<'p> Iterator for Iter<'p> {
    type Item = IterEvent;

    #[inline(always)]
    fn next(&mut self) -> Option<IterEvent> {
        self.next()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_iterator() {
        let path = {
            let mut pb = PathBuilder::new();
            pb.move_to([1.0, 1.0].into());
            pb.line_to([2.0, 2.0].into());
            pb.quad_to([3.0, 3.0].into(), [4.0, 4.0].into());
            pb.cubic_to([5.0, 5.0].into(), [6.0, 6.0].into(), [7.0, 7.0].into());
            pb.close_path();

            pb.move_to([8.0, 8.0].into());
            pb.line_to([9.0, 9.0].into());

            pb.build()
        };

        let path = path.path();

        let mut iter = path.iter();

        assert_eq!(iter.clone().prev_rev(), None);

        {
            let i0 = iter.clone();
            assert_eq!(iter.next(), Some(IterEvent::Begin([1.0, 1.0].into(), true)));

            let mut i2 = iter.clone();
            assert_eq!(i2.prev_rev(), Some(IterEvent::End([1.0, 1.0].into())));

            assert!(i0.test_equal(&i2));
        }

        {
            let i0 = iter.clone();
            assert_eq!(iter.next(), Some(IterEvent::Line(line([1.0, 1.0].into(), [2.0, 2.0].into()))));

            let mut i2 = iter.clone();
            assert_eq!(i2.prev_rev(), Some(IterEvent::Line(line([2.0, 2.0].into(), [1.0, 1.0].into()))));

            assert!(i0.test_equal(&i2));
        }

        {
            let i0 = iter.clone();
            assert_eq!(iter.next(), Some(IterEvent::Quad(quad([2.0, 2.0].into(), [3.0, 3.0].into(), [4.0, 4.0].into()))));

            let mut i2 = iter.clone();
            assert_eq!(i2.prev_rev(), Some(IterEvent::Quad(quad([4.0, 4.0].into(), [3.0, 3.0].into(), [2.0, 2.0].into()))));

            assert!(i0.test_equal(&i2));
        }

        {
            let i0 = iter.clone();
            assert_eq!(iter.next(), Some(IterEvent::Cubic(cubic([4.0, 4.0].into(), [5.0, 5.0].into(), [6.0, 6.0].into(), [7.0, 7.0].into()))));

            let mut i2 = iter.clone();
            assert_eq!(i2.prev_rev(), Some(IterEvent::Cubic(cubic([7.0, 7.0].into(), [6.0, 6.0].into(), [5.0, 5.0].into(), [4.0, 4.0].into()))));

            assert!(i0.test_equal(&i2));
        }

        {
            let i0 = iter.clone();
            assert_eq!(iter.next(), Some(IterEvent::Line(line([7.0, 7.0].into(), [1.0, 1.0].into()))));

            let mut i2 = iter.clone();
            assert_eq!(i2.prev_rev(), Some(IterEvent::Line(line([1.0, 1.0].into(), [7.0, 7.0].into()))));

            assert!(i0.test_equal(&i2));
        }

        {
            let i0 = iter.clone();
            assert_eq!(iter.next(), Some(IterEvent::End([1.0, 1.0].into())));

            let mut i2 = iter.clone();
            assert_eq!(i2.prev_rev(), Some(IterEvent::Begin([1.0, 1.0].into(), true)));

            assert!(i0.test_equal(&i2));
        }

        {
            let i0 = iter.clone();
            assert_eq!(iter.next(), Some(IterEvent::Begin([8.0, 8.0].into(), false)));

            let mut i2 = iter.clone();
            assert_eq!(i2.prev_rev(), Some(IterEvent::End([8.0, 8.0].into())));

            assert!(i0.test_equal(&i2));
        }

        {
            let i0 = iter.clone();
            assert_eq!(iter.next(), Some(IterEvent::Line(line([8.0, 8.0].into(), [9.0, 9.0].into()))));

            let mut i2 = iter.clone();
            assert_eq!(i2.prev_rev(), Some(IterEvent::Line(line([9.0, 9.0].into(), [8.0, 8.0].into()))));

            assert!(i0.test_equal(&i2));
        }

        {
            let i0 = iter.clone();
            assert_eq!(iter.next(), Some(IterEvent::End([9.0, 9.0].into())));

            let mut i2 = iter.clone();
            assert_eq!(i2.prev_rev(), Some(IterEvent::Begin([9.0, 9.0].into(), false)));

            assert!(i0.test_equal(&i2));
        }

        assert_eq!(iter.next(), None);
    }
}

