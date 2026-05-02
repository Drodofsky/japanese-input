from aqt.qt import (
    QWidget, QPointF, QPainter, QPen, QPaintEvent,
    QMouseEvent, QTabletEvent, QEvent,
    Qt, qtmajor,
    QColor, QFont, QFontDatabase, QImage, QRect,
)

from .grid_viewer import GridViewer


class GridDrawViewer(GridViewer):
    """GridViewer with mouse/tablet input.

    Captures user strokes; exposes undo. No buttons — the outer
    reviewer widget owns those.
    """

    def __init__(
        self,
        parent: QWidget | None = None,
        size: int = 300,
        pen_width: float = 8.0,
    ) -> None:
        super().__init__(parent=parent, size=size, pen_width=pen_width)

        self._current_stroke: list[QPointF] = []
        self._drawing: bool = False
        self._hint_char: str = ""
        self._hint_offset: tuple[int, int] | None = None
        self.setAttribute(Qt.WidgetAttribute.WA_AcceptTouchEvents, True)

    # --- public API ---
    def strokes(self) -> list[list[QPointF]]:
        """Return a copy of the currently displayed strokes."""
        return [list(s) for s in self._strokes]



    def undo_stroke(self) -> None:
        """Remove the last completed stroke. Returns new stroke count."""
        if self._strokes:
            self._strokes.pop()
            self.update()

    def clear(self) -> None:
        super().clear()
        self._current_stroke.clear()
        self._drawing = False
        self.clear_hint()

    # --- painting ---

    def paintEvent(self, a0: QPaintEvent | None) -> None:
        if a0 is None:
            return
        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)

        # background, grid, hint, completed strokes, current stroke
        painter.fillRect(self.rect(), self._bg_color)
        self._draw_grid(painter)
        if self._hint_char:
            self._draw_hint(painter)

        pen = QPen(
            self._ink_color, self._pen_width,
            Qt.PenStyle.SolidLine,
            Qt.PenCapStyle.RoundCap,
            Qt.PenJoinStyle.RoundJoin,
        )
        painter.setPen(pen)
        for stroke in self._strokes:
            self._draw_stroke(painter, stroke)
        if self._current_stroke:
            self._draw_stroke(painter, self._current_stroke)
        # --- tablet input ---
    def set_hint(self, character: str) -> None:
        self._hint_char = character
        self._hint_offset = None
        self.update()

    def clear_hint(self) -> None:
        self._hint_char = ""
        self._hint_offset = None
        self.update()

    def has_hint(self) -> bool:
        return bool(self._hint_char)

    def _draw_hint(self, painter: QPainter) -> None:
        hint_color = QColor(self._ink_color)
        hint_color.setAlpha(30)

        font = QFont()
        if "Klee One" in QFontDatabase.families():
            font.setFamily("Klee One")
        font.setPixelSize(int(self._canvas_size * 0.85))

        if self._hint_offset is None:
            self._hint_offset = self._compute_hint_offset(font)

        dest_x, dest_y = self._hint_offset
        buf = self._canvas_size * 2

        painter.setFont(font)
        painter.setPen(QPen(hint_color))
        painter.drawText(
            QRect(dest_x, dest_y, buf, buf),
            Qt.AlignmentFlag.AlignHCenter | Qt.AlignmentFlag.AlignVCenter,
            self._hint_char,
        )

    def _compute_hint_offset(self, font: QFont) -> tuple[int, int]:
        size = self._canvas_size
        buf = size * 2
        img = QImage(buf, buf, QImage.Format.Format_ARGB32)
        img.fill(0)
        tmp = QPainter(img)
        tmp.setFont(font)
        tmp.setPen(QPen(QColor(255, 255, 255, 255)))
        tmp.drawText(
            QRect(0, 0, buf, buf),
            Qt.AlignmentFlag.AlignHCenter | Qt.AlignmentFlag.AlignVCenter,
            self._hint_char,
        )
        tmp.end()

        top, bottom, left, right = buf, 0, buf, 0
        found = False
        for y in range(buf):
            for x in range(buf):
                if (img.pixel(x, y) >> 24) & 0xFF > 10:
                    top = min(top, y)
                    bottom = max(bottom, y)
                    left = min(left, x)
                    right = max(right, x)
                    found = True

        if not found:
            return (0, 0)

        glyph_w = right - left + 1
        glyph_h = bottom - top + 1
        dest_x = (size - glyph_w) // 2 - left
        dest_y = (size - glyph_h) // 2 - top
        return (dest_x, dest_y)  
    def tabletEvent(self, a0: QTabletEvent | None) -> None:
        if a0 is None:
            return
        if qtmajor > 5:
            pos = a0.position()
        else:
            pos = a0.posF()  # type: ignore[attr-defined]

        point = QPointF(pos.x(), pos.y())
        event_type = a0.type()

        if event_type == QEvent.Type.TabletPress:
            self._start_stroke(point)
        elif event_type == QEvent.Type.TabletMove:
            if self._drawing:
                self._add_point(point)
        elif event_type == QEvent.Type.TabletRelease:
            self._end_stroke()

        a0.accept()

    # --- mouse input (fallback) ---

    def mousePressEvent(self, a0: QMouseEvent | None) -> None:
        if a0 is None:
            return
        if a0.button() == Qt.MouseButton.LeftButton:
            self._start_stroke(QPointF(a0.position()))
        a0.accept()

    def mouseMoveEvent(self, a0: QMouseEvent | None) -> None:
        if a0 is None:
            return
        if self._drawing:
            self._add_point(QPointF(a0.position()))
        a0.accept()

    def mouseReleaseEvent(self, a0: QMouseEvent | None) -> None:
        if a0 is None:
            return
        if a0.button() == Qt.MouseButton.LeftButton:
            self._end_stroke()
        a0.accept()

    # --- stroke management ---

    def _start_stroke(self, point: QPointF) -> None:
        self._drawing = True
        self._current_stroke = [point]
        self.update()

    def _add_point(self, point: QPointF) -> None:
        self._current_stroke.append(point)
        self.update()

    def _end_stroke(self) -> None:
        if self._current_stroke:
            self._strokes.append(self._current_stroke)
            self._current_stroke = []
        self._drawing = False
        self.update()