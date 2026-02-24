# Bootstrap Rules

아래 YAML 규칙을 기준으로 `spec` 문자열을 매칭해 bootstrap 템플릿을 선택한다.

- `match_any` 중 하나라도 `spec`에 포함되면 해당 rule이 선택된다.
- `template` 지원값: `rust`, `react-native`, `node-react`

```yaml
rules:
  - name: rust
    match_any:
      - rust
      - cargo
      - tokio
    template: rust

  - name: react-native
    match_any:
      - react native
      - react-native
      - expo
    template: react-native

  - name: node-react
    match_any:
      - react
      - next
      - node
      - typescript
      - javascript
    template: node-react
```
