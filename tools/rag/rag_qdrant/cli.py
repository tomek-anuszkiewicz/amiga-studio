import sys
import argparse
from pathlib import Path

# Ensure UTF-8 output where possible
if sys.platform == "win32":
    try:
        sys.stdout.reconfigure(encoding="utf-8")
        sys.stderr.reconfigure(encoding="utf-8")
    except Exception:
        pass

from .config import DEFAULT_SOURCE, QDRANT_URL, COLLECTION_NAME
from .indexer import KnowledgeIndexer

HELP_TEXT = """
[bold cyan]amiga_rag[/bold cyan] - Multi-Source Local RAG Indexer (Qdrant)

[bold yellow]USAGE:[/bold yellow]
  amiga_rag [PATH] [OPTIONS]

[bold yellow]ARGUMENTS:[/bold yellow]
  [green]PATH[/green]                   Directory to index (e.g. [bold].[/bold] for current folder, or [bold]D:\\GoogleDrive\\AI\\Obsidian[/bold])

[bold yellow]OPTIONS:[/bold yellow]
  [green]-s, --source NAME[/green]      Tag for the indexed source (e.g. 'amiga', 'obsidian').
                         Defaults to folder name or RAG_DEFAULT_SOURCE.
  [green]-l, --list-sources[/green]     Display a table of all indexed sources with file & vector counts.
  [green]--status[/green]               Check Qdrant database connectivity and total collection size.
  [green]--reindex[/green]              Force re-indexing of all files (ignores SHA256 cache).
  [green]-h, --help[/green]             Show this help message and exit.

[bold yellow]EXAMPLES:[/bold yellow]
  amiga_rag . --source amiga
  amiga_rag D:\\GoogleDrive\\AI\\Obsidian --source obsidian
  amiga_rag --list-sources
  amiga_rag --status
"""

PLAIN_HELP_TEXT = """
amiga_rag - Multi-Source Local RAG Indexer (Qdrant)

USAGE:
  amiga_rag [PATH] [OPTIONS]

ARGUMENTS:
  PATH                   Directory to index (e.g. '.' for current folder, or D:\\GoogleDrive\\AI\\Obsidian)

OPTIONS:
  -s, --source NAME      Tag for the indexed source (e.g. 'amiga', 'obsidian').
                         Defaults to folder name or RAG_DEFAULT_SOURCE.
  -l, --list-sources     Display a table of all indexed sources with file & vector counts.
  --status               Check Qdrant database connectivity and total collection size.
  --reindex              Force re-indexing of all files (ignores SHA256 cache).
  -h, --help             Show this help message and exit.

EXAMPLES:
  amiga_rag . --source amiga
  amiga_rag D:\\GoogleDrive\\AI\\Obsidian --source obsidian
  amiga_rag --list-sources
  amiga_rag --status
"""


def print_help():
    try:
        from rich.console import Console
        console = Console()
        console.print(HELP_TEXT)
    except Exception:
        print(PLAIN_HELP_TEXT)


def show_sources_table(indexer: KnowledgeIndexer):
    sources = indexer.get_sources_stats()
    try:
        from rich.console import Console
        from rich.table import Table
        console = Console()
        if not sources:
            console.print("[yellow]No sources indexed yet. Run 'amiga_rag <PATH>' to index a folder.[/yellow]")
            return

        table = Table(title="[bold green]Indexed Knowledge Sources in Qdrant[/bold green]")
        table.add_column("Source Tag", style="cyan", no_wrap=True)
        table.add_column("Indexed Files", justify="right", style="magenta")
        table.add_column("Chunks / Vectors", justify="right", style="green")
        table.add_column("Last Updated", style="dim")

        for s in sources:
            table.add_row(
                s["source"],
                str(s["files_count"]),
                str(s["chunks_count"]),
                s["last_updated"]
            )
        console.print(table)
    except Exception:
        if not sources:
            print("No sources indexed yet.")
            return
        print("-" * 60)
        print(f"{'Source Tag':<15} {'Files':<10} {'Vectors':<10} {'Last Updated'}")
        print("-" * 60)
        for s in sources:
            print(f"{s['source']:<15} {s['files_count']:<10} {s['chunks_count']:<10} {s['last_updated']}")
        print("-" * 60)


def show_status(indexer: KnowledgeIndexer):
    try:
        from rich.console import Console
        console = Console()
        indexer.ensure_collection()
        info = indexer.client.get_collection(COLLECTION_NAME)
        console.print(f"[bold green]Qdrant Status:[/bold green] Connected to {QDRANT_URL}")
        console.print(f"Collection: [cyan]{COLLECTION_NAME}[/cyan]")
        console.print(f"Total Vectors: [bold green]{info.points_count}[/bold green]")
        console.print(f"Collection Status: {info.status}")
    except Exception as e:
        print(f"[Status Error] Failed to connect to Qdrant at {QDRANT_URL}: {e}")


def main():
    if len(sys.argv) <= 1:
        print_help()
        sys.exit(0)

    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("path", nargs="?", default=None, help="Directory path to index")
    parser.add_argument("-s", "--source", default=None, help="Source tag (e.g. 'amiga', 'obsidian')")
    parser.add_argument("-l", "--list-sources", action="store_true", help="List all indexed sources")
    parser.add_argument("--status", action="store_true", help="Show database connection and status")
    parser.add_argument("--reindex", action="store_true", help="Force re-index ignoring hash cache")
    parser.add_argument("-h", "--help", action="store_true", help="Show help")

    try:
        args, unknown = parser.parse_known_args()
    except Exception:
        print_help()
        sys.exit(1)

    if args.help:
        print_help()
        sys.exit(0)

    # Clean path string from accidental trailing quotes or slashes
    target_path_str = args.path
    if target_path_str:
        target_path_str = target_path_str.strip('"\'; ')
    elif unknown:
        # Check if an unknown arg is actually an existing directory
        for u in unknown:
            cleaned = u.strip('"\'; ')
            if Path(cleaned).is_dir():
                target_path_str = cleaned
                unknown.remove(u)
                break

    if unknown and not (args.list_sources or args.status):
        print(f"\n[Error] Unknown option(s): {' '.join(unknown)}\n")
        print_help()
        sys.exit(1)

    try:
        indexer = KnowledgeIndexer()
    except Exception as e:
        print(f"[Error] Initialization failed: {e}")
        sys.exit(1)

    if args.list_sources:
        show_sources_table(indexer)
        return

    if args.status:
        show_status(indexer)
        show_sources_table(indexer)
        return

    if not target_path_str:
        print_help()
        return

    target_dir = Path(target_path_str).resolve()
    if not target_dir.is_dir():
        print(f"[Error] Target path '{target_path_str}' does not exist or is not a directory.")
        sys.exit(1)

    # Determine source name
    source_name = args.source or DEFAULT_SOURCE or target_dir.name.lower()

    try:
        from rich.console import Console
        from rich.progress import Progress, BarColumn, TextColumn
        console = Console()
        console.print(f"[bold cyan]Starting indexing for:[/bold cyan] {target_dir}")
        console.print(f"[bold cyan]Assigned Source Tag:[/bold cyan] [bold green]{source_name}[/bold green]")
        if args.reindex:
            console.print("[yellow]Forced re-indexing enabled (cache ignored).[/yellow]")

        with Progress(
            TextColumn("[progress.description]{task.description}"),
            BarColumn(),
            TextColumn("[progress.percentage]{task.percentage:>3.0f}%"),
            TextColumn("({task.completed}/{task.total})"),
            console=console
        ) as progress:
            task_id = progress.add_task("Scanning...", total=100)

            def progress_callback(completed, total, filename, action):
                progress.update(task_id, total=total, completed=completed, description=f"[{action}] {filename[:30]}")

            stats = indexer.index_directory(
                directory=target_dir,
                source_name=source_name,
                force=args.reindex,
                progress_cb=progress_callback
            )

        console.print("\n[bold green]Indexing Complete![/bold green]")
        console.print(f"  • Files Scanned: {stats['scanned']}")
        console.print(f"  • Newly Indexed: {stats['indexed']}")
        console.print(f"  • Updated:       {stats['updated']}")
        console.print(f"  • Skipped:       {stats['skipped']} (unchanged)")
        console.print(f"  • Deleted:       {stats['deleted']}")
        console.print(f"  • Total Vectors Added: {stats['total_points']}")
        if stats["images_analyzed"] > 0:
            console.print(f"  • Diagrams/OCR Analyzed: {stats['images_analyzed']}")
    except Exception as e:
        print(f"Indexing {target_dir} as source '{source_name}'...")
        stats = indexer.index_directory(target_dir, source_name, force=args.reindex)
        print("\nIndexing Complete!", stats)


if __name__ == "__main__":
    main()
