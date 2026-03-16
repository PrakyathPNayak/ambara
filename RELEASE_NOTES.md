# Ambara v0.5.0 Release Notes

**Release Date:** 16 March 2026

## Highlights

- Chatbot responses now produce query-aware pipelines instead of a fixed default graph.
- Natural-language questions now return relevant filter guidance instead of generic fallback text.
- Offline fallback graph generation now avoids invalid batch-versus-image port combinations.
- Release metadata has been aligned so pushing the `v0.5.0` tag triggers the existing multi-platform release workflow.

## What Changed

### Chatbot generation
- Removed the hardcoded default response path that always produced the same graph.
- Added retrieval-driven pipeline construction for local/mock fallback mode.
- Preserved real LLM generation for configured Anthropic/OpenAI backends.
- Kept repair-loop behavior intact for test-injected and real LLM-backed generation.

### Conversational responses
- Improved intent handling so natural language questions do not require a trailing `?`.
- Added relevant-filter summaries for non-graph chat requests.
- Improved graph-generation success and failure messages so the UI reflects what was actually generated.

### Release readiness
- Bumped workspace, UI, and Tauri app versions to `0.5.0`.
- Updated changelog and release notes for the `v0.5.0` tag-driven GitHub Actions release flow.

## Verification

- `python3 -m pytest chatbot/tests/`: 25 passed
- `cargo test --lib`: 111 passed, 2 ignored
- `cargo check --workspace`: passed
- `npm --prefix ui run build`: passed

## Release workflow

Pushing tag `v0.5.0` triggers [.github/workflows/build-release.yml](.github/workflows/build-release.yml), which builds Linux, macOS, and Windows artifacts and publishes a GitHub release using this file as the release body.

## Links

- [Full Changelog](https://github.com/PrakyathPNayak/ambara/compare/v0.4.0...v0.5.0)
- [Documentation](https://github.com/PrakyathPNayak/ambara#readme)
- [Report Issues](https://github.com/PrakyathPNayak/ambara/issues)
