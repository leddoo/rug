from xml.dom import minidom, Node
import re

doc = minidom.parse(open("res/tiger.svg"))

root = doc.documentElement


def parse_path(data, indent):
    global r

    data = list(data)
    i = 0

    def skip_ws():
        nonlocal i
        while i < len(data) and data[i] == " ":
            i += 1
        return i < len(data)

    def parse_number():
        nonlocal i

        skip_ws()

        result = ""
        while data[i].isdigit() or data[i] == "." or data[i] == "-" or data[i] == "e":
            result += data[i]
            i += 1
        assert result != ""

        if not "." in result:
            result += ".0"

        return result

    def consume(what):
        nonlocal i
        assert data[i] == what
        i += 1

    spaces = indent * "    "

    while skip_ws():
        cmd = data[i]
        i += 1

        if cmd == "M":
            x = parse_number()
            consume(",")
            y = parse_number()
            r.append(spaces + f"pb.move_to([{x}, {y}].into());\n")
        elif cmd == "L":
            while skip_ws() and (data[i].isdigit() or data[i] == "-"):
                x = parse_number()
                consume(",")
                y = parse_number()
                r.append(spaces + f"pb.line_to([{x}, {y}].into());\n")
        elif cmd == "Q":
            while skip_ws() and (data[i].isdigit() or data[i] == "-"):
                x1 = parse_number()
                consume(",")
                y1 = parse_number()
                x2 = parse_number()
                consume(",")
                y2 = parse_number()
                r.append(spaces + f"pb.quad_to([{x1}, {y1}].into(), [{x2}, {y2}].into());\n")
        elif cmd == "C":
            while skip_ws() and (data[i].isdigit() or data[i] == "-"):
                x1 = parse_number()
                consume(",")
                y1 = parse_number()
                x2 = parse_number()
                consume(",")
                y2 = parse_number()
                x3 = parse_number()
                consume(",")
                y3 = parse_number()
                r.append(spaces + f"pb.cubic_to([{x1}, {y1}].into(), [{x2}, {y2}].into(), [{x3}, {y3}].into());\n")
        elif cmd == "Z":
            r.append(spaces + "pb.close_path();\n")
        else:
            print("unknown path command:", cmd)
            assert False

def parse_rgb(data):
    r = r"rgb\((\d+,\d+,\d+)\)"

    m = re.search(r, data)
    assert m

    values = m.groups()

    return values[0]


def visit(node, indent):
    global r

    if node.nodeType != Node.ELEMENT_NODE:
        return

    spaces = indent*"    "

    if node.tagName == "g":
        r.append(spaces + "{\n")
        for child in node.childNodes:
            visit(child, indent + 1)
        r.append(spaces + "}\n")

    elif node.tagName == "path":
        r.append(spaces + "let p = cb.build_path(|pb| {\n")
        parse_path(node.getAttribute("d"), indent + 1)
        r.append(spaces + "});\n")

        fill = node.getAttribute("fill")
        has_fill = fill and fill != "none"

        if has_fill:
            r.append(spaces + f"cb.push(Cmd::FillPathSolid {{ path: p, color: argb_pack_u8s({parse_rgb(fill)}, 255) }});\n")

        if not has_fill:
            r.append(spaces + "let _ = p;\n")

        r.append("\n")

    else:
        print("ignoring", node)

r = []

r.append("""\
    CmdBuf::new(|cb| {
""")

for child in root.childNodes:
    visit(child, indent=2)

r.append("    })")

r = "".join(r)

print(r)


