#![cfg_attr(target_family = "kidneyos", no_std)]
#![cfg_attr(target_family = "kidneyos", no_main)]

#[cfg(target_family = "kidneyos")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

fn main() {
    loop {}
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
