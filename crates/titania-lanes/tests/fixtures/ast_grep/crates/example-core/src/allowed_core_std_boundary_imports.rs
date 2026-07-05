use std::fs_extra;
use std::time::Instantaneous;

pub fn keep_boundary_names(extra: fs_extra::Marker, instant: Instantaneous) -> (fs_extra::Marker, Instantaneous) {
    (extra, instant)
}

mod fs_extra {
    pub struct Marker;
}

pub struct Instantaneous;
