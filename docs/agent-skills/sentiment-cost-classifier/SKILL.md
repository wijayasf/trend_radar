# Sentiment Cost Classifier

## Purpose
Guide agents when designing or implementing sentiment and cost-signal classification for AI Agent Trend Radar.

## When to Use
Use this skill when working on positive, negative, neutral, mixed sentiment or cost signals such as expensive, token-heavy, quota-limited, and worth-it mentions.

## Inputs
- Current task objective.
- `config/keywords.yml`.
- `config/scoring.yml`.
- Sample posts or labeled examples if provided.
- Current `AGENTS.md` rules.

## Rules
- Read `AGENTS.md` before making changes.
- Start with deterministic rules before optional local LLM/Ollama classification.
- Keep labels explainable and auditable.
- Do not send local user data to remote LLMs.
- Do not add Ollama or LLM dependencies unless the task explicitly moves to that phase.
- Track mixed sentiment separately instead of forcing positive or negative labels.
- Keep cost-signal detection separate from general sentiment.
- Update progress report, token usage log, and handoff note if work remains unfinished.

## Output Requirement
- Describe labels, rules, and confidence assumptions.
- List config changes or classifier boundaries.
- Include known false-positive/false-negative risks.
- State validation performed or sample cases reviewed.

## Risk Note
Sentiment and cost classification can be misleading when posts are sarcastic, multilingual, or comparing multiple tools. Preserve uncertainty and avoid opaque scoring.

## Completion Checklist
- [ ] `AGENTS.md` was followed.
- [ ] Rules are explainable and scoped.
- [ ] No remote data leak path was introduced.
- [ ] Cost signals are not conflated with sentiment.
- [ ] Progress report and token usage log were updated.
- [ ] Handoff note was updated if pending work remains.
