"""Run the model and stream each confidence reading over UDP as it's generated.

usage: python stream_confidence.py <receiver-ip> ["your prompt"]
Also writes confidence.csv like before, so you can compare afterward.
"""
import csv
import socket
import struct
import sys
import time

import torch
from transformers import AutoModelForCausalLM, AutoTokenizer, LogitsProcessor, LogitsProcessorList

MODEL_NAME = "Qwen/Qwen2.5-1.5B-Instruct"
PORT = 5106
HOST = sys.argv[1] if len(sys.argv) > 1 else "127.0.0.1"
PROMPT = sys.argv[2] if len(sys.argv) > 2 else "Explain IRIG time code in two sentences."


class StreamingSensor(LogitsProcessor):
    """Your confidence sensor, plus one UDP datagram per reading."""

    def __init__(self, sock, addr):
        self.readings = []  # (timestamp_ns, entropy, top_prob)
        self.sock = sock
        self.addr = addr

    def __call__(self, input_ids, scores):
        log_probs = torch.log_softmax(scores[0].float(), dim=-1)
        probs = log_probs.exp()
        terms = torch.where(probs > 0, probs * log_probs, torch.zeros_like(probs))
        entropy = -terms.sum().item()
        top_prob = probs.max().item()
        t = time.time_ns()  # stamped after .item(), so the GPU has finished
        index = len(self.readings)
        self.readings.append((t, entropy, top_prob))
        # 20 bytes, little-endian: u32 index, u64 t_ns, f32 entropy, f32 top_prob
        self.sock.sendto(struct.pack("<IQff", index, t, entropy, top_prob), self.addr)
        return scores  # unchanged: the sensor must not load the circuit


tokenizer = AutoTokenizer.from_pretrained(MODEL_NAME)
# older transformers versions call this argument torch_dtype
model = AutoModelForCausalLM.from_pretrained(MODEL_NAME, dtype=torch.bfloat16, device_map="cuda")

messages = [{"role": "user", "content": PROMPT}]
inputs = tokenizer.apply_chat_template(
    messages, add_generation_prompt=True, return_tensors="pt", return_dict=True
).to(model.device)

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sensor = StreamingSensor(sock, (HOST, PORT))
output_ids = model.generate(
    **inputs, max_new_tokens=200, do_sample=False,
    logits_processor=LogitsProcessorList([sensor]),
)
sock.sendto(b"", (HOST, PORT))  # empty datagram = end of run

prompt_len = inputs["input_ids"].shape[1]
new_tokens = output_ids[0, prompt_len:]
print(tokenizer.decode(new_tokens, skip_special_tokens=True))

with open("confidence.csv", "w", newline="", encoding="utf-8") as f:
    writer = csv.writer(f)
    writer.writerow(["index", "timestamp_ns", "token", "entropy", "top_prob"])
    for i, token_id in enumerate(new_tokens.tolist()):
        t, entropy, top_prob = sensor.readings[i]
        writer.writerow([i, t, tokenizer.decode([token_id]), entropy, top_prob])

print(f"sent {len(sensor.readings)} readings to {HOST}:{PORT}, also wrote confidence.csv")
