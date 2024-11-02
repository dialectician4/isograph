pnpm format && \
  cargo clippy && \
  pnpm build-demos && \
  ./scripts/check-git-status.sh && \
  pnpm exec tsc --build && \
  pnpm test
if [ $? -eq 0 ]; then
    echo OK
else
    echo FAIL
fi
