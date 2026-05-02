from aqt.qt import (
    QWidget, QSize, QPalette, QColor,
    QPainter, QPen, QPointF, QPaintEvent,
    Qt,
)


class GridViewer(QWidget):
    """Base widget: fixed-size square canvas with grid + static strokes.

    No input handling. Subclasses add drawing or review behavior.
    """

    def __init__(
        self,
        parent: QWidget | None = None,
        size: int = 300,
        pen_width: float = 8.0,
    ) -> None:
        super().__init__(parent)
        palette = self.palette()

        self._canvas_size: int = size
        self._pen_width: float = pen_width
        self._bg_color: QColor = palette.color(QPalette.ColorRole.Base)
        self._ink_color: QColor = palette.color(QPalette.ColorRole.Text)

        # strokes: list of strokes, each stroke is a list of QPointF (pixel space)
        self._strokes: list[list[QPointF]] = []

        self.setFixedSize(QSize(size, size))

    # --- public API ---

    def set_strokes(self, strokes: list[list[QPointF]]) -> None:
        """Replace displayed strokes and repaint."""
        self._strokes = [list(s) for s in strokes]
        self.update()

    def clear(self) -> None:
        """Remove all strokes and repaint."""
        self._strokes.clear()
        self.update()

    def stroke_count(self) -> int:
        return len(self._strokes)

    # --- painting ---

    def paintEvent(self, a0: QPaintEvent | None) -> None:
        if a0 is None:
            return
        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)

        painter.fillRect(self.rect(), self._bg_color)
        self._draw_grid(painter)

        pen = QPen(
            self._ink_color, self._pen_width,
            Qt.PenStyle.SolidLine,
            Qt.PenCapStyle.RoundCap,
            Qt.PenJoinStyle.RoundJoin,
        )
        painter.setPen(pen)

        for stroke in self._strokes:
            self._draw_stroke(painter, stroke)

    def _draw_grid(self, painter: QPainter) -> None:
        w, h = self.width(), self.height()
        cx, cy = w // 2, h // 2

        bg = self._bg_color
        text = self._ink_color
        grid_color = QColor(
            (bg.red() + text.red()) // 2,
            (bg.green() + text.green()) // 2,
            (bg.blue() + text.blue()) // 2,
            120,
        )

        pen = QPen(grid_color, 3, Qt.PenStyle.DashLine)
        painter.setPen(pen)

        painter.drawLine(cx, 0, cx, h)
        painter.drawLine(0, cy, w, cy)

    def _draw_stroke(self, painter: QPainter, stroke: list[QPointF]) -> None:
        if len(stroke) < 2:
            if len(stroke) == 1:
                painter.drawPoint(stroke[0])
            return
        for i in range(len(stroke) - 1):
            painter.drawLine(stroke[i], stroke[i + 1])
