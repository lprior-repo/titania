
pub fn absolute_head() -> ::std::result::Result<u8, String> {
    Ok(1)
}

pub fn alloc_error() -> Result<u8, alloc::string::String> {
    Ok(2)
}

pub fn mixed() -> std::result::Result<u8, alloc::string::String> {
    Ok(3)
}

pub fn absolute_error() -> Result<u8, ::std::string::String> {
    Ok(4)
}
