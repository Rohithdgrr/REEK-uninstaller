# Project skills (OpenCode)

Vendored from [alirezarezvani/claude-skills](https://github.com/alirezarezvani/claude-skills)
at commit `19392f7` (2026-08-26) — **coding domains only**
(`engineering/` + `engineering-team/`, converted with the
`compatibility: opencode` frontmatter per upstream `scripts/convert.sh`).

137 skills. Each folder is self-contained (`SKILL.md` + `scripts/` /
`references/` / `templates/` where the source skill ships them).

## Updating / adding more domains

```powershell
git clone -c core.symlinks=true --depth 1 https://github.com/alirezarezvani/claude-skills.git <tmp>
# edit -IncludeTops in convert-opencode.ps1 to add domains, then re-run it
```

Available source domains: `engineering`, `engineering-team`, `product-team`,
`marketing-skill`, `marketing`, `c-level-advisor`, `c-level-agents`,
`project-management`, `ra-qm-team`, `compliance-os`, `business-growth`,
`business-operations`, `commercial`, `finance`, `research`, `research-ops`,
`productivity`, `agent-launcher`, `markdown-html`, `loop-library`.
