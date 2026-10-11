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


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("input", type=Path, help="JSON bruto (array de lições)")
    p.add_argument(
        "-o",
        "--output",
        type=Path,
        default=Path("fixtures/local/lessons-pt.json"),
        help="Saída reader-json (default: fixtures/local/lessons-pt.json)",
    )
    args = p.parse_args()
    data = json.loads(args.input.read_text(encoding="utf-8"))
    if not isinstance(data, list):
        raise SystemExit("Entrada deve ser um array JSON de lições")
    doc = convert(data)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    raw = args.output.with_name(args.output.stem + ".raw.json")
    if raw.resolve() != args.input.resolve():
        raw.write_bytes(args.input.read_bytes())
    print(f"OK: {args.output} ({len(doc['pages'])} páginas)")


if __name__ == "__main__":
    main()
