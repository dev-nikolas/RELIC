#!/usr/bin/env python3
"""
tools/fix_holdout_75_gold.py
Audits and precisely corrects the gold RIR and expected_output of tests/holdout_templates_v1.json
matching each template's semantic initial_quantity, operand, and unit.
"""
import json, re
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent
file_path = ROOT_DIR / "tests" / "holdout_templates_v1.json"

SPANISH_WORDS = {
    "cero": 0, "uno": 1, "un": 1, "una": 1, "dos": 2, "tres": 3, "cuatro": 4, "cinco": 5,
    "seis": 6, "siete": 7, "ocho": 8, "nueve": 9, "diez": 10, "once": 11, "doce": 12,
    "trece": 13, "catorce": 14, "quince": 15, "dieciséis": 16, "dieciseis": 16, "diecisiete": 17,
    "dieciocho": 18, "diecinueve": 19, "veinte": 20, "veintiuno": 21, "veintidós": 22,
    "veintidos": 22, "veintitrés": 23, "veintitres": 23, "veinticuatro": 24, "veinticinco": 25,
    "veintiséis": 26, "veintiseis": 26, "veintisiete": 27, "veintiocho": 28, "veintinueve": 29,
    "treinta": 30
}

def extract_prompt_numbers(text: str):
    nums = []
    tokens = re.findall(r"\b[\wáéíóúñ]+\b|\d+", text.lower())
    for t in tokens:
        if t.isdigit():
            nums.append(int(t))
        elif t in SPANISH_WORDS:
            nums.append(SPANISH_WORDS[t])
    return nums

with open(file_path, "r", encoding="utf-8") as f:
    cases = json.load(f)

for idx, c in enumerate(cases):
    inp = c["input"]
    gold = c.get("gold_rir", {})
    st = c.get("expected_status")
    tid = c.get("id", "")
    
    if st == "VALID":
        p_nums = extract_prompt_numbers(inp)
        norm = inp.lower()
        
        # Determine semantic initial_quantity and operand based on template pattern
        if "sáquele" in norm or "saquele" in norm: # "sáquele {b} a {a}" -> init=a, op=b (sub)
            init_q = p_nums[1]
            op_q = p_nums[0]
            op_kind = "subtract"
            unit = ""
        elif norm.startswith("a ") and ("quítale" in norm or "quitale" in norm): # "a {a} quítale {b}" -> init=a, op=b (sub)
            init_q = p_nums[0]
            op_q = p_nums[1]
            op_kind = "subtract"
            unit = ""
        elif "menos" in norm and "son?" in norm: # "{a} menos {b} son?" -> init=a, op=b (sub)
            init_q = p_nums[0]
            op_q = p_nums[1]
            op_kind = "subtract"
            unit = ""
        elif "llegaron" in norm and "a las" in norm: # "llegaron {b} {units} a las {a} que ya tenía guardadas" -> init=a, op=b (add)
            init_q = p_nums[1]
            op_q = p_nums[0]
            op_kind = "add"
            # find unit
            m_unit = re.search(r"llegaron \w+ (\w+)", norm)
            unit = m_unit.group(1) if m_unit else ""
        elif "me gasté" in norm or "me gaste" in norm: # "me gasté {b} de las {a} {units} que tenía" -> init=a, op=b (sub)
            init_q = p_nums[1]
            op_q = p_nums[0]
            op_kind = "subtract"
            m_unit = re.search(r"de las \w+ (\w+)", norm)
            unit = m_unit.group(1) if m_unit else ""
        elif "de los" in norm and "presté" in norm: # "de los {a} {units} que tenía, presté {b}" -> init=a, op=b (sub)
            init_q = p_nums[0]
            op_q = p_nums[1]
            op_kind = "subtract"
            m_unit = re.search(r"de los \w+ (\w+)", norm)
            unit = m_unit.group(1) if m_unit else ""
        else:
            init_q = p_nums[0] if len(p_nums) > 0 else 0
            op_q = p_nums[1] if len(p_nums) > 1 else 0
            op_kind = gold.get("operations", [{}])[0].get("operator", "subtract")
            unit = ""

        if op_kind == "subtract":
            res = init_q - op_q
        elif op_kind == "add":
            res = init_q + op_q
        elif op_kind == "multiply":
            res = init_q * op_q
        else:
            res = init_q // op_q if op_q != 0 else 0

        c["gold_rir"]["initial_quantity"] = float(init_q)
        if "operations" in c["gold_rir"] and len(c["gold_rir"]["operations"]) > 0:
            c["gold_rir"]["operations"][0]["operator"] = op_kind
            c["gold_rir"]["operations"][0]["operand"] = float(op_q)
        else:
            c["gold_rir"]["operations"] = [{"operator": op_kind, "operand": float(op_q)}]
        
        c["expected_output"] = f"{res} {unit}".strip() if unit else str(res)

with open(file_path, "w", encoding="utf-8") as f:
    json.dump(cases, f, indent=2, ensure_ascii=False)
print("tests/holdout_templates_v1.json perfectamente corregido.")
