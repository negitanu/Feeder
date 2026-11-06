#!/usr/bin/env python3
"""Inventory source files only. Never opens runtime data, credentials or databases."""
import argparse, ast, hashlib, json, subprocess
from pathlib import Path
p=argparse.ArgumentParser(); p.add_argument('source',type=Path); p.add_argument('--output',type=Path,required=True); a=p.parse_args()
root=a.source.resolve(); rows=[]
for file in sorted((root/'feedastrolabe').glob('*.py')):
    tree=ast.parse(file.read_text()); routes=[]
    for node in ast.walk(tree):
        if isinstance(node,(ast.FunctionDef,ast.AsyncFunctionDef)):
            for dec in node.decorator_list:
                if isinstance(dec,ast.Call) and isinstance(dec.func,ast.Attribute) and dec.func.attr in ('route','get','post','put','patch','delete') and dec.args and isinstance(dec.args[0],ast.Constant):
                    methods=next((ast.literal_eval(k.value) for k in dec.keywords if k.arg=='methods'),[dec.func.attr.upper()] if dec.func.attr!='route' else ['GET'])
                    routes.append({'path':dec.args[0].value,'methods':methods,'function':node.name,'line':node.lineno})
    rows.append({'path':str(file.relative_to(root)),'sha256':hashlib.sha256(file.read_bytes()).hexdigest(),'routes':routes})
try: commit=subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD'],text=True).strip()
except subprocess.CalledProcessError: commit=None
result={'source_commit':commit,'note':'Source worktree inventory; may contain uncommitted source changes. Routes are decorator fragments; blueprint prefixes and authorization require review. No runtime data accessed.','modules':rows}
a.output.parent.mkdir(parents=True,exist_ok=True); a.output.write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
print(f'{len(rows)} modules; {sum(len(r["routes"]) for r in rows)} route decorators')
