# OpenCode Configuration for Ganesha

This directory contains configuration for the OpenCode server that powers Ganesha.

## Setting Up Models

To use models with Ganesha, you need to set up API keys for your preferred providers:

### Anthropic (Claude models)
1. Get your API key from [Anthropic Console](https://console.anthropic.com/)
2. Set environment variable: `ANTHROPIC_API_KEY=your-key-here`

### OpenAI (GPT models)
1. Get your API key from [OpenAI Platform](https://platform.openai.com/)
2. Set environment variable: `OPENAI_API_KEY=your-key-here`

## Configuring Models

Edit `config.json` to:
- Add new models
- Change the default model
- Adjust model parameters (temperature, maxTokens, etc.)

## Slash Commands

Project commands live in `commands/` (e.g. `/ponytail*` from
`@dietrichgebert/ponytail`). Restart `opencode serve` after adding new ones.

## Skills

Coding skills vendored from
[alirezarezvani/claude-skills](https://github.com/alirezarezvani/claude-skills)
live in `skills/` (137 skills, `engineering/` + `engineering-team/` domains).
See `skills/README.md` for provenance and update instructions.

## Troubleshooting

If models aren't connecting:
1. Check your API keys are set correctly
2. Verify the opencode server is running (should auto-start with Ganesha)
3. Check the server logs for any errors
4. Ensure your `.opencode/config.json` is valid JSON

## Default Models

The following models are pre-configured:
- `claude-3.5-sonnet` (default)
- `claude-3-opus`
- `gpt-4`
- `gpt-4-turbo`

You can add or remove models as needed by editing `config.json`.
