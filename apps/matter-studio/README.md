# Matter Studio

Matter Studio is a local visual workbench for Matter Core.

It gives you:

- a dark chat interface for Matter-focused AI work
- OpenAI-compatible chat through `OPENAI_API_KEY`
- Gemini chat through `GEMINI_API_KEY`
- local Matter execution through `matter-cli`
- quick actions for `run`, `check-json`, `reflect-json`, and `reflexive-guard-json`

## Run

From this folder:

```powershell
copy .env.example .env
npm start
```

Then open:

```text
http://127.0.0.1:4177
```

## API Keys

Put keys in `.env`. Do not put them in frontend code.

```text
OPENAI_API_KEY=sk-...
OPENAI_MODEL=gpt-4o-mini

GEMINI_API_KEY=...
GEMINI_MODEL=gemini-1.5-flash
```

ChatGPT Plus is not the same as OpenAI API billing. The app uses API keys.

## Matter CLI

By default the server looks for a release/debug `matter-cli.exe`, then falls back to `cargo run -q -p matter-cli --`.

You can override:

```text
MATTER_CLI=F:\path\to\matter-cli.exe
```

