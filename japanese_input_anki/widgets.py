from __future__ import annotations
from aqt.qt import (
    QWidget, QVBoxLayout, QHBoxLayout, QGridLayout, QPushButton,
    QPainter, QPen, QPolygonF, QPointF, QRectF, Qt,
    QPaintEvent, QMouseEvent, QTabletEvent, QEvent, QByteArray,
    QColor, QApplication, QPalette
)
from typing import TYPE_CHECKING


if TYPE_CHECKING:
    from PyQt6.QtSvg import QSvgRenderer
    from . import japanese_input_py  # pyright: ignore[reportMissingModuleSource]
    KanjiGrid = japanese_input_py.KanjiGrid
    ResultKind =  japanese_input_py.ResultKind
    AnalyzeResult = japanese_input_py.AnalyzeResult
else:
    try:
        from PyQt5.QtSvg import QSvgRenderer
    except ModuleNotFoundError:
        from PyQt6.QtSvg import QSvgRenderer

Stroke = list[tuple[float, float]]
INITIAL_SLOTS: int = 5


class GridViewer(QWidget):
    def __init__(self, svg: str, size: int, pen_width: float | None = None,
                 parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self.setFixedSize(size, size)
        self._size: int = size
        self._renderer: QSvgRenderer = QSvgRenderer(QByteArray(svg.encode()))
        self._strokes: list[Stroke] = []
        self._pen: QPen = QPen(
            _stroke_qcolor(),
            pen_width if pen_width is not None else size * 8.0 / 300.0,
            Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap, Qt.PenJoinStyle.RoundJoin,
        )

    def set_svg(self, svg: str) -> None:
        self._renderer.load(QByteArray(svg.encode()))
        self.update()

    def set_strokes(self, strokes: list[Stroke]) -> None:
        self._strokes = [list(s) for s in strokes]
        self.update()

    def paintEvent(self, a0: QPaintEvent |None) -> None:
        p: QPainter = QPainter(self)
        p.setRenderHint(QPainter.RenderHint.Antialiasing)
        self._renderer.render(p, QRectF(self.rect()))
        p.setPen(self._pen)
        s: int = self._size
        for stroke in self._strokes:
            if len(stroke) == 1:
                p.drawPoint(QPointF(stroke[0][0] * s, stroke[0][1] * s))
            else:
                p.drawPolyline(QPolygonF([QPointF(x * s, y * s) for x, y in stroke]))
        p.end()
    def changeEvent(self, a0: QEvent | None) -> None:
        if a0 is not None and a0.type() == QEvent.Type.PaletteChange:
            self._pen.setColor(_stroke_qcolor())
            self.update()
        super().changeEvent(a0)

    


class GridDrawViewer(GridViewer):
    def __init__(self, grid: KanjiGrid, size: int,
                 corner_radius: float = 8.0,
                 parent: QWidget | None = None) -> None:
        self._grid: KanjiGrid = grid
        self._corner_radius: float = corner_radius
        self._hint: str = ""
        self._pen_down: bool = False
        super().__init__(self._render_svg(), size, parent=parent)
    def _render_svg(self) -> str:
        if self._hint:
            return self._grid.generate_with_hint(
                _grid_hex(), self._corner_radius, self._hint, _hint_hex())
        return self._grid.generate(_grid_hex(), self._corner_radius)


    def _norm(self, pos: QPointF) -> tuple[float, float]:
        return (pos.x() / self._size, pos.y() / self._size)

    def _start(self, pos: QPointF) -> None:
        self._strokes.append([self._norm(pos)])
        self.update()

    def _extend(self, pos: QPointF) -> None:
        if self._strokes:
            self._strokes[-1].append(self._norm(pos))
            self.update()

    def strokes(self) -> list[Stroke]:
        return [list(s) for s in self._strokes]

    def stroke_count(self) -> int:
        return len(self._strokes)

    def undo_stroke(self) -> None:
        if self._strokes:
            self._strokes.pop()
            self.update()

    def clear(self) -> None:
        self._strokes = []
        self.update()

    def set_hint(self, hint: str) -> None:
        self._hint = hint
        self.set_svg(self._render_svg())
    def clear_hint(self) -> None:
        self._hint = ""
        self.set_svg(self._render_svg())

    def has_hint(self) -> bool:
        return bool(self._hint)

    def mousePressEvent(self, a0: QMouseEvent|None) -> None:
        if a0:
            self._start(a0.position())
    def mouseReleaseEvent(self, a0: QMouseEvent | None) -> None:
        if a0 and self._strokes and len(self._strokes[-1]) < 2:
            self._strokes.pop()
            self.update()

    def mouseMoveEvent(self, a0: QMouseEvent|None) -> None:
        if a0:
            self._extend(a0.position())

    def tabletEvent(self, a0: QTabletEvent|None) -> None:
        if a0:
            t: QEvent.Type = a0.type()
            if t == QEvent.Type.TabletPress:
                self._pen_down = True
                self._start(a0.position())
            elif t == QEvent.Type.TabletMove and self._pen_down:
                self._extend(a0.position())
            elif t == QEvent.Type.TabletRelease:
                self._pen_down = False
                if a0 and self._strokes and len(self._strokes[-1]) < 2:
                    self._strokes.pop()
                    self.update()

            a0.accept()
    def changeEvent(self, a0: QEvent | None) -> None:
        if a0 is not None and a0.type() == QEvent.Type.PaletteChange:
            self.set_svg(self._render_svg())
        super().changeEvent(a0)


class InputWidget(QWidget):
    SLOT_SIZE: int = 60
    SLOTS_PER_ROW: int = 5

    def __init__(self, kanji_grid: KanjiGrid, parent: QWidget | None = None,
                 canvas_size: int = 300,
                 corner_radius: float = 8.0) -> None:
        super().__init__(parent)
        self._canvas_size: int = canvas_size
        self._grid = kanji_grid
        self._corner_radius: float = corner_radius
        self._bg: str = kanji_grid.generate(_grid_hex(), corner_radius)
        self._committed: list[list[Stroke]] = []
        self._slot_widgets: list[GridViewer] = []
        self._expected_answer: str = ""
        self._filled: int = 0
        self._build_ui()

    def _build_ui(self) -> None:
        outer: QVBoxLayout = QVBoxLayout(self)
        outer.setContentsMargins(4, 4, 4, 4)
        outer.setSpacing(4)

        slot_row: QHBoxLayout = QHBoxLayout()
        slot_row.addStretch()
        self._slot_grid: QGridLayout = QGridLayout()
        self._slot_grid.setSpacing(0)
        slot_row.addLayout(self._slot_grid)
        slot_row.addStretch()
        outer.addLayout(slot_row)

        canvas_row: QHBoxLayout = QHBoxLayout()
        canvas_row.addStretch()
        self._canvas: GridDrawViewer = GridDrawViewer(
            self._grid, self._canvas_size,
            self._corner_radius)
        canvas_row.addWidget(self._canvas)
        canvas_row.addStretch()
        outer.addLayout(canvas_row)

        btn_row: QHBoxLayout = QHBoxLayout()
        btn_row.addStretch()
        for label, handler in (("取消", self._on_undo), ("次へ", self._on_commit),
                               ("手本", self._on_hint)):
            btn: QPushButton = QPushButton(label)
            btn.setFixedSize(64, 52)
            btn.clicked.connect(handler)
            btn_row.addWidget(btn)
        btn_row.addStretch()
        outer.addLayout(btn_row)

        for _ in range(INITIAL_SLOTS):
            self._add_empty_slot()

    def reset(self) -> None:
        self._canvas.clear()
        self._canvas.clear_hint()
        while len(self._slot_widgets) > INITIAL_SLOTS:
            slot: GridViewer = self._slot_widgets.pop()
            self._slot_grid.removeWidget(slot)
            slot.deleteLater()
        for slot in self._slot_widgets:
            slot.set_strokes([])
        self._committed.clear()
        self._filled = 0

    def commits(self) -> list[list[Stroke]]:
        return [[list(s) for s in kanji] for kanji in self._committed]

    def auto_commit_pending(self) -> None:
        if self._canvas.stroke_count() > 0:
            self._on_commit()

    def set_expected_answer(self, expected: str) -> None:
        self._expected_answer = expected
        self._canvas.clear_hint()
    
    def changeEvent(self, a0: QEvent | None) -> None:
        if a0 is not None and a0.type() == QEvent.Type.PaletteChange:
            self._bg = self._grid.generate(_grid_hex(), self._corner_radius)
            for slot in self._slot_widgets:
                slot.set_svg(self._bg)
        super().changeEvent(a0)

    def _on_hint(self) -> None:
        if self._canvas.has_hint():
            self._canvas.clear_hint()
            return
        idx: int = len(self._committed)
        if 0 <= idx < len(self._expected_answer):
            self._canvas.set_hint(self._expected_answer[idx])

    def _on_undo(self) -> None:
        if self._canvas.stroke_count() == 0 and self._committed:
            self._pop_last_kanji()
        else:
            self._canvas.undo_stroke()

    def _on_commit(self) -> None:
        strokes: list[Stroke] = self._canvas.strokes()
        if not strokes:
            return
        self._committed.append(strokes)
        self._add_slot(strokes)
        self._canvas.clear()
        self._canvas.clear_hint()

    def _make_slot(self) -> GridViewer:
        slot: GridViewer = GridViewer(self._bg, self.SLOT_SIZE)
        slot.setStyleSheet("border: 0px solid palette(mid); border-radius: 4px;")
        return slot

    def _add_empty_slot(self) -> None:
        slot: GridViewer = self._make_slot()
        index: int = len(self._slot_widgets)
        self._slot_grid.addWidget(slot, index // self.SLOTS_PER_ROW,
                                  index % self.SLOTS_PER_ROW)
        self._slot_widgets.append(slot)

    def _add_slot(self, strokes: list[Stroke]) -> None:
        if self._filled < len(self._slot_widgets):
            self._slot_widgets[self._filled].set_strokes(strokes)
        else:
            slot: GridViewer = self._make_slot()
            slot.set_strokes(strokes)
            index: int = len(self._slot_widgets)
            self._slot_grid.addWidget(slot, index // self.SLOTS_PER_ROW,
                                      index % self.SLOTS_PER_ROW)
            self._slot_widgets.append(slot)
        self._filled += 1

    def _pop_last_kanji(self) -> None:
        if not self._committed:
            return
        self._committed.pop()
        self._filled -= 1
        if len(self._slot_widgets) > INITIAL_SLOTS:
            slot: GridViewer = self._slot_widgets.pop()
            self._slot_grid.removeWidget(slot)
            slot.deleteLater()
        else:
            self._slot_widgets[self._filled].set_strokes([])


def _blend(fg: QColor, bg: QColor, t: float) -> QColor:
    return QColor(
        round(fg.red()   * (1 - t) + bg.red()   * t),
        round(fg.green() * (1 - t) + bg.green() * t),
        round(fg.blue()  * (1 - t) + bg.blue()  * t),
    )

def _stroke_qcolor() -> QColor:
    return QApplication.palette().color(QPalette.ColorRole.WindowText)

def _grid_hex() -> str:
    pal: QPalette = QApplication.palette()
    return _blend(pal.color(QPalette.ColorRole.WindowText),
                  pal.color(QPalette.ColorRole.Window), 0.65).name()

def _hint_hex() -> str:
    pal: QPalette = QApplication.palette()
    return _blend(pal.color(QPalette.ColorRole.WindowText),
                  pal.color(QPalette.ColorRole.Window), 0.80).name()



class _SvgCell(QWidget):
    def __init__(self, svg: str, size: int, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self.setFixedSize(size, size)
        self._renderer = QSvgRenderer(QByteArray(svg.encode()))

    def paintEvent(self, a0: QPaintEvent | None) -> None:
        p = QPainter(self)
        p.setRenderHint(QPainter.RenderHint.Antialiasing)
        self._renderer.render(p, QRectF(self.rect()))
        p.end()


class ReviewWidget(QWidget):
    def __init__(self, parent: QWidget | None = None, canvas_size: int = 300) -> None:
        super().__init__(parent)
        self._cell: int = canvas_size // 2
        self._outer: QVBoxLayout = QVBoxLayout(self)
        self._outer.setContentsMargins(4, 4, 4, 4)
        self._outer.setSpacing(8)

    def clear(self) -> None:
        while self._outer.count():
            item = self._outer.takeAt(0)
            w = item.widget() if item is not None else None
            if w is not None:
                w.deleteLater()
            else:
                # nested layout
                lay = item.layout() if item is not None else None
                if lay is not None:
                    while lay.count():
                        sub = lay.takeAt(0)
                        sw = sub.widget() if sub is not None else None
                        if sw is not None:
                            sw.deleteLater()

    def set_analyses(self, analyses: list[AnalyzeResult],recognition_ok: bool) -> None:
        from . import japanese_input_py # pyright: ignore[reportMissingModuleSource]  # dirty fix
        ResultKind =  japanese_input_py.ResultKind

        self.clear()

        all_correct = all(a.kind == ResultKind.NothingFound for a in analyses)
        if all_correct:
            mark = HANAMARU_SVG if recognition_ok else SANKAKU_SVG
            row = QHBoxLayout()
            row.addStretch()
            row.addWidget(_SvgCell(mark, self._cell))
            row.addStretch()
            self._outer.addLayout(row)
            return

        for a in analyses:
            # skip the per-kanji correct ones in a mixed view; only show the errors
            if a.kind == ResultKind.NothingFound:
                continue
            row = QHBoxLayout()
            row.addStretch()
            row.addWidget(_SvgCell(a.wrong, self._cell))    # left: what you drew
            row.addWidget(_SvgCell(a.correct, self._cell))  # right: correct
            row.addStretch()
            self._outer.addLayout(row)
    def hide(self) -> None:
        self.clear()
        super().hide()



HANAMARU_SVG = """<svg xmlns="http://www.w3.org/2000/svg" width="340" height="340" viewBox="0 0 340 340">
  <g transform="translate(0.000000,340.000000) scale(0.100000,-0.100000)" fill-rule="evenodd">
    <path d="M1137 3008 c-126 -16 -223 -65 -310 -158 -81 -85 -113 -142 -139
-242 -20 -82 -21 -96 -5 -293 2 -27 -1 -30 -43 -37 -217 -37 -354 -316 -261
-532 30 -68 103 -169 151 -207 l39 -32 -53 -57 c-108 -118 -135 -246 -77 -365
36 -72 105 -140 179 -177 72 -35 224 -34 303 3 92 43 139 92 139 145 0 33 -5
46 -22 58 -34 24 -59 20 -140 -21 -101 -52 -172 -53 -239 -5 -85 62 -99 137
-39 216 37 49 116 106 146 106 41 0 79 43 79 88 0 48 -24 78 -69 88 -71 15
-182 120 -224 213 -56 121 -5 273 101 302 37 10 43 8 122 -40 88 -54 110 -59
154 -36 58 29 55 67 -16 206 l-58 113 0 120 c0 112 2 125 27 171 106 199 370
267 521 135 55 -48 81 -100 102 -200 18 -84 35 -108 86 -116 43 -7 84 23 95
69 25 117 199 237 344 237 180 -1 355 -253 256 -371 -74 -88 17 -189 122 -135
145 73 410 -38 491 -207 65 -135 10 -228 -137 -231 -57 -1 -67 -5 -88 -29 -48
-56 -24 -138 43 -152 76 -16 105 -75 111 -226 15 -374 -222 -645 -471 -539
l-59 24 -36 -19 c-48 -26 -60 -60 -43 -124 16 -63 0 -100 -70 -166 -82 -76
-210 -116 -264 -81 -22 14 -25 24 -25 74 0 50 -4 62 -25 82 -52 49 -145 14
-145 -54 0 -72 -246 -113 -389 -65 -119 40 -164 122 -126 227 29 80 30 106 3
135 -86 95 -189 -15 -188 -202 1 -166 91 -277 269 -332 125 -38 324 -32 436
15 33 14 36 13 71 -15 85 -67 249 -67 391 2 73 36 184 151 221 230 l32 66 84
5 c204 11 384 170 465 414 58 173 64 366 15 512 l-24 73 58 54 c67 62 92 120
92 215 0 252 -239 479 -534 508 l-70 7 -12 74 c-24 141 -128 286 -255 356
-157 85 -371 57 -541 -72 l-57 -43 -18 26 c-115 167 -273 237 -476 212z" fill="#e6331b"/>
    <path d="M1635 2361 c-312 -50 -596 -281 -651 -529 -59 -263 70 -562 292 -679
249 -132 568 -36 675 202 60 135 15 284 -128 421 -102 98 -241 113 -387 41
-151 -75 -204 -174 -152 -282 66 -133 221 -184 354 -116 97 50 85 157 -16 139
-131 -23 -130 -23 -167 12 -41 40 -36 57 31 96 31 18 64 28 107 31 76 5 103
-9 166 -91 84 -108 59 -216 -68 -293 -232 -140 -501 24 -543 330 -18 137 12
226 116 336 244 258 614 283 841 56 122 -121 183 -243 192 -382 7 -99 -5 -151
-57 -256 -151 -302 -474 -510 -729 -468 -57 9 -69 15 -121 66 -66 62 -106 71
-154 34 -47 -37 -37 -88 31 -160 92 -97 138 -114 308 -113 131 0 146 2 227 31
333 119 621 445 668 757 49 326 -210 701 -550 795 -83 23 -214 33 -285 22z" fill="#e6331b"/>
  </g>
</svg>
"""

SANKAKU_SVG = """<svg xmlns="http://www.w3.org/2000/svg" width="340" height="340" viewBox="0 0 340 340">
  <path d="M170 60 L290 280 L50 280 Z" fill="none" stroke="#e6331b" stroke-width="14" stroke-linejoin="round" stroke-linecap="round"/>
</svg>"""