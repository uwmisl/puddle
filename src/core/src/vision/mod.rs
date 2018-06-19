
extern "C" {
    fn hello_world();
}

pub fn hello_world_wrapper() {
    unsafe {
        hello_world();
    }
}
