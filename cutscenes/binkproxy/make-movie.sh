#!/usr/bin/env bash
# Turn any video into the pair the Bink replacement reads:
#
#   cutscenes/binkproxy/make-movie.sh <input> <outdir>/<Name> [crf] [--audio <file>]
#
# Usually called by build-movie.sh, which owns the cutscenes/movies/<Name>/ layout.
#
# writes <outdir>/<Name>.mp4 (AV1 video, no audio) and <outdir>/<Name>.wav
# (16-bit PCM).  <Name> must match the .bik it stands in for, e.g. Intro.
#
# WHY AV1 AND A SEPARATE WAV -- measured under Proton-CachyOS, not chosen:
#   H.264 and VP9 in MP4 both come back from Media Foundation with no video
#   stream (MF_E_INVALIDSTREAMNUMBER).  GStreamer autoplugs the decoder, it
#   "failed to initialise", and the retry asks for Proton's media converter,
#   which does not register outside Steam.  AV1 decodes (dav1d).  AAC audio
#   is refused the same way, so the audio goes beside the video as PCM and
#   the DLL reads it itself.
# Colour: tagged BT.601 limited range, which is what the source YUV was (it
# came from Bink); untagged, GStreamer picks BT.709 for anything >= 720 lines.
# Dimensions must be even.
#
#   --audio <file>   take the WAV from <file> instead of from <input> -- build-movie.sh
#                    passes the stock .bik, so the audio skips the intermediate's AAC.
set -euo pipefail

in="$1"
out="$2"
shift 2
crf=28
if [ $# -gt 0 ] && [ "${1#--}" = "$1" ]; then crf="$1"; shift; fi
audio="$in"
while [ $# -gt 0 ]; do
    case "$1" in
        --audio) audio="$2"; shift ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
    shift
done

# SVT-AV1 prints its own banner past ffmpeg's -v; 1 = errors only.
SVT_LOG=1 ffmpeg -v error -y -i "$in" -an \
    -c:v libsvtav1 -preset 6 -crf "$crf" -g 60 -pix_fmt yuv420p \
    -colorspace bt470bg -color_primaries bt470bg -color_trc smpte170m -color_range tv \
    -movflags +faststart "$out.mp4"
ffmpeg -v error -y -i "$audio" -vn -c:a pcm_s16le "$out.wav"

ffprobe -v error -show_entries stream=codec_name,width,height,r_frame_rate,sample_rate,channels:format=duration \
    -of compact "$out.mp4" "$out.wav" 2>/dev/null || true
ls -l "$out.mp4" "$out.wav"
