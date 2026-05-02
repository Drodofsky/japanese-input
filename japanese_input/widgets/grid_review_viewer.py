from dataclasses import dataclass, field
from aqt.qt import (
    QWidget, QPointF, QColor, QPainter, QPen, QPaintEvent, QFont,
    Qt,
)

from .grid_viewer import GridViewer


@dataclass
class StrokeStyle:
    """How one stroke should be rendered."""
    color: QColor
    alpha: float = 1.0          # 0.0–1.0; multiplied with color's alpha
    number: int | None = None   # 1-indexed overlay number, or None
    qualities: list[float] | None = None  # per-point [0,1]; if set, segments colored by quality

class GridReviewViewer(GridViewer):
    """GridViewer that renders strokes with per-stroke styling.

    Used by ReviewWidget to display fix snapshots with highlights.
    """

    def __init__(
        self,
        parent: QWidget | None = None,
        size: int = 300,
        pen_width: float = 8.0,
    ) -> None:
        super().__init__(parent=parent, size=size, pen_width=pen_width)
        self._styles: list[StrokeStyle] = []

    # --- public API ---

    def set_styled_strokes(
        self,
        strokes: list[list[QPointF]],
        styles: list[StrokeStyle],
    ) -> None:
        if len(strokes) != len(styles):
            raise ValueError(
                f"strokes ({len(strokes)}) and styles ({len(styles)}) length mismatch"
            )
        self._strokes = [list(s) for s in strokes]
        self._styles = list(styles)
        self.update()

    # --- painting ---

    def paintEvent(self, a0: QPaintEvent | None) -> None:
        if a0 is None:
            return
        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)

        painter.fillRect(self.rect(), self._bg_color)
        self._draw_grid(painter)

        # If no styles set, fall back to default ink color
        if not self._styles or len(self._styles) != len(self._strokes):
            pen = QPen(
                self._ink_color, self._pen_width,
                Qt.PenStyle.SolidLine,
                Qt.PenCapStyle.RoundCap,
                Qt.PenJoinStyle.RoundJoin,
            )
            painter.setPen(pen)
            for stroke in self._strokes:
                self._draw_stroke(painter, stroke)
            return

        # Styled rendering
        for stroke, style in zip(self._strokes, self._styles):
            if style.qualities is not None:
                self._draw_stroke_with_qualities(painter, stroke, style.qualities, style.alpha)
            else:
                color = QColor(style.color)
                color.setAlphaF(color.alphaF() * style.alpha)
                pen = QPen(
                    color, self._pen_width,
                    Qt.PenStyle.SolidLine,
                    Qt.PenCapStyle.RoundCap,
                    Qt.PenJoinStyle.RoundJoin,
                )
                painter.setPen(pen)
                self._draw_stroke(painter, stroke)
        # Number overlays drawn last so they sit on top
        font = QFont()
        font.setPixelSize(int(self._canvas_size * 0.06))
        font.setBold(True)
        painter.setFont(font)

        number_pen = QPen(self._ink_color)
        painter.setPen(number_pen)

        for stroke, style in zip(self._strokes, self._styles):
            if style.number is None or not stroke:
                continue
            anchor = stroke[0]
            painter.drawText(
                QPointF(anchor.x() + 4, anchor.y() - 4),
                str(style.number),
            )
    def _draw_stroke_with_qualities(
        self,
        painter: QPainter,
        stroke: list[QPointF],
        qualities: list[float],
        alpha: float,
    ) -> None:
        if len(stroke) < 2:
            if stroke:
                painter.drawPoint(stroke[0])
            return
        # qualities is per-point; segment color = avg of endpoint qualities
        n_segments = len(stroke) - 1
        for i in range(n_segments):
            q_a = qualities[i] if i < len(qualities) else 1.0
            q_b = qualities[i + 1] if i + 1 < len(qualities) else 1.0
            q = (q_a + q_b) * 0.5
            color = self._quality_color(q)
            color.setAlphaF(color.alphaF() * alpha)
            pen = QPen(
                color, self._pen_width,
                Qt.PenStyle.SolidLine,
                Qt.PenCapStyle.RoundCap,
                Qt.PenJoinStyle.RoundJoin,
            )
            painter.setPen(pen)
            painter.drawLine(stroke[i], stroke[i + 1])

    @staticmethod
    def _quality_color(q: float) -> QColor:
        """Interpolate red → yellow → green over q in [0, 1]."""
        q = max(0.0, min(1.0, q))
        if q < 0.5:
            # red → yellow
            t = q * 2.0
            r = 200
            g = int(50 + (170 - 50) * t)
            b = int(50 + (30 - 50) * t)
        else:
            # yellow → green
            t = (q - 0.5) * 2.0
            r = int(220 + (40 - 220) * t)
            g = int(170 + (160 - 170) * t)
            b = int(30 + (70 - 30) * t)
        return QColor(r, g, b)