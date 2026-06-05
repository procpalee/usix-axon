//! 셸 실행 (실험 기능). `policy::set_shell_enabled` 로 명시 동의했을 때만 호출.
//!
//! ⚠️ 데몬이 시킨 명령을 사용자 권한으로 실행한다. 출력은 64KB 절단(문자 경계 보존).

const MAX_OUT: usize = 64 * 1024;

/// `(출력, 성공여부)`. 출력 = stdout+stderr 합본.
pub fn run(command: &str) -> (String, bool) {
    if command.trim().is_empty() {
        return ("빈 명령".into(), false);
    }
    let output = if cfg!(target_os = "windows") {
        std::process::Command::new("cmd").args(["/C", command]).output()
    } else {
        std::process::Command::new("sh").args(["-c", command]).output()
    };
    match output {
        Ok(o) => {
            let mut s = String::from_utf8_lossy(&o.stdout).into_owned();
            if !o.stderr.is_empty() {
                if !s.is_empty() {
                    s.push('\n');
                }
                s.push_str(&String::from_utf8_lossy(&o.stderr));
            }
            // 64KB 절단 — String::truncate 는 문자 경계 아니면 panic 하므로 경계까지 당긴다.
            if s.len() > MAX_OUT {
                let mut end = MAX_OUT;
                while !s.is_char_boundary(end) {
                    end -= 1;
                }
                s.truncate(end);
                s.push_str("\n…(절단됨)");
            }
            (s, o.status.success())
        }
        Err(e) => (format!("실행 실패: {e}"), false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_and_captures() {
        let (out, ok) = run("echo usix");
        assert!(ok);
        assert!(out.contains("usix"));
    }

    #[test]
    fn nonzero_exit_is_failure() {
        assert!(!run("exit 3").1);
    }

    #[test]
    fn empty_rejected() {
        assert!(!run("   ").1);
    }
}
