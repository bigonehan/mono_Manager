# llm chat system
- orc chat -n "이름" 을 하면 chat 모드가 활성화된다.
- chat 모드 활성화 시 모든 대화 내용은 orc 설치 폴더 .temp/이름.yaml 파일에 기록된다.
- orc chat -n "이름" 을 하면 현재 세션은 임의의 session_id 값(8자리 난수)을 할당받는다.
- 내부 변수 llm_id 에는 방금 생성한 session_id 값을 저장한다.
- chat 모드의 messages 형태는 message_id | command | data | receiver | sender_id 의미를 가진다.
- chat 모드 종료 단축키는 ctrl+d 키이며 chat 모드 실행 시 도움말로 종료 방법을 출력한다.
- .temp/이름.yaml 파일이 없으면 에러를 출력하고 종료한다. 자동 생성하지 않는다.
- 채팅방 파일 형식은 YAML 이다.
> orc chat -n "a" 라는 이름으로 명령을 호출한다.
> orc 설치 폴더 .temp/a.yaml 가 있는지 확인한다.
> 파일이 없으면 에러를 출력하고 종료한다.
> 파일이 있으면 build_llm_id() 함수를 호출해서 8자리 난수 session_id 를 만든다.
> 내부 변수 llm_id 에 8자리 난수를 저장한다.
> users 에 sender_id(session_id) 를 반영한다.
> .temp/이름.yaml 파일의 messages 를 last_read_message_id 기준으로 읽는다.
> config 옵션 MAX_read_time 동안 대기 후 다시 이름.yaml 파일의 messages 값을 가져온다.
> 새로운 값이 있다면 값을 표시하고 last_read_message_id 를 갱신한다.

# llm chat system - send
- orc chat -n "이름" -m "메시지" -i "receiver_id"(옵션) --data "data"(옵션) 으로 메시지를 보낼 수 있다.
- 이때 orc 설치폴더 .temp/이름.yaml 이 없으면 에러를 출력하고 종료한다. 자동 생성하지 않는다.
- 메시지를 보내면 message_id | command | data | receiver | sender_id 의미에 맞게 YAML messages 에 저장한다.
- receiver 는 생략할 수 있다.
- data 는 생략할 수 있다.
- ID 분리 규칙은 다음과 같다: session_id(세션), sender_id(발신자), message_id(메시지).
> orc chat -n "a" -m "hello" 를 입력한다.
> orc 설치 폴더 .temp/a.yaml 파일이 있는지 검사한다.
> 파일이 없으면 에러를 출력하고 종료한다.
> 파일이 있으면 sender_id 용 임의 아이디 8자리를 생성한다.
> 내부 변수 llm_id 에 방금 만든 난수를 저장한다.
> messages 에 message_id=<new_id>, command="hello", data=null, receiver=null, sender_id=<llm_id> 를 기록한다.

# chat room yaml format
- chat room 파일은 아래 필드를 가진 YAML 구조를 사용한다.
> room_name: "a"
> users:
>   - user_id: "A1B2C3D4"
>     role: "user"
> messages:
>   - message_id: "M0000001"
>     command: "hello"
>     data: null
>     receiver: null
>     sender_id: "A1B2C3D4"
>     created_at: "2026-03-03T00:00:00Z"
