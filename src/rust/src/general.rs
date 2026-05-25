/// C ABI: Return the next power of two for `n`.
/// If `n` is already a power of two, returns `n`.
/// For `n == 0`, returns `0` to match original C++ behavior.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_uppow2(n: u32) -> u32 {
    if n == 0 {
        0
    } else {
        n.next_power_of_two()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uppow2() {
        assert_eq!(fceux11_rust_uppow2(0), 0);
        assert_eq!(fceux11_rust_uppow2(1), 1);
        assert_eq!(fceux11_rust_uppow2(2), 2);
        assert_eq!(fceux11_rust_uppow2(3), 4);
        assert_eq!(fceux11_rust_uppow2(4), 4);
        assert_eq!(fceux11_rust_uppow2(5), 8);
        assert_eq!(fceux11_rust_uppow2(15), 16);
        assert_eq!(fceux11_rust_uppow2(16), 16);
        assert_eq!(fceux11_rust_uppow2(17), 32);
        assert_eq!(fceux11_rust_uppow2(0x8000_0000), 0x8000_0000);
    }
}
