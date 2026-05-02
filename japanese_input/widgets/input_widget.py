from aqt.qt import (
    QWidget, QVBoxLayout, QHBoxLayout, QGridLayout,
    QPushButton, QPointF,
)

from .grid_viewer import GridViewer
from .grid_draw_viewer import GridDrawViewer
INITIAL_SLOTS: int = 5

class InputWidget(QWidget):
    """User-facing input widget.

    Holds a drawing canvas, an undo button, a commit button, and a
    dynamic list of slot previews showing previously committed kanji.
    """

    SLOT_SIZE: int = 60
    SLOTS_PER_ROW: int = 5

    def __init__(
        self,
        parent: QWidget | None = None,
        canvas_size: int = 300,
    ) -> None:
        super().__init__(parent)
        self._canvas_size: int = canvas_size

        # committed strokes per kanji, in canvas-pixel coordinates
        self._committed: list[list[list[QPointF]]] = []
        self._slot_widgets: list[GridViewer] = []
        self._expected_answer: str = ""
        self._filled: int = 0
        self._build_ui()

    # --- UI construction ---

    def _build_ui(self) -> None:
        outer = QVBoxLayout(self)
        outer.setContentsMargins(4, 4, 4, 4)
        outer.setSpacing(4)

        # slot grid (top, wraps), centered
        slot_row = QHBoxLayout()
        slot_row.addStretch()
        self._slot_grid = QGridLayout()
        self._slot_grid.setSpacing(0)
        slot_row.addLayout(self._slot_grid)
        slot_row.addStretch()
        outer.addLayout(slot_row)

 
        # canvas, centered
        canvas_row = QHBoxLayout()
        canvas_row.addStretch()
        self._canvas = GridDrawViewer(size=self._canvas_size)
        canvas_row.addWidget(self._canvas)
        canvas_row.addStretch()
        outer.addLayout(canvas_row)


        # buttons
        btn_row = QHBoxLayout()
        btn_row.addStretch()

        btn_undo = QPushButton("取消")
        btn_undo.setFixedSize(64, 52)
        btn_undo.clicked.connect(self._on_undo)
        btn_row.addWidget(btn_undo)

        btn_commit = QPushButton("次へ")
        btn_commit.setFixedSize(64, 52)
        btn_commit.clicked.connect(self._on_commit)
        btn_row.addWidget(btn_commit)

        btn_hint = QPushButton("手本")
        btn_hint.setFixedSize(64, 52)
        btn_hint.clicked.connect(self._on_hint)
        btn_row.addWidget(btn_hint)

        btn_row.addStretch()
        outer.addLayout(btn_row)

        for _ in range(INITIAL_SLOTS):
            self._add_empty_slot()

    def reset(self) -> None:
        self._canvas.clear()
        # Remove dynamically-added slots beyond the initial 7
        while len(self._slot_widgets) > INITIAL_SLOTS:
            slot = self._slot_widgets.pop()
            self._slot_grid.removeWidget(slot)
            slot.deleteLater()
        # Clear all initial slots
        for slot in self._slot_widgets:
            slot.set_strokes([])
        self._committed.clear()
        self._filled = 0
    
    def commits(self) -> list[list[list[tuple[float, float]]]]:
        """Return committed strokes, normalized to [0,1] canvas space."""
        inv = 1.0 / self._canvas_size
        return [
            [
                [(p.x() * inv, p.y() * inv) for p in stroke]
                for stroke in kanji
            ]
            for kanji in self._committed
        ]

    def auto_commit_pending(self) -> None:
        """If the canvas has uncommitted strokes, commit them now."""
        if self._canvas.stroke_count() > 0:
            self._on_commit()

    def set_expected_answer(self, expected: str) -> None:
        self._expected_answer = expected
        # if hint was showing, clear it — old expected is no longer relevant
        self._canvas.clear_hint()
    # --- button handlers ---

    def _on_hint(self) -> None:
        if self._canvas.has_hint():
            self._canvas.clear_hint()
            return
        idx = len(self._committed)
        if 0 <= idx < len(self._expected_answer):
            self._canvas.set_hint(self._expected_answer[idx])
    def _on_undo(self) -> None:
        if self._canvas.stroke_count() == 0 and self._committed:
            self._pop_last_kanji()
        self._canvas.undo_stroke()


    def _on_commit(self) -> None:
        strokes = self._canvas.strokes()
        if not strokes:
            return
        self._committed.append(strokes)
        self._add_slot(strokes)
        self._canvas.clear()

    # --- slot management ---


    def _make_slot(self) -> GridViewer:
        ratio = self.SLOT_SIZE / self._canvas_size
        slot = GridViewer(size=self.SLOT_SIZE, pen_width=8.0 * ratio)
        slot.setStyleSheet(
            "border: 0px solid palette(mid); border-radius: 4px;"
        )
        return slot

    def _add_empty_slot(self) -> None:
        slot = self._make_slot()
        index = len(self._slot_widgets)
        row = index // self.SLOTS_PER_ROW
        col = index % self.SLOTS_PER_ROW
        self._slot_grid.addWidget(slot, row, col)
        self._slot_widgets.append(slot)

    def _add_slot(self, strokes: list[list[QPointF]]) -> None:
        scaled = self._scale_strokes(strokes, self._canvas_size, self.SLOT_SIZE)
        # Fill next empty pre-rendered slot if available
        if self._filled < len(self._slot_widgets):
            self._slot_widgets[self._filled].set_strokes(scaled)
        else:
            # Beyond initial 7: create a new slot dynamically
            slot = self._make_slot()
            slot.set_strokes(scaled)
            index = len(self._slot_widgets)
            row = index // self.SLOTS_PER_ROW
            col = index % self.SLOTS_PER_ROW
            self._slot_grid.addWidget(slot, row, col)
            self._slot_widgets.append(slot)
        self._filled += 1
    def _pop_last_kanji(self) -> None:
        if not self._committed:
            return
        self._committed.pop()
        self._filled -= 1
        # If the last slot is one of the initial 7, just clear it.
        # If it's a dynamically added one, remove it entirely.
        if len(self._slot_widgets) > INITIAL_SLOTS:
            slot = self._slot_widgets.pop()
            self._slot_grid.removeWidget(slot)
            slot.deleteLater()
        else:
            self._slot_widgets[self._filled].set_strokes([])

    @staticmethod
    def _scale_strokes(
        strokes: list[list[QPointF]],
        source_size: int,
        target_size: int,
        padding: int = 4,
    ) -> list[list[QPointF]]:
        available = target_size - padding * 2
        scale = available / source_size
        return [
            [QPointF(p.x() * scale + padding, p.y() * scale + padding) for p in s]
            for s in strokes
        ]