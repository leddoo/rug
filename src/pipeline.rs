use crate::wide::*;
use crate::image::*;


pub struct Pipeline<'i> {
    target: ImgMut<'i>,
}


impl<'i> Pipeline<'i> {
    pub fn new(target: ImgMut<'i>) -> Pipeline<'i> {
        Pipeline {
            target
        }
    }

    pub fn fill_mask(&mut self, offset: U32x2, mask: Img) {
        let bounds = self.target.bounds();

        let _min = offset.min(bounds);
        let _max = (offset + mask.bounds()).min(bounds);

        unimplemented!()
    }
}

