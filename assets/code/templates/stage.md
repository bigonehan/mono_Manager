# stage

## stage_init_plan
- input: name, description, path, spec
- collect: feature list (`#기능 이름 - 기능 규칙 > 기능 순서`)
- output: project.md `## plan`, drafts_list.yaml.planned

## stage_draft
- show: planned list
- create: `./.project/feature/<feature>/draft.yaml`
- wait: set_draft, add_draft, enter_parallel

## stage_build
- check_draft -> build_draft -> enter_check_job

## constraints
- domain/flow constraints are composed into task constraints.
