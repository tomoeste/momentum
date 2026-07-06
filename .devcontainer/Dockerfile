FROM node:22-slim

RUN npm install -g @anthropic-ai/claude-code \
    && apt-get update && apt-get install -y git curl build-essential pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Non-root user — required, the flag refuses to start as root
RUN useradd -m -s /bin/bash agent
USER agent
WORKDIR /work

# Pre-trust /work so headless -p runs actually apply permissions.allow
RUN echo '{"projects":{"/work":{"hasTrustDialogAccepted":true}}}' > /home/agent/.claude.json