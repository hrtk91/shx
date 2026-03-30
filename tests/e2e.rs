use std::process::Command;

fn run_shx(input: &str) -> String {
    let output = Command::new("dash")
        .arg("-c")
        .arg(&shx::transpile(input).unwrap())
        .output()
        .expect("failed to run dash");
    assert!(
        output.status.success(),
        "dash exited with error: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn e2e_echo() {
    assert_eq!(run_shx("echo hello"), "hello\n");
}

#[test]
fn e2e_if_else() {
    let input = r#"
x=1
if [ "$x" -eq 1 ] {
  echo "yes"
} else {
  echo "no"
}
"#;
    assert_eq!(run_shx(input), "yes\n");
}

#[test]
fn e2e_for_loop() {
    let input = r#"
for i in a b c {
  echo "$i"
}
"#;
    assert_eq!(run_shx(input), "a\nb\nc\n");
}

#[test]
fn e2e_while_loop() {
    let input = r#"
n=0
while [ "$n" -lt 3 ] {
  n=$((n + 1))
  echo "$n"
}
"#;
    assert_eq!(run_shx(input), "1\n2\n3\n");
}

#[test]
fn e2e_match() {
    let input = r#"
val="bar"
match "$val" {
  "foo" => echo "got foo"
  "bar" | "baz" => echo "got bar or baz"
  _ => echo "other"
}
"#;
    assert_eq!(run_shx(input), "got bar or baz\n");
}

#[test]
fn e2e_nested() {
    let input = r#"
for i in 1 2 {
  if [ "$i" -eq 1 ] {
    echo "one"
  } else {
    echo "two"
  }
}
"#;
    assert_eq!(run_shx(input), "one\ntwo\n");
}

#[test]
fn e2e_heredoc() {
    let input = "cat <<EOF\nhello world\nEOF\n";
    assert_eq!(run_shx(input), "hello world\n");
}

#[test]
fn e2e_comment_ignored() {
    let input = "# this is a comment\necho ok\n";
    assert_eq!(run_shx(input), "ok\n");
}

#[test]
fn e2e_match_multiline_arm() {
    let input = r#"
match "start" {
  "start" => {
    echo "starting"
    echo "done"
  }
  _ => echo "unknown"
}
"#;
    assert_eq!(run_shx(input), "starting\ndone\n");
}

#[test]
fn e2e_passthrough_posix() {
    let input = r#"
FOO="hello"
echo "${FOO} world"
"#;
    assert_eq!(run_shx(input), "hello world\n");
}
