# Gorilla: Large Language Model Connected with Massive APIs

**Authors:** Shishir G. Patil, Tianjun Zhang, Xin Wang, Joseph E. Gonzalez
**Published:** arXiv:2305.15334
**URL:** https://arxiv.org/abs/2305.15334

## Abstract

Gorilla is a finetuned LLaMA-based model that surpasses GPT-4 on writing API
calls. When combined with a document retriever, Gorilla demonstrates a strong
capability to adapt to test-time document changes, reducing hallucination.

## Key Contributions

1. **Retrieval-Augmented API Generation**: Combining retrieval with generation
   dramatically reduces hallucinated API calls (wrong names, wrong arguments).

2. **APIBench**: A benchmark for evaluating API call generation quality across
   HuggingFace, TorchHub, and TensorHub APIs.

3. **Document Retriever Integration**: At generation time, relevant API
   documentation is retrieved and injected into the prompt, ensuring the model
   has accurate, up-to-date information.

4. **Structured API Descriptions Matter**: The format of API documentation
   significantly affects generation quality. Structured formats (name, args,
   return type, description) outperform free-text descriptions.

## Applicability to Ambara Graph Generation

- **Retriever-augmented generation is critical**: The current pipeline already
  uses ChromaDB retrieval, but the retrieved data format needs improvement.
- **Structured filter cards**: Each filter should be presented as a compact,
  structured "API card" with name, input_ports (name:type), output_ports
  (name:type), parameters (name:type:default), and a one-line description.
- **Hallucination prevention**: By providing exact port names and types in the
  prompt, the model is less likely to invent non-existent ports.
