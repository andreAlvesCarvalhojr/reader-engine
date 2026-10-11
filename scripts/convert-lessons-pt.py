#!/usr/bin/env python3
"""Converte o JSON bruto de lições (array) para reader-json do engine."""
from __future__ import annotations

import argparse
import json
from pathlib import Path


def convert(lessons: list) -> dict:
    pages = []
    for lesson in lessons:
        lid = lesson.get("id") or "lesson"
        title = lesson.get("title") or lid
        bits = [f"<article><h1>{title}</h1>"]
        if lesson.get("period"):
            bits.append(f"<p><strong>Período:</strong> {lesson['period']}</p>")
        if lesson.get("memoryVerse"):
            bits.append(
                f"<section><h2>Verso de memorização</h2>{lesson['memoryVerse']}</section>"
            )
        if lesson.get("readings"):
            bits.append(f"<section><h2>Leituras</h2>{lesson['readings']}</section>")
        bits.append("</article>")
        pages.append({"id": f"{lid}-overview", "title": title, "html": "".join(bits)})

        for i, day in enumerate(lesson.get("days") or []):
            day_title = day.get("title") or day.get("name") or f"Dia {i + 1}"
            label = " · ".join(
                x for x in [day.get("name"), day.get("date"), day_title] if x
            )
            html = ["<article>"]
            if day.get("kicker"):
                html.append(f"<p class=\"kicker\">{day['kicker']}</p>")
            html.append(f"<h2>{day_title}</h2>")
            if day.get("bibleYear"):
                html.append(f"<p><em>{day['bibleYear']}</em></p>")
            html.append(day.get("content") or "")
            html.append("</article>")
            pages.append(
                {"id": f"{lid}-d{i + 1}", "title": label, "html": "".join(html)}
            )

    return {
        "format": "reader-json",
        "title": "Lições da Escola Sabatina (PT)",
        "authors": ["Material de teste local"],
        "language": "pt-BR",
        "pages": pages,
    }


def resolve_input(path: Path) -> Path:
    """Aceita um .json ou uma pasta contendo um .json."""
    path = path.expanduser().resolve()
    if path.is_file():
        return path
    if path.is_dir():
        preferred = [
            path / "lessons-pt.json",
            path / "lessons-pt.raw.json",
            path / "lesson-json.json",
        ]
        for candidate in preferred:
            if candidate.is_file():
                print(f"Usando arquivo na pasta: {candidate}")
                return candidate
        jsons = sorted(path.glob("*.json"))
        if len(jsons) == 1:
            print(f"Usando unico JSON na pasta: {jsons[0]}")
            return jsons[0]
        if jsons:
            listing = "\n".join(f"  - {j}" for j in jsons)
            raise SystemExit(
                f"{path} e uma pasta com varios JSON. Passe o arquivo:\n{listing}\n"
                f"Ex.: python3 scripts/convert-lessons-pt.py \"{jsons[0]}\""
            )
        raise SystemExit(
            f"{path} e uma pasta sem arquivos .json.\n"
            f"Liste o conteudo: ls -la \"{path}\"\n"
            "Depois passe o arquivo .json, nao a pasta."
        )
    raise SystemExit(
        f"Nao encontrado: {path}\n"
        "Passe o caminho do arquivo .json (ou da pasta que o contem).\n"
        "Dica: no Finder, arraste o arquivo .json para o Terminal.\n"
        "Ex.: python3 scripts/convert-lessons-pt.py ~/Desktop/lesson-json/lessons-pt.json"
    )


def main() -> None:
    p = argparse.ArgumentParser(
        description=__doc__,
        epilog=(
            "Exemplos:\n"
            "  python3 scripts/convert-lessons-pt.py ~/Desktop/lesson-json/lessons-pt.json\n"
            "  python3 scripts/convert-lessons-pt.py ~/Desktop/lesson-json\n"
            "  ./scripts/install-local-sample.sh"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    p.add_argument(
        "input",
        type=Path,
        help="Arquivo .json bruto OU pasta que contenha o .json",
    )
    p.add_argument(
        "-o",
        "--output",
        type=Path,
        default=Path("fixtures/local/lessons-pt.json"),
        help="Saída reader-json (default: fixtures/local/lessons-pt.json)",
    )
    args = p.parse_args()
    src = resolve_input(args.input)
    data = json.loads(src.read_text(encoding="utf-8"))
    if not isinstance(data, list):
        raise SystemExit(
            f"Entrada deve ser um array JSON de lições (arquivo: {src})"
        )
    doc = convert(data)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    raw = args.output.with_name(args.output.stem + ".raw.json")
    if raw.resolve() != src.resolve():
        raw.write_bytes(src.read_bytes())
    print(f"OK: {args.output} ({len(doc['pages'])} páginas)")


if __name__ == "__main__":
    main()
