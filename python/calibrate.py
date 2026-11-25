"""Calibration run: ask the model questions with known answers and record how unsure it was.

usage: python calibrate.py [output.csv] [count]
Defaults: calib_results.csv, 120 questions. Then, in the repo:
    cargo run --bin ch10cal -- calib_results.csv

The questions are generated, so every answer key is computed, not remembered:
multiplication and addition with fixed random seeds, plus a short list of capitals.
"""
import csv
import random
import re
import sys
import time
import unicodedata

import torch
from transformers import AutoModelForCausalLM, AutoTokenizer, LogitsProcessor, LogitsProcessorList

MODEL_NAME = "Qwen/Qwen2.5-1.5B-Instruct"
OUT = sys.argv[1] if len(sys.argv) > 1 else "calib_results.csv"
COUNT = int(sys.argv[2]) if len(sys.argv) > 2 else 120

CAPITALS = [
    ("France", "Paris"), ("Japan", "Tokyo"), ("Canada", "Ottawa"), ("Australia", "Canberra"),
    ("Brazil", "Brasilia"), ("Turkey", "Ankara"), ("Kenya", "Nairobi"), ("Egypt", "Cairo"),
    ("Spain", "Madrid"), ("Germany", "Berlin"), ("Norway", "Oslo"), ("Peru", "Lima"),
    ("Vietnam", "Hanoi"), ("Morocco", "Rabat"), ("New Zealand", "Wellington"), ("Switzerland", "Bern"),
    ("Nigeria", "Abuja"), ("Pakistan", "Islamabad"), ("Ecuador", "Quito"), ("Hungary", "Budapest"),
    ("Poland", "Warsaw"), ("Portugal", "Lisbon"), ("Thailand", "Bangkok"), ("Iceland", "Reykjavik"),
    ("Finland", "Helsinki"), ("Ghana", "Accra"), ("Laos", "Vientiane"), ("Uruguay", "Montevideo"),
    ("Tanzania", "Dodoma"), ("Ivory Coast", "Yamoussoukro"),
]


def questions(count):
    rng = random.Random(106)
    qs = []
    for i in range(count):
        kind = ("mul", "mul", "add", "cap")[i % 4]
        if kind == "mul":
            a, b = rng.randint(12, 99), rng.randint(12, 99)
            qs.append((kind, f"What is {a} times {b}? Answer with just the number.", str(a * b)))
        elif kind == "add":
            a, b = rng.randint(100, 999), rng.randint(100, 999)
            qs.append((kind, f"What is {a} plus {b}? Answer with just the number.", str(a + b)))
        else:
            country, city = CAPITALS[(i // 4) % len(CAPITALS)]
            qs.append((kind, f"What is the capital of {country}? Answer with just the city name.", city))
    return qs


def norm(text):
    text = unicodedata.normalize("NFKD", text).encode("ascii", "ignore").decode()
    return re.sub(r"[^a-z0-9]", "", text.lower())


def grade(kind, answer, expected):
    if kind in ("mul", "add"):
        nums = re.findall(r"-?\d[\d,]*", answer)
        return bool(nums) and nums[-1].replace(",", "") == expected  # last number, in case it restates the question
    return norm(expected) in norm(answer)


class Sensor(LogitsProcessor):
    """Your confidence sensor: entropy and top-token probability for every token."""

    def __init__(self):
        self.readings = []

    def __call__(self, input_ids, scores):
        log_probs = torch.log_softmax(scores[0].float(), dim=-1)
        probs = log_probs.exp()
        terms = torch.where(probs > 0, probs * log_probs, torch.zeros_like(probs))
        self.readings.append((-terms.sum().item(), probs.max().item()))
        return scores


tokenizer = AutoTokenizer.from_pretrained(MODEL_NAME)
model = AutoModelForCausalLM.from_pretrained(MODEL_NAME, dtype=torch.bfloat16, device_map="cuda")

rows = []
start = time.time()
for i, (kind, question, expected) in enumerate(questions(COUNT)):
    inputs = tokenizer.apply_chat_template(
        [{"role": "user", "content": question}], add_generation_prompt=True,
        return_tensors="pt", return_dict=True).to(model.device)
    sensor = Sensor()
    out = model.generate(**inputs, max_new_tokens=16, do_sample=False,
                         logits_processor=LogitsProcessorList([sensor]))
    answer = tokenizer.decode(out[0, inputs["input_ids"].shape[1]:], skip_special_tokens=True).strip()
    ents = [e for e, _ in sensor.readings] or [0.0]
    tops = [t for _, t in sensor.readings] or [1.0]
    ok = grade(kind, answer, expected)
    rows.append([i, kind, question, expected, answer, int(ok),
                 sum(ents) / len(ents), max(ents), ents[0], sum(tops) / len(tops), len(ents)])
    print(f"{i + 1:4}/{COUNT}  {'ok   ' if ok else 'WRONG'}  {question[:48]:48}  -> {answer[:24]}")

with open(OUT, "w", newline="", encoding="utf-8") as f:
    w = csv.writer(f)
    w.writerow(["id", "kind", "question", "expected", "answer", "correct",
                "mean_entropy", "max_entropy", "first_entropy", "mean_top_prob", "tokens"])
    w.writerows(rows)

wrong = sum(1 for r in rows if not r[5])
print(f"\n{len(rows)} questions, {wrong} wrong, {time.time() - start:.0f} s. Wrote {OUT}")
