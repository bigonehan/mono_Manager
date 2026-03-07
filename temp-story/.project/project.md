# info
name : temp-story
description : story test project
spec : 장편
path : /home/tree/project/rust-orc/temp-story

# scene
- 이야기의 장면 목록

# character
- 이야기의 등장인물 목록

# rules
- 프로젝트 내부의 공통 규칙

# constraints
- 프로젝트 내부의 공통 제약

# domains
## story
### states
- 기획
- 작성
- 퇴고
### action
- 장면 작성
- 캐릭터 설정
### rules
- 모든 항목은 `-` 리스트로 작성
### constraints
- spec 값은 장편/단편/중 중 하나만 허용
