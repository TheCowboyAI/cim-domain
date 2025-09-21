#!/usr/bin/env bash

# Copyright (c) 2025 - Cowboy AI, LLC.
#
# Copyright validation for cim-domain. Mirrors the historical `.claude` validator
# but lives under `scripts/` so it can be invoked in Codex workflows.

set -euo pipefail

RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'
BOLD='\033[1m'

COPYRIGHT_YEAR="2025"
COPYRIGHT_HOLDER="Cowboy AI, LLC."

EXCLUSION_PATTERNS=(
  "\\.lock$"
  "^target/"
  "^result/"
  "^\.direnv/"
  "^\.git/"
  "\\.gitignore$"
  "\\.gitattributes$"
  "\\.(png|jpg|jpeg|gif|ico)$"
)

copyright_required() {
  local path="$1"
  for pattern in "${EXCLUSION_PATTERNS[@]}"; do
    [[ "$path" =~ $pattern ]] && return 1
  done
  case "${path##*.}" in
    rs|md|nix) return 0 ;;
    *) return 1 ;;
  esac
}

copyright_block() {
  case "$1" in
    rs)
      cat <<TEMPLATE
/*
 * Copyright (c) 2025 - Cowboy AI, LLC.
 */
TEMPLATE
      ;;
    md)
      echo "<!-- Copyright (c) 2025 - Cowboy AI, LLC. -->"
      ;;
    nix)
      echo "# Copyright (c) 2025 - Cowboy AI, LLC."
      ;;
    *)
      return 1
      ;;
  esac
}

has_copyright() {
  local file="$1"
  [[ ! -f "$file" ]] && return 1
  local head
  head=$(head -n 10 "$file" 2>/dev/null || echo "")
  grep -q "Copyright (c) $COPYRIGHT_YEAR - $COPYRIGHT_HOLDER" <<<"$head"
}

validate_file() {
  local file="$1"
  local mode="${2:-check}"
  if ! copyright_required "$file"; then
    [[ "$mode" == fix ]] && echo -e "${BLUE}SKIP${NC}  $file"
    return 0
  fi
  if has_copyright "$file"; then
    [[ "$mode" == fix ]] && echo -e "${GREEN}OK${NC}    $file"
    return 0
  fi
  if [[ "$mode" == fix ]]; then
    local ext="${file##*.}"
    local block
    block=$(copyright_block "$ext") || return 1
    if [[ "$ext" == "rs" ]]; then
      tmp=$(mktemp)
      printf "%s

" "$block" > "$tmp"
      cat "$file" >> "$tmp"
      mv "$tmp" "$file"
    else
      printf "%s

" "$block" > "$file".tmp
      tail -n +1 "$file" >> "$file".tmp
      mv "$file".tmp "$file"
    fi
    echo -e "${GREEN}FIXED${NC} $file"
    return 0
  fi
  echo -e "${RED}MISSING${NC} $file"
  return 1
}

validate_repo() {
  local mode="${1:-check}"
  local missing=0
  while IFS= read -r -d '' file; do
    validate_file "$file" "$mode" || missing=$((missing + 1))
  done < <(find . -type f -print0)
  if [[ $missing -gt 0 ]]; then
    echo -e "${RED}${BOLD}✗ COPYRIGHT CHECK FAILED${NC} ($missing file(s) missing header)"
    return 1
  fi
  echo -e "${GREEN}${BOLD}✓ COPYRIGHT CHECK PASSED${NC}"
}

validate_staged() {
  local mode="${1:-check}"
  local exit=0
  local files
  files=$(git diff --cached --name-only --diff-filter=ACM 2>/dev/null || echo "")
  [[ -z "$files" ]] && {
    echo "No staged files to check."; return 0;
  }
  while IFS= read -r file; do
    [[ -z "$file" ]] && continue
    validate_file "$file" "$mode" || exit=1
    if [[ "$mode" == fix && $? -eq 0 ]]; then
      git add "$file"
    fi
  done <<<"$files"
  return $exit
}

usage() {
  cat <<HELP
Usage: $0 [--fix|--staged|--fix-staged|--file PATH|--fix-file PATH]
Checks (and optionally fixes) copyright headers for Rust, Markdown, and Nix sources.
HELP
}

case "${1:-}" in
  --fix) validate_repo fix ;;
  --staged) validate_staged check ;;
  --fix-staged) validate_staged fix ;;
  --file) validate_file "${2:?--file requires a path}" ;;
  --fix-file) validate_file "${2:?--fix-file requires a path}" fix ;;
  -h|--help) usage ;;
  "") validate_repo check ;;
  *) usage >&2; exit 1 ;;
esac
