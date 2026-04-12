# Basic Usage

## 1) Set environment variables

```bash
export EXTERNAL_SUBAGENT_BASE_URL="http://127.0.0.1:8787"
export EXTERNAL_SUBAGENT_API_KEY="your-key"
export EXTERNAL_SUBAGENT_MODEL="gpt-4o-mini"
```

## 2) Write prompt file

```bash
cat > /tmp/external-subagent-prompt.txt <<'EOF'
You are a code reviewer.
Task: review this design section and list top 5 risks.
Constraints:
- Keep each risk in one sentence.
- Include a mitigation for each risk.
Output format:
- Bullet list only.
EOF
```

## 3) Invoke external subagent

```bash
bash ".cursor/skills/external-api-subagent/scripts/run_external_subagent.sh" \
  --prompt-file "/tmp/external-subagent-prompt.txt"
```
