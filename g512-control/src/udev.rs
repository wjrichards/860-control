//! The shell command that installs the udev rule, shown on "No access"
//! status pages.

/// The rules file, as packaged.
pub const RULES: &str = include_str!("../../packaging/70-g512-control.rules");
const TARGET: &str = "/etc/udev/rules.d/70-g512-control.rules";

/// The rule lines of `rules`: everything but comments and blank lines.
fn rule_lines(rules: &str) -> impl Iterator<Item = &str> {
    rules.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#'))
}

/// Install every rule line, so following it never drops another device's
/// rule, then reload udev and re-trigger the hidraw nodes.
pub fn install_command(rules: &str) -> String {
    let quoted: Vec<String> = rule_lines(rules).map(|l| format!("  '{}'", l.replace('\'', r"'\''"))).collect();
    format!(
        "printf '%s\\n' \\\n{} \\\n  | sudo tee {TARGET}\nsudo udevadm control --reload && sudo udevadm trigger --subsystem-match=hidraw",
        quoted.join(" \\\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_every_rule_line() {
        let cmd = install_command(RULES);
        assert!(cmd.contains(r#"ATTRS{idProduct}=="c33c""#), "keyboard rule: {cmd}");
        assert!(cmd.contains(r#"ATTRS{idProduct}=="c08b""#), "mouse rule: {cmd}");
        assert!(cmd.contains("| sudo tee /etc/udev/rules.d/70-g512-control.rules"));
    }

    #[test]
    fn command_round_trips_to_the_rules() {
        // The quoted arguments between printf and tee are exactly the rule lines.
        let cmd = install_command(RULES);
        let args: Vec<&str> = cmd
            .lines()
            .filter_map(|l| l.trim().strip_prefix('\'').and_then(|l| l.strip_suffix("' \\").or(l.strip_suffix('\''))))
            .collect();
        let expected: Vec<&str> = rule_lines(RULES).collect();
        assert_eq!(args, expected);
        assert_eq!(expected.len(), 2, "keyboard and mouse");
    }

    #[test]
    fn quotes_are_escaped() {
        assert!(install_command("a'b\n").contains(r"'a'\''b'"));
    }
}
