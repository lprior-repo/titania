mod sync {
    pub struct Mutex;

    impl Mutex {
        pub fn new(_value: i32) -> Self {
            Self
        }
    }
}

pub fn counter() {
    let _m = sync::Mutex::new(0);
}
