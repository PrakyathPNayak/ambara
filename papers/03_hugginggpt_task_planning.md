# HuggingGPT: Solving AI Tasks with ChatGPT and its Friends in Hugging Face

**Authors:** Yongliang Shen, Kaitao Song, Xu Tan, Dongsheng Li, Weiming Lu, Yueting Zhuang
**Published:** NeurIPS 2023 | arXiv:2303.17580
**URL:** https://arxiv.org/abs/2303.17580

## Abstract

HuggingGPT presents an LLM-powered agent that uses ChatGPT to conduct task
planning when receiving a user request, select models according to their function
descriptions, execute each subtask with the selected model, and summarize the
response according to the execution results. The key insight is using LLMs as a
controller to manage existing AI models (tools) through a 4-stage pipeline.

## Key Contributions — The 4-Stage Pipeline

1. **Task Planning**: LLM parses user request into structured subtasks with
   dependencies. Each subtask has: task type, model requirements, and
   input/output specification.

2. **Model Selection**: For each subtask, select the best model/tool from the
   available catalog based on capability descriptions and resource constraints.

3. **Task Execution**: Execute selected models/tools in dependency order,
   passing outputs between steps.

4. **Response Generation**: Summarize results for the user.

## Critical Design Decisions

- **Structured task representation**: Each subtask is a JSON object with
  `task_id`, `task_type`, `dependencies`, `input`, `output`.
- **Dependency graph**: Tasks form a DAG, enabling parallel execution of
  independent subtasks.
- **Tool descriptions are compact**: Each tool has name, description,
  input_type, output_type — not full API specs.

## Applicability to Ambara Graph Generation

**This is the most directly applicable paper.** Ambara's graph generation is
essentially HuggingGPT's pipeline applied to image processing:

- **Stage 1 (Plan)**: Decompose "process dim astrophotography images" into
  subtasks: [load_folder, stack, dark_frame_subtract, hot_pixel_removal,
  histogram_stretch, save].
- **Stage 2 (Select)**: For each subtask, select the best Ambara filter using
  semantic retrieval + compact tool descriptions.
- **Stage 3 (Connect)**: Wire the selected filters together using port-type
  compatibility rules (this can be deterministic!).
- **Stage 4 (Parameterize)**: Set parameter values based on the user query
  context.

The key difference: Ambara doesn't need to execute each subtask independently
— it builds a graph that the Rust engine executes as a whole. So Stages 1-4
are all *graph construction* steps.
