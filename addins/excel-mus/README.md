# axon MUS — 엑셀 작업창 애드인

선택 범위에서 **화폐단위표본추출(MUS/PPS)** 표본을 결정론으로 뽑아 새 시트에 출력하는 Office.js 애드인이다. 수치 계산은 전부 `axon-core`의 순수 결정론 코어를 **WASM**(`crates/axon-wasm`)으로 호출 — 데이터는 기기를 떠나지 않는다. 데스크톱 axon 앱과 **동일한 단일 알고리즘 소스**라 같은 (모집단·PM·시드)는 같은 표본을 낸다.

## 동작
1. 시트에서 분석 대상(헤더행 + 데이터행)을 선택하거나, 선택을 비우면 활성 시트의 사용 영역 전체를 쓴다.
2. 작업창에서 **금액열 · 수행중요성(PM) · 신뢰수준(90/95/99%) · 시드**를 지정.
3. **추출** → 선택된 행 + `Type`(Key Item|PPS Sample)·`Book_Value`·`Audit_Value` 컬럼을 새 시트(`MUS 95% s42`)에 출력. 시트명에 시드가 박혀 워크페이퍼가 재현 가능.

> 양수 금액만 모집단. PM 이상은 Key Item 전수, 나머지는 시드 기반 체계적 추출.

## 개발 실행 (요구: Rust + wasm-pack, Node + npm)

```bash
# 1) WASM 코어 빌드 → src/wasm-pkg/ (axon-core 단일 소스)
npm run wasm

# 2) 의존성 + dev 인증서(최초 1회 — Office 작업창은 HTTPS 필수)
npm install
npm run certs

# 3) HTTPS dev 서버(https://localhost:3000) + Excel 사이드로드
npm run dev          # 별도 터미널에서 유지
npm run start        # office-addin-debugging — 데스크톱 Excel 에 사이드로드
```

웹 Excel 에서는 `삽입 → 추가 기능 → 내 추가 기능 → 내 추가 기능 업로드`로 `manifest.xml`을 올린다.

- `npm run validate` — manifest 검증
- `npm run build` — 정적 번들(`dist/`)

## 배포 메모
`manifest.xml`의 `<Id>` GUID·`SourceLocation`·`AppDomains`·아이콘 URL 의 `https://localhost:3000` 호스트를 실제 배포 HTTPS 도메인으로 교체할 것. 릴리스용 WASM 최적화가 필요하면 binaryen 설치 후 `crates/axon-wasm/Cargo.toml`의 `wasm-opt = false`를 제거.
