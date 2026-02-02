#!/usr/bin/env python3
"""
Extract room generator related objects from a full jmap file.
"""

import json
import sys
from collections import deque


def get_super_struct(obj: dict) -> str | None:
    """Get the super_struct (parent class) of a jmap object."""
    if "struct" in obj:
        return obj["struct"].get("super_struct")
    return obj.get("super_struct")


def extract_rma_objects(jmap_path: str, output_path: str, trace: str | None = None):
    with open(jmap_path) as f:
        jmap = json.load(f)

    objects = jmap.get("objects", {})

    # Seed classes - room generator and related types
    base_seeds = [
        "/Script/FSD.RoomGenerator",
        "/Script/FSD.RoomGeneratorBase",
        "/Script/FSD.RoomFeature",
        "/Script/FSD.RandomSelector",
        "/Script/FSD.RoomFeatureSelector",
    ]

    # Build inheritance map: class -> list of subclasses
    subclasses: dict[str, list[str]] = {}
    for path, obj in objects.items():
        super_struct = get_super_struct(obj)
        if super_struct:
            subclasses.setdefault(super_struct, []).append(path)

    # Find all subclasses of base seeds recursively
    def collect_subclasses(base: str, into: set[str]):
        into.add(base)
        for sub in subclasses.get(base, []):
            if sub not in into:
                collect_subclasses(sub, into)

    seeds_set = set()
    for seed in base_seeds:
        collect_subclasses(seed, seeds_set)
    seeds = list(seeds_set)

    print(f"Found {len(seeds)} seed classes (including subclasses)", file=sys.stderr)

    # BFS to collect all referenced types
    needed = set()
    parent = {}  # Track who referenced what
    queue = deque(seeds)

    for s in seeds:
        parent[s] = None

    while queue:
        path = queue.popleft()
        if path in needed:
            continue
        if path not in objects:
            continue

        needed.add(path)
        obj = objects[path]

        # Collect only type-relevant references (not outer/children/CDO)
        refs = collect_type_refs(obj)
        for ref in refs:
            if ref not in needed and ref not in parent:
                parent[ref] = path
                if ref in objects:
                    queue.append(ref)

    # Trace a specific path if requested
    if trace:
        if trace in parent:
            chain = []
            cur = trace
            while cur is not None:
                chain.append(cur)
                cur = parent.get(cur)
            print("Reference chain:", file=sys.stderr)
            for i, p in enumerate(reversed(chain)):
                print(f"  {'  ' * i}{p}", file=sys.stderr)
        else:
            print(f"{trace} not found in references", file=sys.stderr)

    # Build minimal jmap
    minimal = {
        "metadata": jmap.get("metadata"),
        "image_base_address": jmap.get("image_base_address", "0x0"),
        "objects": {k: v for k, v in objects.items() if k in needed},
        "vtables": {},
        "names": None,
    }

    print(f"Extracted {len(minimal['objects'])} objects from {len(objects)}", file=sys.stderr)

    with open(output_path, "w") as f:
        json.dump(minimal, f, indent=2)

    print(f"Wrote {output_path}", file=sys.stderr)


def collect_type_refs(obj: dict) -> set[str]:
    """Collect type-relevant references from a jmap object.

    Only follows:
    - super_struct (parent class)
    - class (object's class)
    - properties (property type definitions)
    - struct/enum references in property types

    Ignores:
    - outer (package hierarchy)
    - children (child objects)
    - class_default_object (CDO)
    - property_values (runtime values)
    """
    refs = set()

    # Class reference
    if "object" in obj:
        inner = obj["object"]
    else:
        inner = obj

    if "class" in inner and inner["class"].startswith("/Script/"):
        refs.add(inner["class"])

    # Super struct (parent class)
    if "struct" in obj:
        struct = obj["struct"]
        if "super_struct" in struct and struct["super_struct"]:
            refs.add(struct["super_struct"])
        # Properties from struct
        for prop in struct.get("properties", []):
            refs.update(collect_property_type_refs(prop))

    # Direct super_struct (for ScriptStruct)
    if "super_struct" in obj and obj["super_struct"]:
        refs.add(obj["super_struct"])

    # Direct properties
    for prop in obj.get("properties", []):
        refs.update(collect_property_type_refs(prop))

    return refs


def collect_property_type_refs(prop: dict) -> set[str]:
    """Collect type references from a property definition."""
    refs = set()

    # Struct property
    if "struct" in prop and prop["struct"].startswith("/Script/"):
        refs.add(prop["struct"])

    # Enum property
    if "enum" in prop and prop["enum"] and prop["enum"].startswith("/Script/"):
        refs.add(prop["enum"])

    # Object/Class property (skip meta_class - not needed for serialization)
    if "property_class" in prop and prop["property_class"].startswith("/Script/"):
        refs.add(prop["property_class"])
    if "interface_class" in prop and prop["interface_class"].startswith("/Script/"):
        refs.add(prop["interface_class"])

    # Array/Set inner type
    if "inner" in prop:
        refs.update(collect_property_type_refs(prop["inner"]))

    # Map key/value types
    if "key_prop" in prop:
        refs.update(collect_property_type_refs(prop["key_prop"]))
    if "value_prop" in prop:
        refs.update(collect_property_type_refs(prop["value_prop"]))

    # Enum container
    if "container" in prop:
        refs.update(collect_property_type_refs(prop["container"]))

    return refs


if __name__ == "__main__":
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <input.jmap> <output.jmap> [--trace PATH]", file=sys.stderr)
        sys.exit(1)

    trace = None
    if "--trace" in sys.argv:
        idx = sys.argv.index("--trace")
        if idx + 1 < len(sys.argv):
            trace = sys.argv[idx + 1]

    extract_rma_objects(sys.argv[1], sys.argv[2], trace=trace)
