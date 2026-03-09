# problem
- rc에서 제공하던 CLI 명령을 orc CLI에서도 직접 호출하고 싶다.

# tasks
- orc CLI에 `clit` 명령을 추가한다.
- `orc clit ...` 호출 시 시스템의 `rc` 실행 파일로 인수를 포워딩한다.
- usage/help에 `clit` 명령을 명시한다.

# check
- `cargo check --manifest-path /home/tree/project/rust-orc/Cargo.toml`
- `cargo run --manifest-path /home/tree/project/rust-orc/Cargo.toml -- clit --help`
