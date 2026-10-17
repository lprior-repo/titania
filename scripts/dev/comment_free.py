"""Comment-free enforcement tool.

Exemption rationale lives here as prose, never in comment syntax:
MARKER_FIXTURE_EXEMPTIONS names fixture files whose test contract requires
real suppression-marker lines. The production detector only recognizes
markers inside genuine comment text, so a string literal holding the marker
text would falsify the fixture. Exempt paths are skipped by process() and
pass --check with marker lines intact.
"""
import os
import sys
MARKER_FIXTURE_EXEMPTIONS = frozenset(
    [
        os.path.normpath(
            "crates/titania-lanes/tests/fixtures/ast_grep/bypass/"
            "bypass_inline_suppression_violation.rs"
        ),
    ]
)
def is_exempt(path):
    return os.path.normpath(path) in MARKER_FIXTURE_EXEMPTIONS
def strip_rust(text):
    chars = list(text)
    total = len(chars)
    parts = []
    current = []
    code = [False]
    seen = [False]
    mode = [0]
    depth = [0]
    hashes = [0]
    def emit(ch):
        current.append(ch)
        if ch != " " and ch != "\t" and ch != "\r" and ch != "\n":
            seen[0] = True
        if ch != " " and ch != "\t" and ch != "\r" and ch != "\n":
            code[0] = True
    def end_line():
        if mode[0] == 3 or mode[0] == 5:
            parts.append("".join(current) + "\n")
        elif code[0]:
            parts.append("".join(current).rstrip() + "\n")
        elif not seen[0]:
            parts.append("\n")
        del current[:]
        code[0] = False
        seen[0] = False
        if mode[0] == 1:
            mode[0] = 0
    def is_ident(ch):
        return ch.isalnum() or ch == "_"
    i = 0
    while i < total:
        c = chars[i]
        nxt = chars[i + 1] if i + 1 < total else ""
        nxt2 = chars[i + 2] if i + 2 < total else ""
        m = mode[0]
        if c == "\n":
            end_line()
            i += 1
        elif m == 1:
            seen[0] = True
            i += 1
        elif m == 2:
            seen[0] = True
            if c == "/" and nxt == "*":
                depth[0] += 1
                i += 2
            elif c == "*" and nxt == "/":
                depth[0] -= 1
                i += 2
                if depth[0] == 0:
                    mode[0] = 0
            else:
                i += 1
        elif m == 3:
            if c == "\\":
                emit(c)
                if nxt != "":
                    emit(nxt)
                    i += 2
                else:
                    i += 1
            elif c == '"':
                emit(c)
                mode[0] = 0
                i += 1
            else:
                emit(c)
                i += 1
        elif m == 5:
            if c == '"':
                need = hashes[0]
                k = 0
                while k < need and i + 1 + k < total and chars[i + 1 + k] == "#":
                    k += 1
                after = i + 1 + need
                if k == need and (after >= total or chars[after] != "#"):
                    emit(c)
                    for _ in range(need):
                        emit("#")
                    i = after
                    mode[0] = 0
                else:
                    emit(c)
                    i += 1
            else:
                emit(c)
                i += 1
        elif c == "/" and nxt == "/":
            mode[0] = 1
            i += 2
        elif c == "/" and nxt == "*":
            mode[0] = 2
            depth[0] = 1
            i += 2
        elif c == '"':
            emit(c)
            mode[0] = 3
            i += 1
        elif c == "r":
            j = i + 1
            h = 0
            while j < total and chars[j] == "#":
                h += 1
                j += 1
            if j < total and chars[j] == '"':
                for t in range(i, j + 1):
                    emit(chars[t])
                mode[0] = 5
                hashes[0] = h
                i = j + 1
            else:
                emit(c)
                i += 1
        elif c == "'":
            if nxt == "\\":
                emit(c)
                emit(nxt)
                i += 2
                k = 0
                while i < total and chars[i] != "'" and k < 10:
                    emit(chars[i])
                    i += 1
                    k += 1
                if i < total and chars[i] == "'":
                    emit(chars[i])
                    i += 1
            elif nxt != "" and nxt2 == "'":
                emit(c)
                emit(nxt)
                emit(nxt2)
                i += 3
            elif nxt.isalpha() or nxt == "_":
                emit(c)
                i += 1
                while i < total and is_ident(chars[i]):
                    emit(chars[i])
                    i += 1
            else:
                emit(c)
                i += 1
        else:
            emit(c)
            i += 1
    if current:
        if code[0]:
            parts.append("".join(current).rstrip())
        elif mode[0] == 3 or mode[0] == 5:
            parts.append("".join(current))
        elif not seen[0]:
            parts.append("")
    return "".join(parts)
def split_shell_comment(line, start):
    i = start
    total = len(line)
    quote = ""
    triple = False
    while i < total:
        c = line[i]
        nxt = line[i + 1] if i + 1 < total else ""
        if quote != "":
            if triple:
                if line.startswith(quote, i):
                    i += 3
                    quote = ""
                    triple = False
                else:
                    if c == "\\" and not quote.startswith("'"):
                        i += 1
                    i += 1
            else:
                if c == "\\" and quote == '"':
                    i += 2
                elif c == quote:
                    quote = ""
                    i += 1
                else:
                    i += 1
        else:
            if c == "'" or c == '"':
                if line.startswith(c * 3, i):
                    quote = c
                    triple = True
                    i += 3
                else:
                    quote = c
                    i += 1
            elif c == "#":
                if i == 0 or line[i - 1] in " \t;&|()":
                    return i
                i += 1
            else:
                i += 1
    return -1
def strip_shell_python_block(lines):
    out = []
    for line in lines:
        pos = split_shell_comment(line, 0)
        if pos == -1:
            out.append(line)
        elif line[:pos].strip() == "":
            continue
        else:
            out.append(line[:pos].rstrip())
    return out
def find_heredocs(line):
    found = []
    k = 0
    while k < len(line):
        if line.startswith("<<", k):
            k += 2
            dash = False
            if k < len(line) and line[k] == "-":
                dash = True
                k += 1
            quoted = ""
            if k < len(line) and (line[k] == "'" or line[k] == '"'):
                quoted = line[k]
                k += 1
            name = ""
            while k < len(line) and (line[k].isalnum() or line[k] == "_"):
                name += line[k]
                k += 1
            if quoted != "" and k < len(line) and line[k] == quoted:
                k += 1
            if name != "":
                tag = ("-" if dash else "") + name
                if "PYEOF" in name:
                    found.append((tag, "python"))
                else:
                    found.append((tag, "text"))
        else:
            k += 1
    return found
def strip_shell(text):
    lines = text.split("\n")
    out = []
    bodies = []
    pending = []
    pending_kind = ""
    i = 0
    total = len(lines)
    while i < total:
        line = lines[i]
        if bodies:
            tag, kind = bodies[0]
            delim = tag[1:] if tag.startswith("-") else tag
            check = line.lstrip("\t") if tag.startswith("-") else line
            if check == delim:
                if pending_kind == "python":
                    out.extend(strip_shell_python_block(pending))
                else:
                    out.extend(pending)
                pending = []
                pending_kind = ""
                bodies.pop(0)
                out.append(line)
            else:
                if kind == "python" and pending_kind == "":
                    pending_kind = "python"
                pending.append(line)
            i += 1
            continue
        if i == 0 and line.lstrip().startswith("#!"):
            out.append(line)
            i += 1
            continue
        found = find_heredocs(line)
        pos = split_shell_comment(line, 0)
        if pos == -1:
            out.append(line)
        elif line[:pos].strip() != "":
            out.append(line[:pos].rstrip())
        bodies.extend(found)
        i += 1
    out.extend(pending)
    result = "\n".join(out)
    if text.endswith("\n") and not result.endswith("\n"):
        result += "\n"
    return result
def strip_python(text):
    kept = strip_shell_python_block(text.split("\n"))
    result = "\n".join(kept)
    if text.endswith("\n") and not result.endswith("\n"):
        result += "\n"
    return result


def process(path, write):
    if is_exempt(path):
        return False
    with open(path, encoding="utf-8") as handle:
        text = handle.read()
    if path.endswith(".sh"):
        result = strip_shell(text)
    elif path.endswith(".py"):
        result = strip_python(text)
    else:
        result = strip_rust(text)
    if result != text:
        sys.stdout.write(path + "\n")
        if write:
            with open(path, "w", encoding="utf-8") as handle:
                handle.write(result)
        return True
    return False
def default_targets():
    import os
    roots = ["crates", "fixtures", "titania", "verification", "scripts"]
    found = []
    for root in roots:
        if not os.path.isdir(root):
            continue
        for base, dirs, files in os.walk(root):
            parts = base.split(os.sep)
            if "target" in parts or ".titania" in parts or ".git" in parts:
                dirs[:] = []
                continue
            for name in files:
                if name.endswith(".rs") or name.endswith(".sh") or name.endswith(".py"):
                    found.append(os.path.join(base, name))
    return sorted(found)


def main(argv):
    write = "--write" in argv
    check = "--check" in argv
    given = [item for item in argv[1:] if not item.startswith("--")]
    targets = given if given else default_targets()
    dirty = False
    for path in targets:
        if process(path, write):
            dirty = True
    if check and dirty:
        return 1
    return 0
if __name__ == "__main__":
    sys.exit(main(sys.argv))
