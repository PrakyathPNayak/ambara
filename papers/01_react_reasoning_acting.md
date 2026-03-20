# ReAct: Synergizing Reasoning and Acting in Language Models

**Authors:** Shunyu Yao, Jeffrey Zhao, Dian Yu, Nan Du, Izhak Shafran, Karthik Narasimhan, Yuan Cao
**Published:** ICLR 2023 | arXiv:2210.03629
**URL:** https://arxiv.org/abs/2210.03629

## Abstract

While LLMs have demonstrated impressive capabilities across tasks in language
understanding and interactive decision making, their abilities for reasoning
(e.g. chain-of-thought prompting) and acting (e.g. action plan generation) have
primarily been studied as separate topics. ReAct explores the use of LLMs to
generate both reasoning traces and task-specific actions in an interleaved
manner, allowing for greater synergy between the two: reasoning traces help the
model induce, track, and update action plans as well as handle exceptions, while
actions allow it to interface with external sources to gather additional
information.

## Key Contributions

1. **Interleaved Reasoning + Acting**: Instead of generating a full plan upfront,
   the model alternates between "Thought" (reasoning) and "Action" (tool call)
   steps. This prevents error accumulation from pure chain-of-thought.

2. **Grounded Reasoning**: Actions (API calls, lookups) provide real data that
   the model can reason over, reducing hallucination.

3. **Error Recovery**: When an action returns unexpected results, the model can
   reason about what went wrong and try an alternative action.

4. **Generalizable Pattern**: The Thought → Action → Observation loop applies
   to any domain where external tools/environments are available.

## Applicability to Ambara Graph Generation

- **Multi-step graph building**: Instead of generating the entire graph in one
  shot, use a ReAct loop: reason about what the user needs → select a filter
  → verify its ports → connect it → reason about what's next.
- **Error recovery**: If a connection fails validation, the agent can reason
  about why and select an alternative filter or port.
- **Grounding**: Each step's "observation" is the current partial graph state,
  giving the LLM concrete data to reason over rather than hallucinating
  connections.
