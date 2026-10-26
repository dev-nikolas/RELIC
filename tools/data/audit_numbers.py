#!/usr/bin/env python3
"""
tools/audit_numbers.py
Audits curriculum dataset to ensure all gold operands match prompt numbers.
"""
import json, re, sys

SPANISH_NUMS = {
    0: "cero", 1: "uno", 2: "dos", 3: "tres", 4: "cuatro", 5: "cinco",
    6: "seis", 7: "siete", 8: "ocho", 9: "nueve", 10: "diez",
    11: "once", 12: "doce", 13: "trece", 14: "catorce", 15: "quince",
    16: "dieciséis", 17: "diecisiete", 18: "dieciocho", 19: "diecinueve", 20: "veinte",
    21: "veintiuno", 22: "veintidós", 23: "veintitrés", 24: "veinticuatro", 25: "veinticinco",
    26: "veintiséis", 27: "veintisiete", 28: "veintiocho", 29: "veintinueve", 30: "treinta",
    35: "treinta y cinco", 40: "cuarenta", 50: "cincuenta"
}

def num_to_text(n: int) -> str:
    if n in SPANISH_NUMS: return SPANISH_NUMS[n]
    tens = (n // 10) * 10
    units = n % 10
    if tens in SPANISH_NUMS and units in SPANISH_NUMS:
        return f"{SPANISH_NUMS[tens]} y {SPANISH_NUMS[units]}"
    return str(n)

def audit_dataset(path: str):
    mismatches = 0
    total_valid = 0
    with open(path, "r", encoding="utf-8") as f:
        for idx, line in enumerate(f):
            item = json.loads(line)
            gold = item["gold_rir"]
            if item.get("expected_status") == "VALID" or gold.get("status") == "VALID":
                total_valid += 1
                gold_nums = []
                if "initial_quantity" in gold and gold["initial_quantity"] is not None:
                    gold_nums.append(int(gold["initial_quantity"]))
                for op in gold.get("operations", []):
                    if "operand" in op and op["operand"] is not None:
                        gold_nums.append(int(op["operand"]))
                
                prompt_lower = item["prompt"].lower()
                for gn in gold_nums:
                    str_d = str(gn)
                    str_w = num_to_text(gn)
                    # Check if digit or word appears in prompt
                    # Note word might also have un/una for 1
                    found = (str_d in prompt_lower) or (str_w in prompt_lower)
                    if gn == 1 and ("un " in prompt_lower or "una " in prompt_lower or "un\n" in prompt_lower):
                        found = True
                    if not found:
                        mismatches += 1
                        if mismatches <= 5:
                            print(f"[MISMATCH] Line {idx}: Prompt='{item['prompt']}', Missing gn={gn} (word={str_w})")
                        break
    print(f"Audit completed: {mismatches} mismatches out of {total_valid} valid examples.")
    return mismatches

if __name__ == "__main__":
    ds = sys.argv[1] if len(sys.argv) > 1 else "data/curriculum_dataset_v2_20k.jsonl"
    errs = audit_dataset(ds)
    sys.exit(1 if errs > 0 else 0)
