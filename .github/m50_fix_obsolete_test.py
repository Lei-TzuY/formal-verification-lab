from pathlib import Path

path = Path("tests/combined_fair_cli.rs")
text = path.read_text()
old = '''\n#[test]\nfn direct_monitor_mixed_fairness_remains_fail_closed() {\n    let output = fvlab(&[\n        "monitor",\n        "session-unfair-close",\n        "--weak-fair-action",\n        "close",\n        "--strong-fair-action",\n        "close",\n    ]);\n    assert_eq!(output.status.code(), Some(2));\n    assert!(stderr(&output).contains("cannot combine weak and strong fairness assumptions"));\n}\n'''
count = text.count(old)
if count != 1:
    raise SystemExit(f"obsolete mixed monitor gate: expected exactly one match, found {count}")
path.write_text(text.replace(old, "", 1))
