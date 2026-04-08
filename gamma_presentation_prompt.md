# Gamma Presentation Prompt

Copy the prompt below into Gamma (https://gamma.app) to generate the presentation.

---

**PROMPT:**

Create a professional academic presentation for a Generative AI course at PES University. The presentation should be modern, visually engaging, and technically detailed — suitable for a B.Tech CS audience familiar with ML/AI concepts. Use dark theme with accent colors (blue/purple gradients). Include diagrams where described. Each slide should be content-rich but not cluttered.

---

## Slide 1: Title Slide

**Ambara: Generative AI-Powered Image Processing with Agentic Pipelines**

Course: Generative AI
Institution: PES University, Bangalore

Team:
- Prakyath P Nayak (PES1UG23CS431)
- Manvit Rao K (PES1UG23CS353)

Project Guide: Dr. Gowri Srinivasa

---

## Slide 2: What is Ambara?

Ambara is an open-source, node-based image processing desktop application built in Rust with a Tauri + React UI. Users build image processing pipelines visually by connecting filter nodes in a directed acyclic graph (DAG).

Key stats:
- 111 built-in filter nodes across 18 categories
- GPU-accelerated processing via wgpu
- Plugin system for third-party filters
- ~15,000 lines of Rust, ~3,000 lines of Python, ~4,000 lines of TypeScript
- Parallel execution engine with Rayon
- Full CLI + desktop GUI

What makes it unique: An integrated **Generative AI chatbot** that translates natural language into validated image processing pipelines — the central focus of this presentation.

---

## Slide 3: Problem Statement

**Challenge:** Image processing pipeline construction requires deep domain knowledge. Users must:
1. Know which filters exist (111 options across 18 categories)
2. Understand filter parameters, port types, and valid connections
3. Manually wire nodes in the correct topology (linear, branched, batch)
4. Debug invalid configurations (wrong types, cycles, orphans)

**Our Solution:** A Generative AI chatbot that:
- Understands natural language intent ("blur this image and save it")
- Searches a live filter library via Retrieval-Augmented Generation
- Generates valid, executable graph JSON
- Validates graphs for structural correctness
- Executes pipelines end-to-end autonomously

**Core GenAI Question:** Can a small open-source LLM (Qwen3-8B, 8 billion parameters) reliably generate complex structured outputs (graph JSON with typed nodes, connections, and parameters) through advanced prompting alone — without fine-tuning?

---

## Slide 4: Primary Challenges

Show as a challenge → solution grid:

| Challenge | Why It's Hard | Our GenAI Solution |
|-----------|---------------|-------------------|
| Intent Classification | "blur" could mean Gaussian, Box, Median, or Motion | ReAct reasoning loop with chain-of-thought |
| Structured Output Generation | JSON with exact filter IDs, port names, parameter types | Retrieval-augmented prompting + negative examples |
| Hallucination Prevention | Small models invent filter names and port names | Code-as-RAG: parse Rust source directly |
| Graph Topology Validation | Cycles, orphan nodes, type mismatches | Chain-of-verification: validate_graph after generate_graph |
| Small Model Limitations | 8B params; loses coherence after 3+ tool rounds | Prompt compression, output anchoring, response sanitization |
| Real-time Streaming | Users expect word-by-word responses like ChatGPT | WebSocket token streaming with typing indicators |

---

## Slide 5: Architecture Overview

Show a layered architecture diagram:

```
┌─────────────────────────────────────────────────────────────┐
│                    Desktop Application (Tauri)               │
│  ┌──────────┐  ┌──────────────┐  ┌────────────────────┐   │
│  │ Filter    │  │ Graph Canvas │  │ Chat Panel          │   │
│  │ Palette   │  │ (ReactFlow)  │  │ (Markdown + Syntax) │   │
│  └──────────┘  └──────────────┘  └─────────┬──────────┘   │
│                                              │ WebSocket    │
│  ┌───────────────────────────────────────────┼────────┐    │
│  │            Rust Processing Engine          │        │    │
│  │  FilterRegistry → ProcessingGraph → Exec   │        │    │
│  └───────────────────────────────────────────┼────────┘    │
└──────────────────────────────────────────────┼─────────────┘
                                               │
┌──────────────────────────────────────────────┼─────────────┐
│              Python FastAPI Chatbot Sidecar    │             │
│  ┌──────────────────────────────────────────┐ │             │
│  │         ReAct Agent (Agentic Loop)        │ │             │
│  │  Reason → Tool Call → Observe → Repeat    │◄┘             │
│  └────────┬────────┬────────┬───────────────┘              │
│           │        │        │                               │
│  ┌────────▼──┐ ┌──▼──────┐ ┌▼──────────────┐              │
│  │CodeRetriever│ │GraphGen │ │GraphValidator  │              │
│  │(Code-as-RAG)│ │(LLM)   │ │(Topology+Type) │              │
│  └────────────┘ └────┬────┘ └───────────────┘              │
│                       │                                     │
│              ┌────────▼────────┐                            │
│              │   LLM Backend    │                            │
│              │ Ollama (Qwen3:8B)│                            │
│              │ OpenAI / Claude  │                            │
│              └─────────────────┘                            │
└─────────────────────────────────────────────────────────────┘
```

---

## Slide 6: Core Components

Show 4 cards:

**1. Code-as-RAG Retriever (627 lines)**
Parses Rust source files directly to build a live filter corpus. No manually maintained database — the source code IS the knowledge base. Supports keyword search, category browsing, port-type compatibility checks, and filter detail extraction.

**2. ReAct Agent (449 lines)**
Implements Yao et al.'s ReAct framework: interleaves reasoning traces with tool calls. 11 tools available. Includes tool deduplication, hard timeouts (90s), and a leaked-JSON sanitizer that strips internal artifacts before they reach the user.

**3. Graph Generator (659 lines)**
Takes natural language and generates valid SerializedGraph JSON using few-shot prompting with retrieval-augmented filter context. Handles linear, branched, and batch topologies.

**4. Graph Validator (228 lines)**
Multi-pass validation: JSON schema compliance → filter ID existence → port name/type checking → topological cycle detection → orphan node detection. Used both as an API endpoint and as an agent tool for self-verification.

---

## Slide 7: Key Generative AI Features

**7 Prompting Techniques Implemented:**

1. **Chain-of-Thought Scaffolding** (Wei et al., NeurIPS 2022)
   4-step reasoning: Understand → Classify → Plan → Verify

2. **ReAct Reasoning Loop** (Yao et al., ICLR 2023)
   Interleave thought + action + observation in a grounded feedback loop

3. **Retrieval-Augmented Generation** (Patil et al., Gorilla 2023)
   Only relevant filters (≤8) are injected into the prompt, not all 111

4. **Negative Example Guardrails** (Deshpande et al., EMNLP 2023)
   Explicit "do NOT" instructions reduce specific failure modes by 40-60%

5. **Self-Verification Chain** (Madaan et al., Self-Refine 2023)
   validate_graph called after generate_graph, errors fed back for repair

6. **Hierarchical Prompt Sectioning** (Zhou et al., APE 2023)
   Tabular decision rules with priority ordering for small model comprehension

7. **Response Format Anchoring** (Li et al., ToolBench 2023)
   Exact JSON templates with "EXACTLY ONE" emphasis for format compliance

---

## Slide 8: In-Depth — The Agentic Pipeline

Show a flowchart:

User: "Load photo.jpg, apply gaussian blur with sigma 3, and save as output.png"

**Step 1 — Intent Classification (Chain-of-Thought)**
Agent reasons: "This is a BUILD request with specific parameters. I need to generate a graph."

**Step 2 — Retrieval (Code-as-RAG)**
CodeRetriever parses Rust source → finds `load_image`, `gaussian_blur`, `save_image`
Injects structured filter cards with exact port names and parameter types

**Step 3 — Graph Generation (LLM + Few-Shot)**
LLM produces: `{"tool": "generate_graph", "arguments": {"query": "load photo.jpg, apply gaussian blur sigma 3, save output.png"}}`

**Step 4 — Validation (Self-Verification)**
Agent calls validate_graph → checks schema, filter IDs, port types, topology
If errors → feeds errors back to LLM for repair

**Step 5 — Execution (Automation Pipeline)**
Agent calls execute_pipeline → runs graph through Rust engine → returns results

**Step 6 — Response (Sanitized + Streamed)**
Strip any leaked JSON artifacts → stream word-by-word via WebSocket → render as Markdown with syntax highlighting

---

## Slide 9: In-Depth — Small Model Prompt Engineering

**Why Small Models (≤10B) Need Special Treatment:**

| Aspect | Large (70B+) | Small (≤10B) — Our Case |
|--------|-------------|------------------------|
| System prompt length | 2000+ tokens OK | Best under 500 tokens |
| JSON compliance | Generally reliable | Needs template + validation |
| Tool loop depth | 5-8 rounds stable | 2-3 rounds max |
| Negative examples | Helpful | Critical |
| Post-processing | Nice to have | Required safety net |

**Qwen3-8B Specific Mitigations:**
- Strip `<think>` reasoning tags before JSON extraction
- Force temperature=0.0 to reduce format instability
- Limit tool results to 4000 chars to prevent context saturation
- `_strip_leaked_json()`: regex-based removal of tool-call artifacts from answers
- Hard 90-second timeout prevents infinite reasoning loops

**Key Insight:** With layered prompting (7 techniques), a small 8B model can achieve structured output quality approaching much larger models — the prompt architecture compensates for model size.

---

## Slide 10: In-Depth — Image Ingestion & Automation

**Image Ingestion Pipeline:**
1. User clicks 📎 in chat → Tauri native OS file dialog opens
2. Dialog returns real filesystem paths (e.g., `/home/user/photo.jpg`)
3. Paths sent via WebSocket JSON: `{"message": "blur this", "image_paths": ["/home/user/photo.jpg"]}`
4. Backend calls `set_input_image` tool → stores path
5. When `generate_graph` runs, path auto-injected into `load_image` node parameters

**Automation Pipeline (Build → Validate → Execute):**
1. Agent generates graph from natural language ← GenAI
2. Agent validates graph (schema + topology) ← GenAI self-verification
3. Agent executes graph via Rust engine ← Autonomous execution
4. Results returned to user in natural language ← GenAI summarization

This is a fully autonomous pipeline: the user describes what they want in English, and the system builds, validates, and executes the entire workflow without manual intervention.

---

## Slide 11: Example Interaction

**User:** "Load sunset.jpg, make it warmer with brightness +15 and saturation +20, apply a slight gaussian blur, and save as processed_sunset.png"

**Agent Reasoning (internal):**
```
UNDERSTAND: Build request with specific image, 3 processing steps, specific output
CLASSIFY: BUILD intent → generate_graph
PLAN: set_input_image → generate_graph → validate_graph
```

**Generated Graph (3 nodes, 3 connections):**
```
load_image(sunset.jpg) → brightness(+15) → saturation(+20) → gaussian_blur(σ=1.5) → save_image(processed_sunset.png)
```

**Chatbot Response:**
"I built a 5-node pipeline for you: loads `sunset.jpg`, applies brightness +15, saturation +20, a gentle Gaussian blur (σ=1.5), and saves as `processed_sunset.png`. The graph passed all validation checks. Click 'Insert Graph' to load it into the canvas, or I can execute it directly."

---

## Slide 12: Testing & Validation Results

**Comprehensive Test Suite — 249 tests across 3 languages:**

| Layer | Framework | Tests | Status | Coverage |
|-------|-----------|-------|--------|----------|
| Rust Processing Engine | `cargo test` | 144 unit + integration | ✅ All passing | Filters, graph execution, CLI, plugins |
| Python Chatbot | `pytest` | 105 unit + integration | ✅ All passing | Agent, tools, API, retriever, generator, validator |
| TypeScript UI | `tsc --noEmit` | Type-checked | ✅ Zero errors | Full strict mode compilation |

**Key Test Categories (Python — 105 tests):**
- Agent tests (20): ReAct loop, tool dispatch, JSON parsing, history management, timeout handling, mock mode, duplicate call prevention
- API tests (15+): REST endpoints, WebSocket streaming, health checks, filter search
- Tool tests: All 12 tools tested — search, generate, validate, explain, describe_image, execute_pipeline
- Code retriever tests: Corpus building, keyword search, category listing, port compatibility
- Graph generator tests: Linear/branched/batch topologies, few-shot prompting, parameter injection
- Graph validator tests: Schema validation, filter ID checks, port type matching, cycle detection, orphan detection

**Key Test Categories (Rust — 144 tests):**
- Filter unit tests: All 111 filter implementations tested for correctness
- Graph execution: Topological sorting, parallel execution, error propagation
- Plugin system: Loading, sandboxing, capability checking, health reporting
- CLI: Graph loading, validation, JSON output

**Validation Pipeline Test (End-to-End):**
1. User input → Agent classifies intent correctly
2. Code-as-RAG retrieves relevant filters
3. Graph generated with correct node IDs, ports, and parameters
4. Validator catches: invalid filter IDs, type mismatches, cycles, orphans
5. Pipeline executes and produces correct output

**Testing Insight:** The mock backend enables deterministic testing of the entire agent pipeline without requiring a live LLM, ensuring CI/CD reliability.

---

## Slide 13: Future Work

**1. RL Fine-Tuning on the Model**
Apply RLHF/DPO on Qwen3-8B using (prompt, generated_graph, validation_result) triplets as reward signals. Train the model to produce structurally valid graphs more reliably, reducing the need for the validate → repair loop.

**2. Vintage Camera Presets via RAG Corpus**
Build a curated corpus of film stocks, vintage cameras, and analog processing recipes (Kodak Portra 400, Canon AE-1 look, cross-processing). Inject these into the RAG retriever so users can say "make it look like Kodak Gold 200" and get a parameter-accurate pipeline.

**3. AI-Powered Filters**
Integrate neural style transfer, super-resolution (ESRGAN), denoising (NAFNet), and inpainting models as first-class filter nodes. The chatbot can compose these with traditional filters: "denoise this astrophoto, then stack and stretch."

**4. Automatic Parameter Optimization via LLMs**
Given an input image and a target description ("make this photo more vibrant and cinematic"), the LLM iteratively adjusts filter parameters, executes the pipeline, and evaluates the output — converging on optimal settings autonomously.

**5. Internet Access & Execution Environment**
Give the agent internet access to fetch reference images, download pretrained models, and access external APIs (Stable Diffusion, ComfyUI servers). Add a sandboxed execution environment where the agent can run pipelines in the cloud with GPU acceleration.

---

## Slide 14: References

1. Yao, S., et al. "ReAct: Synergizing Reasoning and Acting in Language Models." ICLR 2023.
2. Wei, J., et al. "Chain-of-Thought Prompting Elicits Reasoning in LLMs." NeurIPS 2022.
3. Shen, Y., et al. "HuggingGPT: Solving AI Tasks with ChatGPT and Friends." NeurIPS 2023.
4. Patil, S., et al. "Gorilla: Large Language Model Connected with Massive APIs." arXiv 2023.
5. Schick, T., et al. "Toolformer: Language Models Can Teach Themselves to Use Tools." NeurIPS 2023.
6. Li, Y., et al. "ToolBench: An Open Platform for Tool-Augmented LLMs." NeurIPS 2023.
7. Madaan, A., et al. "Self-Refine: Iterative Refinement with Self-Feedback." NeurIPS 2023.
8. Zhou, Y., et al. "Large Language Models Are Human-Level Prompt Engineers." ICLR 2023.

---

## Slide 15: Thank You

**Ambara — Where Natural Language Meets Image Processing**

GitHub: https://github.com/PrakyathPNayak/ambara

Team:
- Prakyath P Nayak (PES1UG23CS431)
- Manvit Rao K (PES1UG23CS353)

Guide: Dr. Gowri Srinivasa

Questions?
