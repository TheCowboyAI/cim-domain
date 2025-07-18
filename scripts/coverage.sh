#!/usr/bin/env bash
# Script to run test coverage with various options

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}CIM Domain Test Coverage${NC}"
echo "========================"
echo ""

# Parse command line arguments
COVERAGE_TYPE="${1:-lib}"
OUTPUT_FORMAT="${2:-Html}"

case "$COVERAGE_TYPE" in
    "lib")
        echo -e "${YELLOW}Running library coverage...${NC}"
        cargo tarpaulin --lib --out "$OUTPUT_FORMAT"
        ;;
    "all")
        echo -e "${YELLOW}Running full coverage with all features...${NC}"
        cargo tarpaulin --all-features --out "$OUTPUT_FORMAT"
        ;;
    "examples")
        echo -e "${YELLOW}Running coverage including examples...${NC}"
        cargo tarpaulin --lib --examples --out "$OUTPUT_FORMAT"
        ;;
    "detailed")
        echo -e "${YELLOW}Running detailed coverage analysis...${NC}"
        cargo tarpaulin --lib \
            --out "$OUTPUT_FORMAT" \
            --exclude-files "*/tests/*" \
            --exclude-files "*/examples/*" \
            --exclude-files "*/target/*" \
            --ignore-panics \
            --ignored
        ;;
    "llvm")
        echo -e "${YELLOW}Running LLVM coverage...${NC}"
        cargo llvm-cov --html
        echo -e "${GREEN}Coverage report generated in target/llvm-cov/html/index.html${NC}"
        exit 0
        ;;
    *)
        echo -e "${RED}Unknown coverage type: $COVERAGE_TYPE${NC}"
        echo "Usage: $0 [lib|all|examples|detailed|llvm] [Html|Xml|Lcov|Json]"
        exit 1
        ;;
esac

# Display results based on output format
case "$OUTPUT_FORMAT" in
    "Html")
        echo -e "${GREEN}Coverage report generated in tarpaulin-report.html${NC}"
        ;;
    "Xml")
        echo -e "${GREEN}Coverage report generated in cobertura.xml${NC}"
        ;;
    "Lcov")
        echo -e "${GREEN}Coverage report generated in lcov.info${NC}"
        ;;
    "Json")
        echo -e "${GREEN}Coverage report generated in tarpaulin-report.json${NC}"
        ;;
esac

# Show coverage summary
echo ""
echo -e "${GREEN}Coverage Summary:${NC}"
cargo tarpaulin --lib --print-summary