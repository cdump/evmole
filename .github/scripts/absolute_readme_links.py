import argparse
import re
from pathlib import Path


parser = argparse.ArgumentParser(description='Make repository links absolute in the packaged README.')
parser.add_argument('source', type=Path)
parser.add_argument('out', type=Path)
parser.add_argument('--repo', default='cdump/evmole')
parser.add_argument('--ref', required=True)
args = parser.parse_args()

github = f'https://github.com/{args.repo}'
raw = f'https://raw.githubusercontent.com/{args.repo}/{args.ref}'


def markdown_link(match):
    label, target = match.groups()
    if label.startswith('!'):
        return f'{label}({raw}/{target})'
    path, separator, fragment = target.partition('#')
    kind = 'tree' if (args.source.parent / path).is_dir() else 'blob'
    return f'{label}({github}/{kind}/{args.ref}/{path.rstrip("/")}{separator}{fragment})'


readme = args.source.read_text(encoding='utf-8')
readme = re.sub(r'(!?\[[^\[\]\n]*\])\(\./([^\s)]+)\)', markdown_link, readme)
# The generated benchmark tables use HTML links to provider directories.
readme = re.sub(
    r'href="(benchmark/providers/[^/"]+)/?"',
    rf'href="{github}/tree/{args.ref}/\1"',
    readme,
)
args.out.write_text(readme, encoding='utf-8', newline='\n')
