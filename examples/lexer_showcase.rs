fn main() -> i32 {
    // Lexer showcase for keywords, operators, comments, and literals.
    let mut x: i32 = 40 + 2;
    let ok: bool = true && !false;
    let letter: char = 'r';
    let text: str = "rust";

    /*
       Block comments may contain nested comments in this teaching lexer.
       /* nested */
    */
    if ok && x >= 42 {
        return x;
    } else {
        while x < 42 {
            x = x + 1;
        }
    }

    return 0;
}

