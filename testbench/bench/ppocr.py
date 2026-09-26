"""PaddleOCR's PP-OCRv3 English models, run through OpenCV's dnn module.

Tesseract is a document reader: it expects dark text on a light page, and reads game
text only after preprocessing -- three passes that still missed 6% of the phrases in 86
bench frames, and read "Werewolf Pack" (dark on a bright blue gradient button, 3440x1440)
as "B er If Pack".  PP-OCR is trained on scene text, any colour on any ground, and read
all 547 of them (testbench/README.md, "OCR").  It needs no package beyond the
python-opencv the bench already uses: the two ONNX models are fetched on first use into
the bench cache, pinned by revision and checked by sha256.  Both are Apache-2.0.

ocr(img) returns text *lines*, each split into words with boxes interpolated along the
line, in the shape vision.ocr() returned tesseract's.
"""
import hashlib
import sys
import threading
import urllib.request

import cv2
import numpy as np

from . import config

MODELS = config.CACHE / 'models'
DET = ('text_detection_en_ppocrv3_2023may.onnx',
       'https://huggingface.co/opencv/text_detection_ppocr/resolve/'
       'c299e81aedeaf5d4f741fccccea491fb32db005f/text_detection_en_ppocrv3_2023may.onnx',
       '03f550c6b406fda8bf54bd8327815f6c7e2edd98cea02348c93d879254366587')
REC = ('en_PP-OCRv3_rec_infer.onnx',
       'https://huggingface.co/SWHL/RapidOCR/resolve/'
       '1cfba2e90fc938db55889873735088de210cc173/PP-OCRv3/en_PP-OCRv3_rec_infer.onnx',
       'ef7abd8bd3629ae57ea2c28b425c1bd258a871b93fd2fe7c433946ade9b5d9ea')
# PaddleOCR's ppocr/utils/en_dict.txt, in order; CTC class 0 is the blank, and the
# recogniser was trained with use_space_char, which appends one more space
CHARS = ['<blank>'] + list('0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`'
                           'abcdefghijklmnopqrstuvwxyz{|}~!"#$%&\'()*+,-./ ') + [' ']
MAX_SIDE = 2560      # detector input; a 3440x1440 frame reads in ~1 s at this size

_lock = threading.Lock()     # cv2.dnn nets are not thread-safe; `run --jobs` shares them
_det = _rec = None
_error = None


def _fetch(name, url, sha):
    path = MODELS / name
    if path.exists() and hashlib.sha256(path.read_bytes()).hexdigest() == sha:
        return path
    MODELS.mkdir(parents=True, exist_ok=True)
    print(f'a2test: fetching OCR model {name}', file=sys.stderr)
    data = urllib.request.urlopen(url, timeout=60).read()
    got = hashlib.sha256(data).hexdigest()
    if got != sha:
        raise RuntimeError(f'{name}: sha256 {got}, expected {sha}')
    tmp = path.with_suffix('.part')
    tmp.write_bytes(data)
    tmp.replace(path)
    return path


def available():
    """True if the models are loaded or can be; the reason is kept in `error()` if not."""
    global _det, _rec, _error
    with _lock:
        if _det is not None:
            return True
        if _error is not None:
            return False
        try:
            det = cv2.dnn.TextDetectionModel_DB(str(_fetch(*DET)))
            det.setBinaryThreshold(0.3).setPolygonThreshold(0.5).setUnclipRatio(2.0).setMaxCandidates(500)
            det.setInputParams(1.0 / 255.0, (0, 0), (122.67891434, 116.66876762, 104.00698793))
            _rec = cv2.dnn.readNet(str(_fetch(*REC)))
            _det = det
            return True
        except Exception as e:
            _error = f'{type(e).__name__}: {e}'
            return False


def error():
    return _error


def _recognise(crop):
    h, w = crop.shape[:2]
    W = max(16, min(1600, round(48 * w / h / 8) * 8))
    blob = cv2.dnn.blobFromImage(crop, 1 / 127.5, (W, 48), (127.5, 127.5, 127.5), swapRB=True)
    _rec.setInput(blob)
    out = _rec.forward()[0]                  # time steps x classes
    idx, prob = out.argmax(1), out.max(1)
    text, ps, prev = [], [], 0
    for i, p in zip(idx, prob):              # greedy CTC: collapse repeats, drop blanks
        if i != prev and i != 0:
            text.append(CHARS[i])
            ps.append(p)
        prev = i
    return ''.join(text).strip(), (float(np.mean(ps)) if ps else 0.0)


def _rectify(img, quad):
    """The detected quad, cut out and straightened."""
    pts = cv2.boxPoints(cv2.minAreaRect(np.asarray(quad, dtype=np.float32)))
    s, d = pts.sum(1), np.diff(pts, axis=1).ravel()
    tl, br, tr, bl = pts[s.argmin()], pts[s.argmax()], pts[d.argmin()], pts[d.argmax()]
    w = int(max(np.linalg.norm(tr - tl), np.linalg.norm(br - bl)))
    h = int(max(np.linalg.norm(bl - tl), np.linalg.norm(br - tr)))
    if w < 4 or h < 4:
        return None
    m = cv2.getPerspectiveTransform(np.float32([tl, tr, br, bl]),
                                    np.float32([[0, 0], [w, 0], [w, h], [0, h]]))
    return cv2.warpPerspective(img, m, (w, h), borderMode=cv2.BORDER_REPLICATE)


def ocr(img):
    """Words in a BGR image: [{text, conf, x, y, w, h, line, src}], conf in percent.
    Call available() first."""
    H, W = img.shape[:2]
    s = min(1.0, MAX_SIDE / max(H, W))
    words = []
    with _lock:
        _det.setInputSize((max(32, int(W * s) // 32 * 32), max(32, int(H * s) // 32 * 32)))
        quads, _ = _det.detect(img)
        for k, q in enumerate(quads):
            crop = _rectify(img, q)
            if crop is None or crop.shape[0] > crop.shape[1] * 1.5:     # vertical: not UI text
                continue
            text, conf = _recognise(crop)
            if not text:
                continue
            x, y, w, h = cv2.boundingRect(np.asarray(q, dtype=np.int32))
            # the recogniser reads a line; split it into words at interpolated x
            n, cx = len(text), x
            for part in text.split():
                pw = max(1, int(w * len(part) / n))
                words.append(dict(text=part, conf=round(conf * 100, 1), x=cx, y=y, w=pw, h=h,
                                  line=('ppocr', k), src='ppocr'))
                cx += int(w * (len(part) + 1) / n)
    return words
