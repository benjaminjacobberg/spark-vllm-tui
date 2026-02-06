# Running MiniMax-M2.1 with Claude Code via vLLM

## Start the vLLM Server

```bash
vllm serve mratsim/MiniMax-M2.1-FP8-INT4-AWQ \
    --served-model-name MiniMax-M2.1 \
    --trust-remote-code \
    --port 8000 \
    --host 0.0.0.0 \
    --gpu-memory-utilization 0.8 \
    -tp 2 \
    --distributed-executor-backend ray \
    --max-model-len 196608 \
    --load-format fastsafetensors \
    --enable-auto-tool-choice \
    --tool-call-parser minimax_m2 \
    --reasoning-parser minimax_m2_append_think
```

## Launch Claude Code

```bash
ANTHROPIC_BASE_URL=http://spark-472b.local:8000 \
ANTHROPIC_API_KEY=dummy \
ANTHROPIC_AUTH_TOKEN=dummy \
ANTHROPIC_DEFAULT_OPUS_MODEL=MiniMax-M2.1 \
ANTHROPIC_DEFAULT_SONNET_MODEL=MiniMax-M2.1 \
ANTHROPIC_DEFAULT_HAIKU_MODEL=MiniMax-M2.1 \
claude
```

## Zsh Alias

Add the following to your `~/.zshrc`:

```bash
alias minimax='ANTHROPIC_BASE_URL=http://spark-472b.local:8000 \
  ANTHROPIC_API_KEY=dummy \
  ANTHROPIC_AUTH_TOKEN=dummy \
  ANTHROPIC_DEFAULT_OPUS_MODEL=MiniMax-M2.1 \
  ANTHROPIC_DEFAULT_SONNET_MODEL=MiniMax-M2.1 \
  ANTHROPIC_DEFAULT_HAIKU_MODEL=MiniMax-M2.1 \
  claude'
```

Or as a one-liner:

```bash
echo "alias minimax='ANTHROPIC_BASE_URL=http://spark-472b.local:8000 ANTHROPIC_API_KEY=dummy ANTHROPIC_AUTH_TOKEN=dummy ANTHROPIC_DEFAULT_OPUS_MODEL=MiniMax-M2.1 ANTHROPIC_DEFAULT_SONNET_MODEL=MiniMax-M2.1 ANTHROPIC_DEFAULT_HAIKU_MODEL=MiniMax-M2.1 claude'" >> ~/.zshrc && source ~/.zshrc
```

