# Chain-of-Thought Prompting Elicits Reasoning in Large Language Models

**Authors:** Jason Wei, Xuezhi Wang, Dale Schuurmans, Maarten Bosma, Brian Ichter, Fei Xia, Ed Chi, Quoc Le, Denny Zhou
**Published:** NeurIPS 2022 | arXiv:2201.11903
**URL:** https://arxiv.org/abs/2201.11903

## Abstract

Generating a chain of thought — a series of intermediate reasoning steps —
significantly improves the ability of large language models to perform complex
reasoning. Chain-of-thought prompting, where a few chain-of-thought
demonstrations are provided as exemplars, improves performance on arithmetic,
commonsense, and symbolic reasoning tasks.

## Key Contributions

1. **Step-by-Step Decomposition**: Complex problems are broken into intermediate
   steps, each of which is simpler to solve.

2. **Few-Shot Exemplars**: Providing 2-8 worked examples with reasoning chains
   is sufficient to guide the model.

3. **Emergent Capability**: CoT reasoning emerges in models above ~100B
   parameters but can be induced in smaller models with careful prompting.

4. **Transferable Pattern**: The "think step by step" instruction works across
   diverse domains.

## Applicability to Ambara Graph Generation

- **Structured planning prompt**: Ask the LLM to first enumerate the processing
  steps needed ("Step 1: Load images from folder. Step 2: Stack them. Step 3:
  Remove hot pixels...") before generating any JSON.
- **Smaller model compensation**: Since qwen3:8b is a smaller model, CoT
  prompting is essential to get structured outputs. The model needs to "think
  through" the filter chain before assembling JSON.
- **Few-shot with reasoning**: Include examples that show the reasoning process,
  not just input→output pairs.
