

# Japanese Handwriting Input for Anki

An Anki add-on for practicing Japanese handwriting. Draw a hiragana, katakana, or kanji on a canvas inside Anki, and the add-on recognizes what you wrote and inserts it into the active field. For kanji practice, it can also check your drawing against a target character and show you both what went wrong and how it should look.

⚠️ **Early development:** Handwriting recognition and writing correction may not yet be accurate enough for reliable use.

## Features

- Handwriting recognition 
  - draw a hiragana, katakana, or kanji on the canvas and get the matching character inserted into the active field.

- Kanji handwriting feedback
  - draw a kanji you're studying and see your handwriting checked against the reference, with mistakes marked on your strokes and a corrected version shown alongside.



![alt text](media/back.png) 

## Install the Add-on

1. Open Anki
2. Go to Tools → Add-ons
3. Click Get Add-ons...
4. Enter this Add-on ID: 1324989483
5. Restart Anki after installation.

## Recognition

The add-on currently uses a recognition engine based on DTW. For performance reasons, the engine is only able to recognize a character correctly when the correct number of strokes were drawn. The following table shows the recognition results using the *HANDS-nakayosi_t-98-09* dataset, restricted to samples where the correct number of strokes were drawn.

|         | Hiragana | Katakana | Kanji  | Japanese |
| ------- | -------- | -------- | ------ | -------- |
| correct | 69.11%   | 65.44%   | 91.50% | 88.25%   |

This benchmark tracks the recognition engines and models the add-on uses, or has considered using over time. The table compares the percentage of correctly recognized characters across all samples from the *HANDS-nakayosi_t-98-09* dataset for the DTW engine and Zinnia (using a model from Tegaki).

|                         | Hiragana | Katakana | Kanji  | Japanese |
| ----------------------- | -------- | -------- | ------ | -------- |
| correct (DTW)           | 60.04%   | 58.24%   | 46.38% | 45.75%   |
| correct (Zinnia+Tegaki) | 21.84%   | 1.7%     | 41.37% | 39.48%   |






## License & Acknowledgment

The KanjiVG-derived data in `data/` is licensed under
[CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/). See `data/LICENSE.txt` for full attribution.

This project uses TUAT Nakagawa Lab. HANDS-nakayosi_t-98-09 stroke database.
