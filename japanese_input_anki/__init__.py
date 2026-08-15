import os
import platform
import importlib.util
import sys
from pathlib import Path
from typing import Optional, TYPE_CHECKING
from aqt.utils import showCritical
from aqt import mw, gui_hooks
from .widgets import InputWidget, ReviewWidget,_grid_hex, _stroke_qcolor
from anki.cards import Card
from aqt.qt import QVBoxLayout
import json
import re


def _select_native_lib() -> Optional[Path]:
    system = platform.system()
    machine = platform.machine().lower()
    addon_dir = Path(os.path.dirname(os.path.normpath(__file__)))

    if system == "Windows":
        filename = "japanese_input_py.windows-x86_64.pyd"
    elif system == "Linux":
        filename = "japanese_input_py.linux-x86_64.so"
    elif system == "Darwin":
        if "arm" in machine or "aarch64" in machine:
            filename = "japanese_input_py.macos-arm64.so"
        else:
            filename = "japanese_input_py.macos-x86_64.so"
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

    module_name = f"{__name__}.japanese_input_py"
    spec = importlib.util.spec_from_file_location(module_name, lib_path)
    if spec is None or spec.loader is None:
        showCritical(f"Japanese Input: Failed to load import spec for {lib_path}")
        return None

    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


def _reference_map_path() -> Path:
    return Path(os.path.dirname(os.path.normpath(__file__))) / "user_files" / "assets" / "reference_data.bin"

def _model_path() -> Path:
    return Path(os.path.dirname(os.path.normpath(__file__))) / "user_files" / "assets" / "recognizer2_tuned.bin"


if TYPE_CHECKING:
    from . import japanese_input_py  # pyright: ignore[reportMissingModuleSource]
else:
    japanese_input_py = _load_native_module()
    if japanese_input_py is None:
        showCritical("Japanese Input: Could not load dependencies")


reference_map_path = _reference_map_path()
model_path = _model_path()

if not reference_map_path.exists():
    showCritical(f"Japanese Input: reference map not found at {reference_map_path}")
if not model_path.exists():
    showCritical(f"Japanese Input: model not found at {model_path}")
try:
    recognizer : japanese_input_py.Recognizer = japanese_input_py.Recognizer(str(model_path))
except Exception as e:
    showCritical(f"Japanese Input: failed to construct recognizer\n\n{e}")
try:
    kanji_grid = japanese_input_py.KanjiGrid(str(reference_map_path))
except Exception as e:
    showCritical(f"Japanese Input: failed to construct kanji_grid\n\n{e}")

try:
    analyzer = japanese_input_py.Analyzer(str(reference_map_path))
except Exception as e:
    showCritical(f"Japanese Input: failed to construct analyzer\n\n{e}")

_input_widget: InputWidget | None = None
_review_widget: ReviewWidget | None = None
_expected_answer: str = ""
_recognized_answer: str = ""

def _ensure_review_widget() -> "ReviewWidget | None":
    global _review_widget
    if mw is None or mw.reviewer is None:
        return None
    if _review_widget is None:
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
def _is_kana(ch: str) -> bool:
    return (
        "ぁ" <= ch <= "ん"      # hiragana
        or "ァ" <= ch <= "ヶ"   # katakana
        or "ｦ" <= ch <= "ﾝ"    # halfwidth katakana
        or ch in "ーｰ・"        # prolonged sound mark + halfwidth, middle dot
    )

def _on_question_shown(card: Card) -> None:
    global _input_widget, _expected_answer, _review_widget
    if mw is None or mw.reviewer is None:
        return
    if _review_widget is not None:
        _review_widget.clear()

    qfmt = card.template().get("qfmt", "")
    has_type_field = isinstance(qfmt, str) and "{{type:" in qfmt

    if not has_type_field:
        if _input_widget is not None:
            _input_widget.hide()
        return

    mw.reviewer.web.eval(
        "var t = document.getElementById('typeans');"
        "if (t) t.style.display = 'none';"
    )

    _expected_answer = _get_expected_answer(card)

  
    if _input_widget is None:
        _input_widget = InputWidget(canvas_size=300, kanji_grid=kanji_grid)
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
    if _input_widget is None:
        return

    commits = _input_widget.commits()
    if not commits:
        return

    grid_color = _grid_hex()      # same colors the canvas uses
    stroke_color = _stroke_qcolor().name()
    corner_radius = 8.0

    analyses: list[japanese_input_py.AnalyzeResult] = []
    # TODO: make sure each drawn character lines up with its expected character
    for ch, strokes in zip(_expected_answer, commits):
        if _is_kana(ch):
            continue
        try:
            res = analyzer.analyze(ch, strokes, grid_color, corner_radius, stroke_color)
        except Exception as e:
            showCritical(f"[japanese-input] analyze failed for {ch!r}: {e}")
            continue
        analyses.append(res)

    if not analyses:
        return

    review = _ensure_review_widget()
    if review is None:
        return
    recognition_ok = (_recognized_answer == _expected_answer)
    review.set_analyses(analyses, recognition_ok)
    review.show()



def _on_js_message(handled: tuple[bool, object], message: str, context: object) -> tuple[bool, object]:
    global _recognized_answer
    if message != "ans":
        return handled
    if _input_widget is None or not _input_widget.isVisible():
        return handled
    if  mw is None or mw.reviewer is None:
        return handled

    _input_widget.auto_commit_pending()
    commits = _input_widget.commits()
    if not commits:
        return handled

    try:
        result: str = recognizer.compare_with_target(commits, _expected_answer) 
    except Exception as e:
        showCritical(f"[japanese-input] analyze_answer failed: {e}")
        return handled
    _recognized_answer = result 

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