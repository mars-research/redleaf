#[cfg(test)] 
mod tests {
    #[test]
    fn test_bget() {
        let mut bcache = bcache::BCACHE.lock();
        let mut guard = bcache.get(0, 0);
        guard.drop();
    }
}