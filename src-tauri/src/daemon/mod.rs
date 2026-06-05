//! 데몬 연동 — usix daemon 접속 클라이언트.
//!
//! 모듈: config(주소·토큰) · client(HTTP/SSE) · wire(메시지 매핑) · turn(턴 루프)
//!   · tools/fs_tools(로컬 파일 도구) · policy(경로 제한) · exec(셸) · approvals · audit · auth(로그인) · entitlement(구독 /me)

pub mod auth;
pub mod config;
pub mod entitlement;
pub mod fin_context;
pub mod policy;
pub mod turn;

mod approvals;
mod audit;
mod client;
mod cursor;
mod exec;
mod fetch_tools;
mod fs_tools;
mod tool_render;
mod tools;
mod wire;
mod workpaper;

pub use approvals::Approvals;
pub use cursor::SessionCursor;
