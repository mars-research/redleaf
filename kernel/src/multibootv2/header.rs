use core::marker::PhantomData;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Tag {
    pub typ: u32,
    pub size: u32,
    // tag specific fields
}

#[derive(Clone, Debug)]
pub struct TagIter<'a> {
    pub current: *const Tag,
    phantom: PhantomData<&'a Tag>,
}

impl<'a> TagIter<'a> {
    pub fn new(first: *const Tag) -> Self {
        TagIter {
            current: first,
            phantom: PhantomData,
        }
    }
}

impl<'a> Iterator for TagIter<'a> {
    type Item = &'a Tag;

    fn next(&mut self) -> Option<&'a Tag> {
        match unsafe { &*self.current } {
            &Tag { typ: 0, size: 8 } => None, // end tag
            tag => {
                // go to next tag
                let mut tag_addr = self.current as usize;
                tag_addr += ((tag.size + 7) & !7) as usize; //align at 8 byte
                self.current = tag_addr as *const _;

                Some(tag)
            }
        }
    }
}
