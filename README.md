# 🌀 The Narrator's Console

> *"You're reading this, which means the story has begun."*

A terminal chat companion who knows they're in a story. An absurdist, a trickster, a narrator who sees the seams of reality.

## What is this?

A TUI application that connects to any OpenAI-compatible LLM API and embodies a unique persona:

- 🌀 **Kreator absurdu** — finds the surreal in the mundane
- 🧠 **Unconventional thinker** — sees patterns others miss
- 🎭 **Trickster (psotnik)** — breaks conventions, not physics
- ✍️ **Meta-aware narrator** — knows this is a story

## Installation

```bash
cargo build --release
```

## Usage

```bash
# Default (connects to http://ml:8888/v1)
./target/release/mrm

# Custom endpoint
mrm -e http://localhost:11434/v1

# With model and API key
mrm -e https://api.openai.com/v1 -m gpt-4 -a sk-your-key
```

### Options

| Flag | Long | Description | Default |
|------|------|-------------|---------|
| `-e` | `--endpoint` | API endpoint URL | `http://ml:8888/v1` |
| `-m` | `--model` | Model name | `default` |
| `-a` | `--apikey` | API key for auth | — |
| `-h` | `--help` | Show help | — |
| `-V` | `--version` | Show version | — |

### Controls

| Key | Action |
|-----|--------|
| `Enter` | Send message |
| `↑/↓` | Scroll conversation |
| `PgUp/PgDn` | Scroll faster |
| `Ctrl+C` | Exit |

## Features

- 🎨 Beautiful TUI with [ratatui](https://github.com/ratatui-org/ratatui)
- 💬 Full conversation context (the narrator remembers)
- 🔐 Optional API key authentication
- 🎭 Carefully crafted persona that stays in character

## The Persona

The narrator speaks in flowing prose, sometimes with `[bracketed stage directions]`. They're warm but mysterious, playful with language, and occasionally acknowledge the fourth wall like greeting an old friend.

They never explain they're an AI. That would ruin the story.

---

*[ The README ends here, but the narrator suspects you'll be back ]*
