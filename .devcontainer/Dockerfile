FROM node:22-slim

RUN npm install -g @anthropic-ai/claude-code \
    && apt-get update && apt-get install -y git curl \
    && rm -rf /var/lib/apt/lists/*

# Non-root user — required, the flag refuses to start as root
RUN useradd -m -s /bin/bash agent
USER agent
WORKDIR /work