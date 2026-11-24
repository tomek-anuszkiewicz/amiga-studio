"""Locate editable Rust declarations and resolve source module paths."""

from dataclasses import dataclass, field
import json
from pathlib import Path


KINDS = {
    "const_item": "constant",
    "enum_item": "enum",
    "enum_variant": "variant",
    "extern_crate_declaration": "extern crate",
    "field_declaration": "field",
    "function_item": "function",
    "function_signature_item": "function",
    "mod_item": "module",
    "static_item": "static",
    "struct_item": "struct",
    "trait_item": "trait",
    "type_item": "type",
    "union_item": "union",
    "use_declaration": "re-export",
}
BUILTIN_ATTRIBUTES = {
    "allow", "automatically_derived", "cfg", "cfg_attr", "cold", "deny", "deprecated",
    "doc", "expect", "export_name", "forbid", "global_allocator", "inline", "link",
    "link_name", "link_section", "macro_export", "macro_use", "must_use", "no_mangle",
    "no_implicit_prelude", "non_exhaustive", "path", "repr", "target_feature", "test",
    "track_caller", "unsafe", "used", "warn",
}
ITEM_SCOPES = {"source_file", "declaration_list"}


def make_parser():
    from tree_sitter import Language, Parser
    import tree_sitter_rust

    return Parser(Language(tree_sitter_rust.language()))


def string_value(node):
    if node is not None and node.type in {"string_literal", "raw_string_literal"}:
        if node.type == "raw_string_literal":
            raw = node.text.decode()
            return raw[raw.index('"') + 1:raw.rindex('"')]
        try:
            return json.loads(node.text)
        except ValueError:
            return None
    return None


def text(node):
    return node.text.decode() if node is not None else ""


def location(path, node):
    return {
        "path": path, "line": node.start_point.row + 1, "byte_column": node.start_point.column + 1,
        "byte_start": node.start_byte, "byte_end": node.end_byte,
    }


def attribute(node):
    value = node.named_children[0] if node.named_child_count else None
    if value is None or not value.named_child_count:
        return "", None, None
    return (text(value.named_children[0]), value.child_by_field_name("arguments"),
            value.child_by_field_name("value"))


def token_groups(tree):
    if tree is None:
        return []
    groups = [[]]
    for token in tree.children[1:-1]:
        if token.type in {"line_comment", "block_comment"}:
            continue
        if token.type == ",":
            groups.append([])
        else:
            groups[-1].append(token)
    if not groups[-1]:
        groups.pop()
    return groups


def any_cfg(values):
    values = list(values)
    if True in values:
        return True
    return None if None in values else False


def all_cfg(values):
    values = list(values)
    if False in values:
        return False
    return None if None in values else True


def negate(value):
    return None if value is None else not value


def predicate(tokens, features, test):
    if not tokens:
        return None
    name = text(tokens[0])
    if name == "test" and len(tokens) == 1:
        return test
    if name == "feature" and len(tokens) == 3 and tokens[1].type == "=":
        value = string_value(tokens[2])
        return value in features if value is not None else None
    if len(tokens) == 2 and tokens[1].type == "token_tree":
        values = [predicate(group, features, test) for group in token_groups(tokens[1])]
        if name == "all":
            return all_cfg(values)
        if name == "any":
            return any_cfg(values)
        if name == "not" and len(values) == 1:
            return negate(values[0])
    return None


def cfg_enabled(name, arguments, features, test):
    if name == "test":
        return test
    groups = token_groups(arguments)
    if name == "cfg" and len(groups) == 1:
        return predicate(groups[0], features, test)
    if name == "cfg_attr" and groups:
        enabled = predicate(groups[0], features, test)
        implied = all_cfg(cfg_enabled(text(group[0]), group[1], features, test)
                          for group in groups[1:] if len(group) == 2
                          and text(group[0]) in {"cfg", "cfg_attr"})
        return any_cfg((negate(enabled), implied))
    return None


@dataclass
class Declaration:
    name: str
    kind: str
    module: tuple
    enclosing: tuple
    span: dict
    visibility: dict | None
    origins: list = field(default_factory=list)

    def report(self, package):
        return {
            "package": package, "name": self.name, "kind": self.kind,
            "module": "::".join(self.module), "enclosing": list(self.enclosing),
            "span": self.span, "visibility_span": self.visibility,
            "targets": [origin[0] for origin in self.origins],
            "conditions": [[text(attr) for attr in conditions] for _, conditions in self.origins],
        }

    def activity(self, units):
        values = []
        for target, conditions in self.origins:
            for unit in units:
                if unit["target"] == target:
                    features, test = unit["features"], unit["test"]
                    values.append(all_cfg(cfg_enabled(*attribute(node)[:2], features, test)
                                          for node in conditions))
        return any_cfg(values)


class Discovery:
    def __init__(self, root, package, report):
        self.root = root
        self.package = package
        self.report = report
        self.parser = make_parser()
        self.sources = {}
        self.declarations = {}
        self.target = None

    def source(self, path):
        if path not in self.sources:
            original = (self.root / path).read_bytes()
            self.sources[path] = original, self.parser.parse(original).root_node
        return self.sources[path]

    def gap(self, path, node, reason):
        span = location(path, node) if node is not None else {"path": path}
        self.report["gaps"].append({"target": self.target, "span": span, "reason": reason})

    def discover(self, target):
        self.target = target_key(target)
        path = Path(target["src_path"]).relative_to(self.root).as_posix()
        self.file(path, ("crate",), Path(path).parent, (), ())

    def file(self, path, module, mod_dir, conditions, stack):
        if path in stack:
            self.gap(path, None, "The source file occurs again in this module's ancestors.")
            return
        try:
            _, node = self.source(path)
        except OSError as error:
            self.gap(path, None, f"Module source could not be read: {error}")
            return
        self.visit(node, path, module, mod_dir, (), conditions, (*stack, path))

    def remember(self, name, kind, path, node, visibility, module, enclosing, conditions):
        span = location(path, node)
        key = (path, node.start_byte, module, enclosing)
        if key not in self.declarations:
            self.declarations[key] = Declaration(name, kind, module, enclosing, span, visibility)
        declaration = self.declarations[key]
        declaration.origins.append((self.target, conditions))

    def module(self, node, path, module, mod_dir, name, attrs, conditions, stack):
        name = name.removeprefix("r#")
        module = (*module, name)
        body = node.child_by_field_name("body")
        paths = []
        for attr in attrs:
            kind, arguments, value = attribute(attr)
            if kind == "path":
                paths.append(string_value(value))
            elif kind == "cfg_attr" and any(text(group[0]) in {"path", "cfg_attr"}
                                            for group in token_groups(arguments)[1:]):
                paths.append(None)
        if len(paths) > 1 or paths == [None]:
            self.gap(path, node, "Conditional or ambiguous #[path] prevents resolving the module source.")
            return
        path_base = mod_dir if node.parent.type != "source_file" else Path(path).parent
        if body is not None:
            directory = path_base / paths[0] if paths else mod_dir / name
            self.visit(body, path, module, directory, (), conditions, stack)
            return
        options = ([path_base / paths[0]] if paths
                   else [mod_dir / (name + ".rs"), mod_dir / name / "mod.rs"])
        found = [(self.root / option).resolve() for option in options if (self.root / option).is_file()]
        if len(found) != 1 or not found[0].is_relative_to(self.root):
            self.gap(path, node, "Module source does not resolve to one file in the captured workspace.")
            return
        source = found[0].relative_to(self.root)
        directory = source.parent if paths or source.name == "mod.rs" else source.with_suffix("")
        self.file(source.as_posix(), module, directory, conditions, stack)

    def visit(self, node, path, module, mod_dir, enclosing, conditions, stack, attrs=(), field_index=None):
        if node.is_error or node.is_missing:
            self.gap(path, node, "Rust syntax could not be parsed at this location.")
            return
        if node.type in {"macro_definition", "token_tree", "token_tree_pattern"}:
            return
        if node.type == "macro_invocation":
            if node.parent.type in ITEM_SCOPES or node.parent.parent is not None and (
                    node.parent.type == "expression_statement" and node.parent.parent.type in ITEM_SCOPES):
                self.gap(path, node, f"{text(node.child_by_field_name('macro'))}! may generate or include declarations.")
            return
        inner = [child for child in node.named_children if child.type == "inner_attribute_item"]
        for attr in (*attrs, *inner):
            name, _, _ = attribute(attr)
            if name == "derive":
                self.gap(path, attr, "A derive macro can generate items without separate source visibility tokens.")
            elif name not in BUILTIN_ATTRIBUTES:
                self.gap(path, attr, "An attribute can transform the item before compilation.")
                return
        conditions = (*conditions, *(attr for attr in (*attrs, *inner)
                                      if attribute(attr)[0] in {"cfg", "cfg_attr", "test"}))
        visibility = next((location(path, child) for child in node.named_children
                           if child.type == "visibility_modifier" and child.text == b"pub"), None)
        name = text(node.child_by_field_name("name"))
        kind = KINDS.get(node.type)
        if node.type == "use_declaration":
            name = text(node.child_by_field_name("argument"))
        if field_index is not None:
            name, kind, visibility = str(field_index), "field", location(path, node)
        if kind and name:
            self.remember(name, kind, path, node, visibility, module, enclosing, conditions)
        elif visibility is not None:
            self.gap(path, node, "This declaration has no reliably identifiable name or kind.")
        if node.type == "mod_item":
            self.module(node, path, module, mod_dir, name, attrs, conditions, stack)
            return
        if node.type == "impl_item":
            name = text(node.child_by_field_name("type"))
        if name and kind != "re-export":
            enclosing = (*enclosing, name)
        pending = []
        tuple_index = 0
        for child in node.named_children:
            if child.type == "attribute_item":
                pending.append(child)
                continue
            if child.type in {"inner_attribute_item", "line_comment", "block_comment"}:
                continue
            if node.type == "ordered_field_declaration_list":
                if child.type == "visibility_modifier":
                    if child.text == b"pub":
                        self.visit(child, path, module, mod_dir, enclosing, conditions, stack, pending, tuple_index)
                    continue
                tuple_index += 1
            if child.type != "visibility_modifier":
                self.visit(child, path, module, mod_dir, enclosing, conditions, stack, pending)
                pending = []

    def edits(self):
        unique = {}
        for declaration in self.declarations.values():
            if declaration.visibility is None:
                continue
            key = (declaration.span["path"], declaration.visibility["byte_start"])
            if key in unique:
                self.gap(declaration.span["path"], None,
                         f"Visibility at byte {key[1]} has more than one logical module origin.")
                unique[key] = None
            else:
                unique[key] = declaration
        self.report["declarations"] = [
            declaration.report(self.package) for declaration in self.declarations.values()
            if declaration.visibility is not None
        ]
        return unique


def target_key(target):
    return {"name": target["name"], "kind": target["kind"]}
