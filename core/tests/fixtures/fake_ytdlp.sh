#!/bin/sh
# Fake yt-dlp for runner tests: the LAST argument (the URL) selects the behaviour.
for last; do :; done
case "$last" in
  */ok)
    echo "sf-progress:100|1000|NA|50.0|18"
    echo "sf-progress:1000|1000|NA|50.0|0"
    echo "sf-post:Merger"
    echo "sf-file:/tmp/x y.mp4"
    ;;
  */fail_login)
    echo "ERROR: Sign in to confirm you're not a bot" >&2
    exit 1
    ;;
  */slow)
    exec sleep 30
    ;;
  */nofile)
    echo "sf-progress:1|1|NA|NA|NA"
    ;;
  */exists)
    echo "[download] /tmp/My Clip.mp4 has already been downloaded"
    ;;
esac
