from collections.abc import Sequence
from typing import Final, final

@final
class AnalyzeResult:
    @property
    def correct(self, /) -> str: ...
    @property
    def kind(self, /) -> ResultKind: ...
    @property
    def wrong(self, /) -> str: ...

@final
class Analyzer:
    def __new__(cls, /, map_path: str) -> Analyzer: ...
    def analyze(self, /, kanji: str, strokes: Sequence[Sequence[tuple[float, float]]], grid_color: str, corner_radius: float, stroke_color: str) -> AnalyzeResult:
        """
        Analyzes the drawn `strokes` for `kanji` and returns a rendered result.
        
        # Errors
        Returns a `PyRuntimeError` if the kanji is not found in the map or the
        strokes cannot be analyzed.
        """

@final
class KanjiGrid:
    def __new__(cls, /, map_path: str) -> KanjiGrid: ...
    def generate(self, /, grid_color: str, corner_radius: float) -> str: ...
    def generate_with_hint(self, /, grid_color: str, corner_radius: float, hint: str, hint_color: str) -> str: ...

@final
class Recognizer:
    def __new__(cls, /, map_path: str) -> Recognizer: ...
    def compare_with_target(self, /, committed: Sequence[Sequence[Sequence[tuple[float, float]]]], target: str) -> str: ...
    def recognize(self, /, committed: Sequence[Sequence[Sequence[tuple[float, float]]]]) -> str: ...

@final
class ResultKind:
    NothingFound: Final[ResultKind]
    StrokeInsertedOrRemoved: Final[ResultKind]
    StrokeMovedOrScaled: Final[ResultKind]
    StrokeOrder: Final[ResultKind]
    Unknown: Final[ResultKind]
    WrongDrawn: Final[ResultKind]
    def __int__(self, /) -> int: ...
    def __repr__(self, /) -> str: ...
