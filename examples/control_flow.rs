fn main() -> i32 {
    let mut x: i32 = 0;

    while x < 3 {
        x = x + 1;
    }

    if x == 3 {
        return 42;
    } else {
        return 1;
    }
}

