# Toolformer: Language Models Can Teach Themselves to Use Tools

**Authors:** Timo Schick, Jane Dwivedi-Yu, Roberto Dessì, et al.
**Published:** NeurIPS 2023 | arXiv:2302.04761
**URL:** https://arxiv.org/abs/2302.04761

## Abstract

Toolformer shows that LMs can teach themselves to use external tools via simple
APIs. The model learns to decide which APIs to call, when to call them, what
arguments to pass, and how to incorporate results into future predictions. This
is done in a self-supervised way, requiring only a handful of demonstrations
per API.

## Key Contributions

1. **API Call Syntax**: Tools are invoked via inline text markers:
   `[Calculator(3+5)→8]`. The model learns to produce and consume these.

2. **Self-Supervised Training**: The model generates potential API calls,
   filters by whether they improve prediction, and trains on the filtered set.

3. **Minimal Demonstrations**: Only 5-10 examples per tool type are needed.

4. **Tool Selection is Learned**: The model implicitly learns WHEN a tool is
   needed and which tool to use.

## Applicability to Ambara Graph Generation

- **Structured tool descriptions**: Each Ambara filter should be described as a
  callable tool with clear input/output types, not just a blob of JSON metadata.
- **Few demonstrations per pattern**: Instead of dumping all 74 filters, provide
  concise descriptions and a few usage examples per common pattern.
- **Tool call format**: The LLM should "call" filters in a structured format
  that can be parsed deterministically, rather than producing free-form JSON.
