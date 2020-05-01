use alloc::sync::Arc;
use core::ops::Deref;

use tls::ThreadLocal;

use crate::icache::{INode, ICache};

lazy_static! {
    pub static ref CWD: Cwd = Cwd::new();
}

pub struct Cwd(ThreadLocal<Arc<INode>>);

impl Cwd {
    fn new() -> Self {
        Self(ThreadLocal::new(|| ICache::namei(&mut crate::log::LOG.r#try().unwrap().begin_transaction(), "/").unwrap()))
    }
}

impl Deref for Cwd {
    type Target = ThreadLocal<Arc<INode>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
