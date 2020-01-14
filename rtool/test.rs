// GIVEN

#[derive(Copy)]
struct String {
    f1: u64;
    f2: RRef<[u8]>;
}

#[derive(Copy)]
struct Type1 {}

trait DomainAInterface {
    fn read(&self, name: RRef<String>, bytes: usize) -> RRef<[u8]>;
    fn write(&self, name: RRef<String>, bytes: RRef<[u8]>) -> usize;
    fn test3(&self, arg1: RRef<Type1>, arg2: RRef<Type1>);
}

fn perform_something_on_thing(object: RRef<dyn Thing>) -> u64;

// END GIVEN