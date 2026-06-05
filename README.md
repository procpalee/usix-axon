# axon

<div align="center">

**한국어** &nbsp;|&nbsp; [English](README.en.md)

[![Tauri](https://img.shields.io/badge/Tauri-2.x-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-1.85+-CE422B?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/License-AGPL--3.0-blue)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-lightgrey)

회계 업무 종사자를 위한 **로컬 우선·결정론 재무/감사 분석 데스크톱**

</div>

---

전자공시(OpenDART) 재무제표와 원장 데이터를 **로컬에서** 표·그래프·비율로 분석한다. 수치·합계·검증은 전부 결정론 코드로 계산 — LLM 0, 오프라인 동작, 데이터는 기기 밖으로 나가지 않는다. 비싼 독점 감사 분석 도구와 개발자용 라이브러리 사이의 빈자리를 채우는, 회계 업무 종사자용 오픈 분석기다. 회계 실무자가 자기 업무에 쓰려고 직접 만들기에, 현장에서 필요한 것부터 구현한다.

> **개발 중** — OpenDART 조회·다년도 분석·내보내기·추세 그래프는 동작한다. 재무비율·숫자 대사(tie-out)·원장(감사) 분석은 로드맵.

## 특징

- **전자공시 조회** — OpenDART로 상장사 재무제표를 회사명 검색 한 번에
- **N년 전체기간** — 시작·종료 연도를 계정 × 연도 표로 병합 (여러 연도 동시 조회)
- **추세 그래프** — 계정을 클릭해 연도별 추세 비교. 지수(기준=100) 모드로 자릿수가 크게 다른 계정도 한 평면에서 비교
- **내보내기** — 엑셀(`.xlsx`)·CSV·JSON·PDF 인쇄
- **결정론 우선** — 모든 수치·합계를 순수 Rust 결정론 코드로, 네트워크·LLM 없이 정확
- **로컬 우선 = 기밀 유지** — 모든 결정론 분석이 기기 안에서 끝난다. 감사·고객 데이터 유출 없음
- **다크 / 라이트 · 한 / 영** — 고대비 테마와 언어 토글

선택적으로 usix daemon에 연동하면 AI 감사 보조(조서·보고서 초안 등)를 쓸 수 있다 — 이 부분만 유료이며, 연동하지 않으면 어떤 데이터도 기기를 떠나지 않는다.

## 시작하기

**요구사항** — Rust 1.85+ · Node.js + npm · (Linux) `webkit2gtk-4.1`

```bash
npm install
npm run tauri dev      # 개발 모드로 실행
npm run tauri build    # 데스크톱 설치 파일 빌드
```

### OpenDART 키

재무제표 조회에는 OpenDART 인증키가 필요하다 (무료). 발급 절차는 다음과 같다.

1. [OpenDART](https://opendart.fss.or.kr) 접속 → 상단 **인증키 신청/관리 → 인증키 신청**
2. 이름·이메일·사용목적 입력, 약관 동의 후 신청
3. 가입 이메일로 온 인증 메일에서 인증 완료 — 개인은 즉시 발급, 법인은 사업자등록증 심사 후 발급
4. **인증키 신청/관리 → 오픈API 이용현황**에서 발급된 인증키 확인
5. 앱의 **설정 → OpenDART 키**에 입력하면 OS 키체인에 안전하게 저장된다

> 인증키 하나로 기본 1일 20,000건까지 조회할 수 있다.

## 기술 스택

Tauri 2 · Rust · Vanilla TypeScript + Vite · rust_decimal · rust_xlsxwriter

## 라이선스

[AGPL-3.0](LICENSE)
