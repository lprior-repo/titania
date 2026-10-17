pub(super) const fn is_khi_byte(byte: u8) -> bool {
    (byte >= b'A' && byte <= b'Z')
        || (byte >= b'a' && byte <= b'z')
        || (byte >= b'0' && byte <= b'9')
        || byte == b'_'
}
