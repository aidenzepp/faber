//! `faber format --stdout` keeps the `#` comment above every declaration kind
//! (faberlang/faber#1). Requires the built `faber` binary.
//!
//! The source is already canonical, so the formatter must return it byte for
//! byte: a dropped comment and any other layout change both fail. Comments on
//! `@ shared` union fields are not covered here; shared fields carry no
//! `DefId` for the trivia index to key them by.

use std::fs;
use std::process::Command;

#[path = "support/temp.rs"]
mod temp;
use temp::TempDir;

const SOURCE: &str = "\
# function comment
@ public
fn score() → int {
    return 1
}

# class comment
@ public
class Board {
    # class field comment
    mut int size

    # class method comment
    fn area() → int {
        return 1
    }
}

# enum comment
@ public
enum Color {
    # enum member comment
    red,
    # second enum member comment
    blue
}

# interface comment
@ public
interface Readable {
    # interface method comment
    fn read() → string

    # second interface method comment
    fn close()
}

# union comment
@ public
union Piece {
    @ shared
    int rank

    # union variant with fields comment
    Pawn {
        int file
    },
    # union variant comment
    Rook
}

main {
    print \"x\"
}
";

fn format_stdout(source: &str) -> String {
    let dir = TempDir::new("faber-format", "member-comments");
    let path = dir.join("main.fab");
    fs::write(&path, source).expect("write source");
    let output = Command::new(env!("CARGO_BIN_EXE_faber"))
        .args(["format", "--stdout", path.to_str().expect("utf8 path")])
        .output()
        .expect("spawn faber format");
    assert!(
        output.status.success(),
        "faber format --stdout failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf8 stdout")
}

#[test]
fn format_keeps_comment_above_every_declaration_kind() {
    let formatted = format_stdout(SOURCE);
    let missing: Vec<&str> = SOURCE
        .lines()
        .map(str::trim_start)
        .filter(|line| line.starts_with('#') && !formatted.contains(*line))
        .collect();
    assert!(
        missing.is_empty(),
        "faber format dropped {missing:?}\n--- formatted ---\n{formatted}"
    );
    assert_eq!(
        formatted.trim_end(),
        SOURCE.trim_end(),
        "canonical source must format unchanged"
    );
}
