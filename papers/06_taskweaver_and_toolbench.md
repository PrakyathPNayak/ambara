# TaskWeaver: A Code-First Agent Framework

**Authors:** Bo Qiao, Liqun Li, Xu Zhang, et al. (Microsoft Research)
**Published:** arXiv:2311.17541
**URL:** https://arxiv.org/abs/2311.17541

## Abstract

TaskWeaver is a code-first framework for building LLM-powered autonomous agents.
It converts user requests into executable code, treats user-defined plugins as
callable functions, supports rich data structures, flexible plugin usage, and
dynamic plugin selection.

## Key Contributions

1. **Code-First Approach**: Instead of generating natural language plans, the
   LLM generates executable code that calls plugins (tools). This grounds
   outputs in something parseable and verifiable.

2. **Plugin as Function**: Each tool/plugin is described as a function with
   typed parameters and return values. The LLM generates function calls.

3. **Dynamic Plugin Selection**: Not all plugins are in the prompt at once.
   The framework selects relevant plugins based on the task context.

4. **Domain Knowledge Injection**: Through exemplars and system prompts, domain
   knowledge guides the agent's code generation.

5. **Execution verification**: Generated code is actually executed, and errors
   feed back into the agent for self-correction.

## Applicability to Ambara Graph Generation

- **Filters as typed functions**: Present each Ambara filter as a typed function
  signature rather than a JSON blob. E.g.:
  `gaussian_blur(image: Image, sigma: Float=2.5) -> Image`
- **Code-like intermediate format**: Instead of asking the LLM to produce raw
  graph JSON, have it produce a simpler intermediate representation:
  ```
  img = load_image(path="input.png")
  blurred = gaussian_blur(img, sigma=3.0)
  save_image(blurred, path="output.png")
  ```
  This is much easier for small models to generate correctly, and can be
  deterministically compiled into the graph JSON format.
- **Dynamic filter injection**: Only include filter descriptions relevant to
  the current query, not all 75 filters.

# On the Tool Manipulation Capability of Open-source Large Language Models

**Authors:** Qiantong Xu, Fenglu Hong, Bo Li, Changran Hu, Zhengyu Chen, Jian Zhang
**Published:** arXiv:2305.16504
**URL:** https://arxiv.org/abs/2305.16504

## Abstract

This paper demonstrates that open-source LLMs can be enhanced to be competitive
with GPT-4 in tool manipulation through: usage examples, in-context
demonstrations, and generation style regulation.

## Key Contributions

1. **Failure Analysis**: Common tool manipulation failures in open-source LLMs:
   - Wrong tool selection
   - Incorrect argument types/values
   - Missing required arguments
   - Hallucinated tool names

2. **Mitigation Techniques**:
   - **In-context demonstrations**: 3-5 worked examples per tool pattern
   - **System prompts**: Strict output format instructions
   - **Generation style regulation**: Forcing structured output (JSON/code)

3. **ToolBench**: A benchmark showing these techniques boost success rates by
   up to 90% on open-source models.

## Applicability to Ambara Graph Generation

- **Critical for qwen3:8b**: As a smaller open-source model, qwen3:8b needs
  all three mitigation techniques: in-context demos, strict format, and
  generation regulation.
- **Output format enforcement**: Use a very strict, simple output format
  that the model can reliably produce (e.g., line-by-line function calls
  rather than nested JSON).
- **Error pattern awareness**: Anticipate and handle the specific failure modes
  (wrong filter_id, wrong port names, wrong parameter types).
