"""Stage 1 – Plan: decompose a user query into ordered processing steps.

Uses a focused LLM call with a compact filter catalog so that even small
models (qwen3:8b) can reliably produce a structured plan.
"""

from __future__ import annotations

import json
import logging
import re
from typing import Any

LOGGER = logging.getLogger(__name__)

# Compact catalog organized by category.  Only names/IDs – no port metadata,
# keeping the prompt small for the planning stage.
FILTER_CATALOG = """\
Input: load_image (load a single image file), load_folder (load all images from a directory for batch processing)
Output: save_image (save single image to file), batch_save_images (save array of processed images)
Blur: gaussian_blur (smooth image with Gaussian kernel), box_blur (smooth image with box/average kernel)
Color: brightness (adjust brightness), contrast (adjust contrast), saturation (adjust color saturation), grayscale (convert to grayscale), invert (invert colors), sepia (apply warm sepia tone), hue_rotate (rotate pixel hue by angle), threshold (binary black/white threshold), posterize (reduce color levels)
Adjust: gamma (gamma correction for luminance), color_balance (adjust RGB channels independently)
Transform: resize (resize image dimensions), rotate (rotate image by degrees), flip (flip image horizontally/vertically), crop (crop image region)
Sharpen: unsharp_mask (classic unsharp mask sharpening with sigma/amount/threshold), sharpen (simple 3x3 kernel sharpening)
Edge: edge_detect (Sobel/Prewitt edge detection), emboss (directional relief/emboss effect)
Noise: add_noise (add Gaussian or salt-and-pepper noise), denoise (median filter noise reduction)
Draw: draw_rectangle (draw filled/outline rectangle), draw_circle (draw filled/outline circle), draw_line (draw line between two points)
Text: text_overlay (add text to image with built-in bitmap font, configurable position/scale/color)
Composite: blend (blend two images together with opacity), overlay (overlay image on top of another with position)
Astro: image_stack (stack/average multiple images for noise reduction), dark_frame_subtract (subtract dark frame calibration), flat_field_correct (divide by flat field for vignette correction), hot_pixel_removal (remove hot/stuck pixels), histogram_stretch (stretch histogram to enhance faint detail)
Batch: batch_brightness, batch_contrast, batch_saturation, batch_grayscale, batch_invert, batch_resize, batch_rotate, batch_flip, batch_crop, batch_gaussian_blur (all operate on image arrays from load_folder)
Analyze: image_info (get width, height, channels, format, size info)
Utility: preview (display image preview), split_channels (split into R,G,B,A), merge_channels (merge R,G,B,A into image), collect_images (collect up to 4 images into array), value_display (display any value)
Math: add, subtract, multiply, divide, modulo, power, min, max, clamp
Comparison: equal, not_equal, less_than, greater_than, and, or, not, xor
Constants: integer_constant, float_constant, string_constant, boolean_constant, color_constant
Conversion: to_integer, to_float, to_string, to_boolean
Array: array_map, array_filter, array_concat, array_slice\
"""

PLAN_EXAMPLES = [
    {
        "query": "Load an image, blur it, and save it",
        "plan": {
            "reasoning": "Simple linear pipeline: load a single image, apply blur, save output.",
            "topology": "linear",
            "steps": [
                {"step": 1, "operation": "load_image", "description": "Load the input image"},
                {"step": 2, "operation": "gaussian_blur", "description": "Apply Gaussian blur to smooth the image"},
                {"step": 3, "operation": "save_image", "description": "Save the blurred result"},
            ],
        },
    },
    {
        "query": "Process all images in a folder: resize them to 512x512, increase brightness, and save",
        "plan": {
            "reasoning": "Batch pipeline: load folder of images, apply batch operations in sequence, save all.",
            "topology": "batch",
            "steps": [
                {"step": 1, "operation": "load_folder", "description": "Load all images from the folder"},
                {"step": 2, "operation": "batch_resize", "description": "Resize all images to 512x512"},
                {"step": 3, "operation": "batch_brightness", "description": "Increase brightness of all images"},
                {"step": 4, "operation": "batch_save_images", "description": "Save all processed images"},
            ],
        },
    },
    {
        "query": "Blend two images together with 50% opacity and save the result",
        "plan": {
            "reasoning": "Branch/merge pipeline: load two separate images, blend them together, save output.",
            "topology": "branch",
            "steps": [
                {"step": 1, "operation": "load_image", "description": "Load the first/base image"},
                {"step": 2, "operation": "load_image", "description": "Load the second/overlay image"},
                {"step": 3, "operation": "blend", "description": "Blend the two images with 50% opacity"},
                {"step": 4, "operation": "save_image", "description": "Save the blended result"},
            ],
        },
    },
    {
        "query": "Build a complex pipeline to process dim astrophotography images with extensive processing",
        "plan": {
            "reasoning": "Astrophotography pipeline: load folder of light frames, stack to reduce noise (produces single image), apply calibration corrections, enhance faint details, save result.",
            "topology": "linear",
            "steps": [
                {"step": 1, "operation": "load_folder", "description": "Load all light frame images from the directory"},
                {"step": 2, "operation": "image_stack", "description": "Stack/average all frames to reduce noise"},
                {"step": 3, "operation": "hot_pixel_removal", "description": "Remove hot and stuck pixels from the stacked image"},
                {"step": 4, "operation": "histogram_stretch", "description": "Stretch histogram to bring out faint nebula and star detail"},
                {"step": 5, "operation": "contrast", "description": "Enhance contrast to make details pop"},
                {"step": 6, "operation": "save_image", "description": "Save the processed astrophotography image"},
            ],
        },
    },
    {
        "query": "Overlay a watermark image on top of a photo and save",
        "plan": {
            "reasoning": "Branch/merge pipeline: load the base photo and the watermark image, overlay the watermark on top, save result.",
            "topology": "branch",
            "steps": [
                {"step": 1, "operation": "load_image", "description": "Load the base photo"},
                {"step": 2, "operation": "load_image", "description": "Load the watermark image"},
                {"step": 3, "operation": "overlay", "description": "Overlay watermark on top of the photo"},
                {"step": 4, "operation": "save_image", "description": "Save the watermarked result"},
            ],
        },
    },
]


def build_plan_prompt(query: str, catalog_override: str | None = None) -> dict[str, list[dict[str, str]]]:
    """Build the prompt for the planning stage.

    Args:
        query: User's natural language request.
        catalog_override: Optional dynamic catalog string from CodeRetriever.
            If None, uses the static FILTER_CATALOG.

    Returns:
        OpenAI/Anthropic-style messages dict.
    """
    catalog = catalog_override or FILTER_CATALOG
    examples_text = "\n\n".join(
        f'Query: "{ex["query"]}"\nPlan:\n```json\n{json.dumps(ex["plan"], indent=2)}\n```'
        for ex in PLAN_EXAMPLES
    )

    system = (
        "You are a pipeline planner for Ambara, an image processing application. "
        "Your job is to break down the user's image processing request into an ordered "
        "list of concrete processing steps.\n\n"
        "AVAILABLE OPERATIONS BY CATEGORY:\n"
        f"{catalog}\n\n"
        "RULES:\n"
        "1. Every pipeline must start with an input operation (load_image or load_folder).\n"
        "2. Every pipeline must end with an output operation (save_image or batch_save_images).\n"
        "3. If the request involves multiple images from a folder, use load_folder and batch_* operations.\n"
        "4. If the request involves blending/compositing two images, use two load_image steps and blend/overlay.\n"
        "5. Each step should use exactly one operation from the available list.\n"
        "6. Set topology to 'linear' for simple chains, 'batch' for folder processing, 'branch' for compositing.\n"
        "7. Output ONLY valid JSON matching the format shown in examples. No markdown fences, no extra text.\n\n"
        f"EXAMPLES:\n\n{examples_text}"
    )

    user = f'Plan the pipeline for this request: "{query}"'

    return {
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ]
    }


def parse_plan(raw: str) -> dict[str, Any] | None:
    """Parse LLM output into a structured plan.

    Attempts JSON parsing with several fallback strategies for common LLM
    output quirks (markdown fences, trailing text, etc.).

    Args:
        raw: Raw LLM text output.

    Returns:
        Parsed plan dict or None if parsing fails.
    """
    text = raw.strip()

    # Strip qwen3 <think> reasoning tags.
    text = re.sub(r"<think>.*?</think>", "", text, flags=re.DOTALL).strip()

    # Strip markdown code fences if present.
    fence_match = re.search(r"```(?:json)?\s*\n?(.*?)```", text, re.DOTALL)
    if fence_match:
        text = fence_match.group(1).strip()

    # Strip any leading/trailing non-JSON text.
    brace_start = text.find("{")
    if brace_start > 0:
        text = text[brace_start:]
    brace_end = text.rfind("}")
    if brace_end >= 0:
        text = text[: brace_end + 1]

    try:
        plan = json.loads(text)
    except json.JSONDecodeError:
        LOGGER.warning("Failed to parse plan JSON: %s", text[:200])
        return None

    # Validate structure.
    if not isinstance(plan, dict):
        return None
    if "steps" not in plan or not isinstance(plan["steps"], list):
        return None
    if not plan["steps"]:
        return None

    # Normalize step format.
    for step in plan["steps"]:
        if "operation" not in step:
            # Try to infer from description.
            step["operation"] = step.get("filter_id", step.get("tool", ""))
        if "description" not in step:
            step["description"] = step.get("operation", "")

    plan.setdefault("topology", "linear")
    plan.setdefault("reasoning", "")
    return plan
