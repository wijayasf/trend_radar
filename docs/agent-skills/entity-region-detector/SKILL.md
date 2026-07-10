# Entity Region Detector

## Purpose
Guide agents when detecting AI agent/tool entities and region signals for Indonesia and global trend rankings.

## When to Use
Use this skill when editing keyword/alias config, designing entity normalization, mapping posts to regions, or preparing Top 20 Indonesia and Global inputs.

## Inputs
- Current task objective.
- `config/keywords.yml`.
- `config/aliases.yml`.
- Sample posts or expected examples if provided.
- Current `AGENTS.md` rules.

## Rules
- Read `AGENTS.md` before making changes.
- Prefer config-driven detection before adding complex classifier code.
- Keep aliases explainable and easy to review.
- Do not assume a post is Indonesia-only from a single weak signal.
- Preserve raw mention text separately from normalized entity names when designing data flow.
- Track ambiguity rather than forcing low-confidence matches.
- Avoid adding external NLP dependencies unless clearly justified.
- Update progress report, token usage log, and handoff note if work remains unfinished.

## Output Requirement
- List entity and region detection rules added or changed.
- Note ambiguous cases and confidence assumptions.
- Provide sample expected mappings when applicable.
- Identify next validation dataset or test need.

## Risk Note
Entity aliases and region signals can create false rankings if ambiguous names, multilingual posts, or global-local overlap are handled too aggressively.

## Completion Checklist
- [ ] `AGENTS.md` was followed.
- [ ] Detection rules remain config-driven where possible.
- [ ] Ambiguous cases are documented.
- [ ] No unnecessary dependency was added.
- [ ] Progress report and token usage log were updated.
- [ ] Handoff note was updated if pending work remains.
