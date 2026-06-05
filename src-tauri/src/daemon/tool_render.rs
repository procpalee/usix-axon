//! 도구 활동 렌더링 보조 — 인자 요약(detail)·결과 미리보기(preview)를 채팅용 짧은 문자열로.
//! 긴 출력을 그대로 흘리면 채팅이 도배되므로 앞부분만 자른다. CLI 패턴 참조한 자체 구현(§0).

use serde_json::Value;

/// 여러 줄을 공백으로 평탄화 + max 글자에서 컷(… 표시). 한글 char 경계 보존.
fn one_line(s: &str, max: usize) -> String {
    let flat: String = s
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    let t = flat.trim();
    if t.chars().count() <= max {
        return t.to_string();
    }
    let cut: String = t.chars().take(max).collect();
    format!("{cut}…")
}

/// 앞 max_lines 줄만, 각 줄 120자 컷. 더 있으면 "…(N줄 더)".
fn head_lines(s: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    let show = lines.len().min(max_lines);
    let mut out = String::new();
    for line in &lines[..show] {
        out.push_str(&one_line(line, 120));
        out.push('\n');
    }
    if lines.len() > max_lines {
        out.push_str(&format!("…({}줄 더)", lines.len() - max_lines));
    }
    out
}

/// JSON `{"stdout": ...}` 래핑이면 stdout 만 벗긴다(bash 등 구조화 출력).
fn unwrap_stdout(s: &str) -> String {
    serde_json::from_str::<Value>(s)
        .ok()
        .and_then(|v| v.get("stdout").and_then(|x| x.as_str()).map(str::to_string))
        .unwrap_or_else(|| s.to_string())
}

/// 도구 출력 미리보기. read_file 류는 더 길게(6줄), 그 외 3줄/한 줄.
pub fn preview(tool: &str, output: &str) -> String {
    let display = unwrap_stdout(output);
    let display = display.trim();
    if display.is_empty() {
        return String::new();
    }
    let max = if tool == "read_file" { 6 } else { 3 };
    if display.contains('\n') {
        head_lines(display, max)
    } else {
        one_line(display, 160)
    }
}

/// 도구 인자에서 핵심만 뽑아 한 줄로 — fetch=url, bash=command, search=query, file=path.
pub fn detail(tool: &str, args: &Value) -> String {
    let primary = match tool {
        "bash" => args.get("command"),
        "fetch_url" | "fetch" => args.get("url"),
        "web_search" => args.get("query"),
        "read_file" | "write_file" | "edit_file" => args.get("path"),
        _ => None,
    };
    let v = primary
        .or_else(|| {
            ["url", "command", "query", "path", "pattern"]
                .iter()
                .find_map(|k| args.get(*k))
        })
        .and_then(|x| x.as_str())
        .unwrap_or("");
    one_line(v, 120)
}
