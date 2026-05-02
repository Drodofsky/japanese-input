from dataclasses import dataclass
from aqt.qt import (
    QWidget, QVBoxLayout, QHBoxLayout,
    QPushButton, QLabel, QPointF, QColor, Qt,
)

from .grid_review_viewer import GridReviewViewer, StrokeStyle


# Highlight colors
_INK = QColor(60, 60, 60)
_GREEN = QColor(40, 160, 70)    # added (Missing)
_RED = QColor(200, 50, 50)      # removed (Extra) / current sub-step (WrongOrder)
_YELLOW = QColor(220, 170, 30)  # moved (PositionCorrection)


@dataclass
class _ResolvedView:
    """What the viewer should display for a (step, sub_step) pair."""
    strokes: list[list[tuple[float, float]]]
    styles: list[StrokeStyle]
    label: str


class ReviewWidget(QWidget):
    """Stepwise reviewer for one kanji's analysis."""

    def __init__(
        self,
        parent: QWidget | None = None,
        canvas_size: int = 300,
    ) -> None:
        super().__init__(parent)
        self._canvas_size: int = canvas_size
        self._analyses: list[object] = []
        self._kanji_idx: int = 0
        self._step: int = 0
        self._sub_step: int = 0

        self._build_ui()

    # --- public API ---

    def set_analyses(self, analyses: list[object]) -> None:
        self._analyses = list(analyses)
        self._kanji_idx = 0
        self._step = 0
        self._sub_step = 0
        self._refresh()
    # --- UI construction ---

    def _build_ui(self) -> None:
        outer = QVBoxLayout(self)
        outer.setContentsMargins(4, 4, 4, 4)
        outer.setSpacing(4)

        canvas_row = QHBoxLayout()
        canvas_row.addStretch()
        self._viewer = GridReviewViewer(size=self._canvas_size)
        canvas_row.addWidget(self._viewer)
        canvas_row.addStretch()
        outer.addLayout(canvas_row)

        self._counter_label = QLabel("")
        self._counter_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        outer.addWidget(self._counter_label)

        self._label = QLabel("")
        self._label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        outer.addWidget(self._label)

        btn_row = QHBoxLayout()
        btn_row.addStretch()
        self._btn_prev = QPushButton("戻る")
        self._btn_prev.setFixedSize(64, 52)
        self._btn_prev.clicked.connect(self._on_prev)
        btn_row.addWidget(self._btn_prev)

        self._btn_next = QPushButton("次へ")
        self._btn_next.setFixedSize(64, 52)
        self._btn_next.clicked.connect(self._on_next)
        btn_row.addWidget(self._btn_next)
        btn_row.addStretch()
        outer.addLayout(btn_row)

    # --- navigation ---

    def _on_next(self) -> None:
        issues = self._current_issues()
        max_step = len(issues)  # quality step is len(issues)
        
        if self._sub_step < self._max_sub_step():
            self._sub_step += 1
        elif self._step < max_step:
            self._step += 1
            self._sub_step = 0
        elif self._kanji_idx < len(self._analyses) - 1:
            self._kanji_idx += 1
            self._step = 0
            self._sub_step = 0
        else:
            return
        self._refresh()

    def _on_prev(self) -> None:
        if self._sub_step > 0:
            self._sub_step -= 1
        elif self._step > 0:
            self._step -= 1
            self._sub_step = self._max_sub_step()
        elif self._kanji_idx > 0:
            self._kanji_idx -= 1
            # land on quality step of previous kanji
            self._step = len(self._current_issues())
            self._sub_step = 0
        else:
            return
        self._refresh()
    def _max_sub_step(self) -> int:
        if not self._current_issues():
            return 0
        if self._step == len(self._current_issues()):  # quality view
            return 0
        issue = self._current_issues()[self._step].issue  # type: ignore[attr-defined]
        if type(issue).__name__ == "WrongOrder":
            count = len(self._strokes_at_step(self._step))
            return max(0, count - 1)
        return 0
    def _at_start(self) -> bool:
        return self._kanji_idx == 0 and self._step == 0 and self._sub_step == 0

    def _at_end(self) -> bool:
        if not self._analyses:
            return True
        if self._kanji_idx < len(self._analyses) - 1:
            return False
        return (
            self._step == len(self._current_issues())
            and self._sub_step == self._max_sub_step()
        )
    # --- rendering ---

    def _refresh(self) -> None:
        if not self._analyses:
        # ... empty state, same as before but using _analyses ...
            self._counter_label.setText("")
            return
        if not self._current_issues():
            self._viewer.set_strokes([])
            self._label.setText("")
            self._btn_prev.setEnabled(False)
            self._btn_next.setEnabled(False)
            return
        self._counter_label.setText(f"{self._kanji_idx + 1}/{len(self._analyses)}")
        view = self._resolve_view()
        scaled = self._scale_to_canvas(view.strokes)
        self._viewer.set_styled_strokes(scaled, view.styles)
        self._label.setText(view.label)
        self._btn_prev.setEnabled(not self._at_start())
        self._btn_next.setEnabled(not self._at_end())

    def _strokes_at_step(self, step: int) -> list[list[tuple[float, float]]]:
        """Snapshot at the given outer step."""
        return list(self._current_issues()[step].corrected_strokes)  # type: ignore[attr-defined]

    def _previous_snapshot(self, step: int) -> list[list[tuple[float, float]]]:
        """The snapshot from before this step's fix was applied."""
        if step == 0:
            return self._current_user_strokes()
        return self._strokes_at_step(step - 1)

    def _resolve_view(self) -> _ResolvedView:
         # Final step: quality coloring on the last issue's snapshot
        if self._step == len(self._current_issues()):
            if self._current_issues():
                strokes = self._strokes_at_step(len(self._current_issues()) - 1)
            else:
                strokes = self._current_user_strokes()
            styles = [
                StrokeStyle(
                    color=_INK,
                    qualities=(self._current_qualities()[i] if i < len(self._current_qualities()) else None),
                )
                for i in range(len(strokes))
            ]
            return _ResolvedView(
                strokes=strokes, styles=styles, label="出来栄え",
            )
        issue_obj = self._current_issues()[self._step]
        issue = issue_obj.issue  # type: ignore[attr-defined]
        kind = type(issue).__name__

        if kind == "Missing":
            ref_index: int = issue.ref_index  # type: ignore[attr-defined]
            strokes = self._strokes_at_step(self._step)
            styles = [StrokeStyle(color=_INK) for _ in strokes]
            if 0 <= ref_index < len(styles):
                styles[ref_index] = StrokeStyle(color=_GREEN)
            return _ResolvedView(
                strokes=strokes, styles=styles,
                label=f"足りない画 #{ref_index + 1}",
            )

        if kind == "Extra":
            user_index: int = issue.user_index  # type: ignore[attr-defined]
            strokes = self._previous_snapshot(self._step)
            styles = [StrokeStyle(color=_INK) for _ in strokes]
            if 0 <= user_index < len(styles):
                styles[user_index] = StrokeStyle(color=_RED, alpha=0.3)
            return _ResolvedView(
                strokes=strokes, styles=styles,
                label=f"余分な画 #{user_index + 1}",
            )

        if kind == "WrongOrder":
            strokes = self._strokes_at_step(self._step)
            styles = [
                StrokeStyle(color=_INK, number=i + 1)
                for i in range(len(strokes))
            ]
            if 0 <= self._sub_step < len(styles):
                styles[self._sub_step] = StrokeStyle(
                    color=_RED, number=self._sub_step + 1,
                )
            return _ResolvedView(
                strokes=strokes, styles=styles,
                label=f"順序の修正 ({self._sub_step + 1}/{len(strokes)})",
            )

        if kind == "PositionCorrection":
            strokes = self._strokes_at_step(self._step)
            prev = self._previous_snapshot(self._step)
            moved = self._moved_indices(prev, strokes, eps=0.01)
            styles = [
                StrokeStyle(color=_YELLOW if i in moved else _INK)
                for i in range(len(strokes))
            ]
            return _ResolvedView(
                strokes=strokes, styles=styles,
                label="位置の修正",
            )

        # Fallback for unknown variants
        strokes = self._strokes_at_step(self._step)
        return _ResolvedView(
            strokes=strokes,
            styles=[StrokeStyle(color=_INK) for _ in strokes],
            label="修正",
       )
    
    def _current_analysis(self) -> object | None:
        if 0 <= self._kanji_idx < len(self._analyses):
            return self._analyses[self._kanji_idx]
        return None

    def _current_issues(self) -> list[object]:
        a = self._current_analysis()
        if a is None:
            return []
        return list(a.issues)  # type: ignore[attr-defined]

    def _current_user_strokes(self) -> list[list[tuple[float, float]]]:
        a = self._current_analysis()
        if a is None:
            return []
        return list(a.strokes)  # type: ignore[attr-defined]

    def _current_qualities(self) -> list[list[float]]:
        a = self._current_analysis()
        if a is None:
            return []
        return list(a.stroke_qualities)  # type: ignore[attr-defined]

    @staticmethod
    def _moved_indices(
        prev: list[list[tuple[float, float]]],
        curr: list[list[tuple[float, float]]],
        eps: float,
    ) -> set[int]:
        """Indices in curr whose stroke differs from prev by more than eps."""
        moved: set[int] = set()
        for i, stroke in enumerate(curr):
            if i >= len(prev):
                moved.add(i)  # new stroke (shouldn't happen for PositionCorrection)
                continue
            old = prev[i]
            if len(old) != len(stroke):
                moved.add(i)
                continue
            for (ox, oy), (nx, ny) in zip(old, stroke):
                if abs(ox - nx) > eps or abs(oy - ny) > eps:
                    moved.add(i)
                    break
        return moved

    def _scale_to_canvas(
        self,
        strokes_norm: list[list[tuple[float, float]]],
    ) -> list[list[QPointF]]:
        s = self._canvas_size
        return [
            [QPointF(x * s, y * s) for (x, y) in stroke]
            for stroke in strokes_norm
        ]