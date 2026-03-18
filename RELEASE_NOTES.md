# Ambara v0.6.0 Release Notes

**Release Date:** 18 March 2026

## Highlights

- Chatbot responses now produce query-aware pipelines instead of a fixed default graph.
- Natural-language questions now return relevant filter guidance instead of generic fallback text.
- Offline fallback graph generation now avoids invalid batch-versus-image port combinations.
- Release workflow now supports the `v0.6.0` release tag and manual dispatch versioning.
- UI layout has been refined for chatbot visibility, balanced pane sizing, and settings placement.
- Node resizing now updates visible node dimensions (no phantom-only resize behavior).

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
- Updated release workflow for `v0.6.0` default tagging and manual release dispatch support.
- Updated README run instructions with the recommended `./tauri-ui` launcher flow.

### UI behavior
- Moved settings trigger to the right-side panel header.
- Tuned chat panel sizing and typography so history remains scrollable and input stays visible.
- Added resize handles for node components and fixed CSS so resized wrappers are reflected visually.

## Verification

- `python3 -m pytest chatbot/tests/`: passing on focused suites used during release prep
- `npm --prefix ui run build`: passed

## Release workflow

Pushing tag `v0.6.0` (or running workflow dispatch with `release_version=0.6.0`) triggers [.github/workflows/build-release.yml](.github/workflows/build-release.yml), which builds Linux, macOS, and Windows artifacts and publishes a GitHub release using this file as the release body.

## Links

- [Full Changelog](https://github.com/PrakyathPNayak/ambara/compare/v0.5.0...v0.6.0)
- [Documentation](https://github.com/PrakyathPNayak/ambara#readme)
- [Report Issues](https://github.com/PrakyathPNayak/ambara/issues)
