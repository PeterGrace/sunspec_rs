release-patch:
  cargo release  --no-verify patch --execute
release-minor:
  cargo release  --no-verify minor --execute
release-major:
  cargo release  --no-verify major --execute
