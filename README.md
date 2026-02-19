# orchestra

Rust 기반 CLI 오케스트레이터입니다.  
`spec.yaml`/`todos.yaml`를 중심으로 작업을 생성하고, 병렬 실행 UI와 후속 점검 흐름을 제공합니다.

## Build

```bash
cargo build
```

## CLI

```bash
orc serve
```
- 결과 콜백 서버 실행

```bash
orc show-ui
```
- UI 실행 (기존 `run-test` 별칭 지원)

```bash
orc make-spec --project test
```
- 대화형 입력으로 `spec.yaml` 생성

```bash
orc fill-spec --project test --input-path input.txt
```
- `input.txt`(`#`, `>`, `-`)를 파싱해 `spec.yaml`의 `tasks` 채움

```bash
orc make-todos --project test
```
- `spec.yaml` 기반으로 `todos.yaml` 항목 생성/추가

```bash
orc run-paralles --server-url http://127.0.0.1:7878 --n 2 --msg "task A" --msg "task B"
```
- 병렬 작업 실행 (기존 `run-parallel` 별칭 지원)

```bash
orc check-last --project test
```
- 최종 점검/개선 단계 실행 (refactor change에서 수행)

## Data Files

- `./.project/<project>/spec.yaml`
- `./.project/<project>/todos.yaml`
