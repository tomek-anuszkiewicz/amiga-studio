"""Audit source recipes without changing authored content or inferring repairs."""

from html import unescape
from html.parser import HTMLParser
import re

from common.pdf_artifacts import read_json
from .markdown import PARSER
from .text_recipe import REFERENCE, resolve_template


SOURCE = '\ue000'
TAGS = {'table', 'thead', 'tbody', 'tfoot', 'tr', 'th', 'td', 'caption',
        'p', 'div', 'span', 'br', 'strong', 'b', 'em', 'i', 'u', 's',
        'sub', 'sup', 'code', 'pre', 'ul', 'ol', 'li', 'blockquote', 'a', 'img'}
ATTRIBUTES = {'rowspan', 'colspan', 'style', 'class', 'id', 'href', 'src',
              'width', 'height', 'align', 'valign', 'scope', 'start', 'type'}
# Formatting commands consume referenced arguments; visible symbols such as
# \\alpha are content, rather than an unrestricted command-name exemption.
MATH_FORMATTING = {'frac', 'dfrac', 'tfrac', 'sqrt', 'overline', 'underline',
                   'mathrm', 'mathbf', 'mathit', 'mathsf', 'mathtt', 'mathcal',
                   'text', 'operatorname', 'left', 'right', 'begin', 'end'}


class MarkupAudit(HTMLParser):
    def __init__(self, content, issue):
        super().__init__(convert_charrefs=False)
        self.content, self.issue = content, issue

    def handle_starttag(self, tag, attrs):
        if tag not in TAGS:
            self.issue('unsupported_markup', self.get_starttag_text())
        for name, value in attrs:
            if name in {'alt', 'title', 'aria-label'}:
                self.content(value or '')
            elif name not in ATTRIBUTES:
                self.issue('unsupported_attribute', name)
            elif name in {'rowspan', 'colspan', 'width', 'height', 'start'}:
                if not re.fullmatch(r'[0-9]+', value or ''):
                    self.issue('invalid_markup_attribute', f'{name}={value}')

    def handle_endtag(self, tag):
        if tag not in TAGS:
            self.issue('unsupported_markup', f'</{tag}>')

    def handle_data(self, data):
        self.content(data)

    def handle_entityref(self, name):
        self.content(unescape('&' + name + ';'))

    def handle_charref(self, name):
        self.content(unescape('&#' + name + ';'))

    def handle_comment(self, data):
        self.issue('unsupported_markup', '<!--' + data + '-->')

    def handle_decl(self, decl):
        self.issue('unsupported_markup', '<!' + decl + '>')


def validate_book(book, workspace):
    """Report unreferenced content and invalid ranges, not semantic attribution.

    Literal syntax is evaluated after replacing each reference with one inert
    source marker. Expanded PDF text is never parsed or mistaken for added text.
    """
    issues, pages = [], {}
    fields, references = 0, 0

    def field(template, chapter_id, element_id, name, kind):
        nonlocal fields, references
        fields += 1
        context = {'chapter_id': chapter_id, 'element_id': element_id, 'field': name}

        def issue(code, text):
            issues.append({**context, 'code': code, 'text': text})

        if not isinstance(template, str):
            issue('invalid_template', repr(template))
            return
        if SOURCE in template:
            issue('reserved_character', SOURCE)
            return
        masked, resolved, position = [], [], 0
        while position < len(template):
            match = REFERENCE.match(template, position)
            if match:
                references += 1
                page = int(match[1])
                source = SOURCE
                try:
                    if page not in pages:
                        pages[page] = read_json(workspace / '02_preprocess' / f'page_{page:04d}.json')['blocks']
                    source = resolve_template(match[0], {page: pages[page]})
                except (ValueError, KeyError, OSError) as error:
                    issue('invalid_reference', f'{match[0]}: {error}')
                masked.append(''.join(char if char in ' \n' else SOURCE for char in source))
                resolved.append(source)
                position = match.end()
            elif template[position] == '\\':
                if position + 1 >= len(template) or template[position + 1] not in '\\{}':
                    issue('invalid_template_escape', template[position:position + 2])
                    position += 1
                else:
                    masked.append(template[position + 1])
                    resolved.append(template[position + 1])
                    position += 2
            else:
                char = template[position]
                if char in '{}':
                    issue('invalid_reference_syntax', template[position:position + 80])
                if char.isspace() and char not in ' \n':
                    issue('literal_whitespace', repr(char))
                masked.append(char)
                resolved.append(char)
                position += 1

        def content(text):
            for match in re.finditer('[^' + SOURCE + ' \n]+', text):
                issue('literal_content', match[0])

        def math(text):
            def command(match):
                if match[1] not in MATH_FORMATTING:
                    issue('unsupported_math_command', match[0])
                return ''
            text = re.sub(r'\\([A-Za-z]+)', command, text)
            # Braces, scripts and spacing are math structure. Operators and
            # variables still require source references.
            content(re.sub(r'[{}_^]', '', text))

        def html(text):
            parser = MarkupAudit(content, issue)
            parser.feed(text)
            parser.close()

        if name == 'title':
            content(''.join(masked))
            return

        def tokens(entries):
            for token in entries:
                if token.type in {'text', 'code_inline', 'code_block'}:
                    text = token.content
                    if kind == 'callout' and token.type == 'text':
                        text = re.sub(r'^\[![A-Za-z][A-Za-z0-9_-]*\](?= |\n|$)', '', text)
                    content(text)
                elif token.type == 'fence':
                    if token.info and not re.fullmatch(r'[A-Za-z0-9_+-]+', token.info.strip()):
                        issue('invalid_code_language', token.info)
                    content(token.content)
                elif token.type in {'html_inline', 'html_block'}:
                    html(token.content)
                elif token.type.startswith('math_'):
                    math(token.content)
                elif token.type in {'link_open', 'image'}:
                    if token.attrGet('title'):
                        content(token.attrGet('title'))
                if token.children:
                    tokens(token.children)

        shadow = list(''.join(masked))
        expanded = ''.join(resolved)
        code_spans = [(match.start(), match.end()) for match in re.finditer(r'(`+)(.*?)\1', expanded, re.S)]
        offsets, offset = [], 0
        for line in expanded.splitlines(keepends=True):
            offsets.append(offset)
            offset += len(line)
        offsets.append(offset)
        for token in PARSER.parse(expanded):
            if token.type in {'fence', 'code_block'} and token.map:
                code_spans.append((offsets[token.map[0]], offsets[token.map[1]]))
        for escape in re.finditer(r'\\([!"#$%&\'()*+,\-./:;<=>?@\[\]\\^_`{|}~])', expanded):
            start, end = escape.span()
            if any(a <= start < b for a, b in code_spans):
                continue
            if shadow[start] != SOURCE and shadow[start + 1] == SOURCE:
                # The backslash encodes an already referenced punctuation mark;
                # it does not add another displayed character.
                shadow[start:end] = SOURCE * (end - start)
        # Import may split formatting command names across literal characters
        # and PDF references. Recognize the reconstructed command in math only,
        # then preserve its syntax while masking all source-derived arguments.
        for expression in re.finditer(r'(?<!\\)(\${1,2})(.+?)\1', expanded, re.S):
            if any(a <= expression.start() < b for a, b in code_spans):
                continue
            for command in re.finditer(r'\\([A-Za-z]+)', expression[2]):
                start = expression.start(2) + command.start()
                end = expression.start(2) + command.end()
                if any(char != SOURCE for char in shadow[start:end]):
                    shadow[start:end] = expanded[start:end]
        tokens(PARSER.parse(''.join(shadow)))

    for chapter in book['chapters']:
        field(chapter['title'], chapter['id'], None, 'title', None)
        for element in chapter['elements']:
            if 'md_text' in element:
                field(element['md_text'], chapter['id'], element['id'], 'md_text', element['type'])
    return {'valid': not issues, 'checked_fields': fields, 'checked_references': references,
            'invalid_fields': len({(i['chapter_id'], i['element_id'], i['field']) for i in issues}),
            'issues': issues}
