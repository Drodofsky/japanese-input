import os
import platform
import importlib.util
import sys
import json
import re
from pathlib import Path
from typing import Optional

from aqt import mw, gui_hooks
from aqt.qt import QVBoxLayout
from aqt.utils import showCritical
from anki.cards import Card

from .widgets.input_widget import InputWidget
from .widgets.review_widget import ReviewWidget

# ── native module loading ─────────────────────────────────────────────────────

def _select_native_lib() -> Optional[Path]:
    system = platform.system()
    machine = platform.machine().lower()
    addon_dir = Path(os.path.dirname(os.path.normpath(__file__)))

    if system == "Windows":
        filename = "japanese_input_native.windows-x86_64.pyd"
    elif system == "Linux":
        filename = "japanese_input_native.linux-x86_64.so"
    elif system == "Darwin":
        if "arm" in machine or "aarch64" in machine:
            filename = "japanese_input_native.macos-arm64.so"
        else:
            filename = "japanese_input_native.macos-x86_64.so"
    else:
        return None

    return addon_dir / "lib" / filename


def _load_native_module() -> object | None:
    lib_path = _select_native_lib()

    if lib_path is None or not lib_path.exists():
        showCritical(
            "Japanese Input Add-on Error\n\n"
            f"Native library not found: {lib_path}\n\n"
            "This is likely an unsupported platform."
        )
        return None

    module_name = f"{__name__}.japanese_input_native"
    spec = importlib.util.spec_from_file_location(module_name, lib_path)
    if spec is None or spec.loader is None:
        showCritical(f"Japanese Input: Failed to load import spec for {lib_path}")
        return None

    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


def _kanji_map_path() -> Path:
    return Path(os.path.dirname(os.path.normpath(__file__))) / "user_files" / "assets" / "kanji.bin"
def _hiragana_map_path() -> Path:
    return Path(os.path.dirname(os.path.normpath(__file__))) / "user_files" / "assets" / "hiragana.bin"
def _katakana_map_path() -> Path:
    return Path(os.path.dirname(os.path.normpath(__file__))) / "user_files" / "assets" / "katakana.bin"

# ── recognizer + analyzer + widget state ──────────────────────────────────────

_native = _load_native_module()
_recognizer: object | None = None
_analyzer: object | None = None

if _native is not None:
    kanji_map_path = _kanji_map_path()
    if not kanji_map_path.exists():
        showCritical(f"Japanese Input: kanji map not found at {kanji_map_path}")
    hiragana_map_path = _hiragana_map_path()
    if not hiragana_map_path.exists():
        showCritical(f"Japanese Input: kanji map not found at {hiragana_map_path}")
    katakana_map_path = _katakana_map_path()
    if not katakana_map_path.exists():
        showCritical(f"Japanese Input: kanji map not found at {katakana_map_path}")
    else:
        try:
            _recognizer = _native.Recognizer(str(hiragana_map_path),str(kanji_map_path))  # type: ignore[attr-defined]
        except Exception as e:
            showCritical(f"Japanese Input: failed to construct recognizer\n\n{e}")
        try:
            _analyzer = _native.KanjiAnalyzer(str(kanji_map_path))  # type: ignore[attr-defined]
        except Exception as e:
            showCritical(f"Japanese Input: failed to construct analyzer\n\n{e}")

_input_widget: InputWidget | None = None
_review_widget: ReviewWidget | None = None
_expected_answer: str = ""
_has_type_field: bool = False


# ── card utilities ────────────────────────────────────────────────────────────

def _get_expected_answer(card: Card) -> str:
    if mw is None:
        return ""

    qfmt = card.template().get("qfmt", "")
    if not isinstance(qfmt, str):
        return ""

    match = re.search(r"\{\{type:(.+?)\}\}", qfmt)
    if match is None:
        return ""

    field_name = match.group(1)
    note = card.note()
    for name, value in note.items():
        if name == field_name:
            clean = re.sub(r"<[^>]+>", "", value)
            return clean.strip()
    return ""


def _ensure_review_widget() -> ReviewWidget | None:
    global _review_widget
    if mw is None or mw.reviewer is None:
        return None
    if _review_widget is not None:
        return _review_widget

    _review_widget = ReviewWidget(canvas_size=300)
    web_parent = mw.reviewer.web.parentWidget()
    if web_parent is None:
        return None
    layout = web_parent.layout()
    if isinstance(layout, QVBoxLayout):
        web_index = layout.indexOf(mw.reviewer.web)
        layout.insertWidget(web_index + 2, _review_widget)
    _review_widget.hide()
    return _review_widget


# ── hooks ─────────────────────────────────────────────────────────────────────

def _on_question_shown(card: Card) -> None:
    global _input_widget, _expected_answer, _has_type_field
    if mw is None or mw.reviewer is None:
        return

    qfmt = card.template().get("qfmt", "")
    _has_type_field = isinstance(qfmt, str) and "{{type:" in qfmt

    if not _has_type_field:
        if _input_widget is not None:
            _input_widget.hide()
        if _review_widget is not None:
            _review_widget.hide()
        return

    mw.reviewer.web.eval(
        "var t = document.getElementById('typeans');"
        "if (t) t.style.display = 'none';"
    )

    _expected_answer = _get_expected_answer(card)

    if _review_widget is not None:
        _review_widget.hide()

    if _input_widget is None:
        _input_widget = InputWidget(canvas_size=300)
        web_parent = mw.reviewer.web.parentWidget()
        if web_parent is None:
            return
        layout = web_parent.layout()
        if isinstance(layout, QVBoxLayout):
            web_index = layout.indexOf(mw.reviewer.web)
            layout.insertWidget(web_index + 1, _input_widget)

    _input_widget.set_expected_answer(_expected_answer)
    _input_widget.reset()
    _input_widget.show()


def _on_answer_shown(card: Card) -> None:
    if _input_widget is not None:
        _input_widget.hide()
    if not _has_type_field:
        if _review_widget is not None:
            _review_widget.hide()
        return
    if _analyzer is None or _input_widget is None:
        return
    commits = _input_widget.commits()
    if not commits:
        return

    try:
        analyses = _analyzer.analyze(commits, _expected_answer)  # type: ignore[attr-defined]
    except Exception as e:
        print(f"[japanese-input] analyze failed: {e}")
        return

    if not analyses:
        return

    review = _ensure_review_widget()
    if review is None:
        return

    review.set_analyses(analyses)
    review.show()


def _on_js_message(handled: tuple[bool, object], message: str, context: object) -> tuple[bool, object]:
    if message != "ans":
        return handled
    if _input_widget is None or not _input_widget.isVisible():
        return handled
    if _recognizer is None or mw is None or mw.reviewer is None:
        return handled

    _input_widget.auto_commit_pending()
    commits = _input_widget.commits()
    if not commits:
        return handled

    try:
        result: str = _recognizer.analyze_answer(commits, _expected_answer)  # type: ignore[attr-defined]
    except Exception as e:
        print(f"[japanese-input] analyze_answer failed: {e}")
        return handled

    escaped = json.dumps(result)
    mw.reviewer.web.eval(
        f"document.getElementById('typeans').value = {escaped};"
    )
    return handled


def _on_reviewer_will_end() -> None:
    if _input_widget is not None:
        _input_widget.hide()
    if _review_widget is not None:
        _review_widget.hide()


gui_hooks.reviewer_did_show_question.append(_on_question_shown)
gui_hooks.reviewer_did_show_answer.append(_on_answer_shown)
gui_hooks.webview_did_receive_js_message.append(_on_js_message)
gui_hooks.reviewer_will_end.append(_on_reviewer_will_end)