mod foo {
    pub mod std {
        pub mod fs {
            pub fn read() {}
        }
    }
}

pub fn load() {
    foo::std::fs::read();
}
