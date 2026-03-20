# Research Synthesis: Designing Ambara's Agentic Graph Generation Pipeline

## Problem Statement

The current Ambara chatbot pipeline asks a single LLM call to:
1. Understand the user's intent
2. Select appropriate filters from 75 options
3. Determine correct connection topology (linear, branched, batch)
4. Output valid JSON with exact port names, types, and parameters

This is too much for a small model (qwen3:8b) to do in one shot. The prompt
dumps raw JSON metadata for all retrieved filters, flooding the context window
with unstructured data. Common failures:

- **Wrong/hallucinated port names**: The model invents port names not in the
  filter definition
- **Missing connections**: Nodes are placed but not properly connected
- **Wrong topology**: Batch operations mixed with single-image operations
- **Irrelevant filters**: Model picks filters that don't match the user's intent
- **Invalid parameters**: Wrong types or missing required parameters

## Key Lessons from Research Papers

| Paper | Key Insight | Application |
|-------|-------------|-------------|
| ReAct | Interleave reasoning and action | Multi-step graph building with verification |
| Chain-of-Thought | Step-by-step decomposition | Plan before building |
| HuggingGPT | 4-stage pipeline (Plan→Select→Execute→Summarize) | Decompose into Plan→Select→Connect→Parameterize |
| Toolformer | Minimal tool call syntax | Simple intermediate representation |
| Gorilla | Structured API cards + retrieval | Compact filter descriptions |
| TaskWeaver | Code-first, filters as functions | Function-call intermediate format |
| ToolBench | In-context demos, strict format, regulation | Essential for qwen3:8b |

## Design: Multi-Stage Agentic Pipeline

### Architecture Overview

```
User Query
    │
    ▼
┌──────────────────────────────────────┐
│  Stage 1: PLAN                       │
│  LLM decomposes query into ordered   │
│  processing steps (natural language)  │
│  Output: ["load images", "stack",    │
│   "remove hot pixels", "stretch",    │
│   "save result"]                     │
└──────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────┐
│  Stage 2: SELECT                     │
│  For each step, retrieve candidate   │
│  filters and LLM picks the best one  │
│  with parameters.                    │
│  Output: [{filter_id, params}, ...]  │
└──────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────┐
│  Stage 3: CONNECT (Deterministic)    │
│  Wire filters together using         │
│  port-type compatibility rules.      │
│  Handle linear, batch, branch.       │
│  Output: Complete graph JSON         │
└──────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────┐
│  Stage 4: VALIDATE + REPAIR          │
│  Run existing validator. If errors,  │
│  targeted LLM repair of specific     │
│  issues (not full regeneration).     │
│  Output: Valid graph JSON            │
└──────────────────────────────────────┘
```

### Stage 1: Plan (LLM)

**Goal**: Decompose user request into ordered processing steps.

**Prompt design**:
- System: "You are a pipeline planner for image processing. Break down the
  user's request into ordered steps. Each step should be a single operation.
  Output a JSON array of step descriptions."
- Provide 3-5 planning examples as few-shot demonstrations
- Include the COMPLETE list of filter categories and names (compact format):
  ```
  Available operations: Input(load_image, load_folder), 
  Output(save_image, batch_save_images),
  Blur(gaussian_blur, box_blur), Color(brightness, contrast, saturation, ...),
  Transform(resize, rotate, flip, crop), Composite(blend, overlay),
  Astro(image_stack, dark_frame_subtract, flat_field_correct, hot_pixel_removal, histogram_stretch),
  Batch(batch_brightness, batch_resize, ...)
  ```
- User: The query text

**Output format**:
```json
{
  "reasoning": "The user wants to process astrophotography images...",
  "topology": "linear|batch|branch",
  "steps": [
    {"description": "Load images from folder", "category_hint": "Input"},
    {"description": "Stack images", "category_hint": "Astro"},
    ...
  ]
}
```

### Stage 2: Select (Retrieval + LLM)

**Goal**: For each planned step, select the best filter and set parameters.

For each step:
1. Use semantic retrieval to find top-3 candidate filters
2. Format candidates as compact "filter cards":
   ```
   gaussian_blur:
     inputs: image(Image)
     outputs: image(Image)
     params: sigma(Float, default=2.5), kernel_size(Integer, default=3)
     desc: Apply Gaussian blur to smooth image
   ```
3. Ask LLM to pick the best filter and set parameter values
4. This is a simple, focused task the LLM can handle reliably

**Output**: Ordered list of `{filter_id, parameters}` tuples.

### Stage 3: Connect (Deterministic)

**Goal**: Wire selected filters into a valid graph JSON. This is 100% code,
no LLM needed.

**Algorithm**:
1. Create nodes with unique IDs and layout positions
2. For each consecutive pair of nodes, find compatible output→input port pairs
3. Handle special cases:
   - **Batch**: images→images port connections
   - **Branch**: Multiple source nodes feeding into blend/overlay
   - **Fan-out**: One node's output going to multiple destinations
4. Apply type-checking: only connect ports with compatible types

### Stage 4: Validate + Targeted Repair

**Goal**: Run validation and fix specific issues.

Use existing `GraphValidator.validate_all()`. If errors:
- Parse error messages to identify specific broken connections/nodes
- Generate targeted repair prompt for just those issues
- Maximum 2 repair iterations

## Why This Design Works for qwen3:8b

1. **Each LLM call is simple and focused**: Plan in one call, select per step
   in another. No single call needs to understand the entire graph.
2. **Compact context**: Each call gets only the information it needs. The
   planner gets category names, the selector gets 3 filter cards.
3. **Deterministic wiring**: Connection logic is code, not LLM output. This
   eliminates the biggest source of errors (wrong port names, missing edges).
4. **Structured output**: Each LLM output is a small JSON object, not a 
   complex nested graph.
5. **Few-shot exemplars**: Each stage has focused examples showing exactly
   what's expected.
