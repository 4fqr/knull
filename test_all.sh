#!/bin/bash
# =============================================================================
# KNULL LANGUAGE - COMPLETE FEATURE TEST
# =============================================================================

echo "=================================="
echo "  KNULL COMPLETE FEATURE TEST"
echo "=================================="
echo ""

KNULL_BIN="./src/target/release/knull"
FAILED=0
PASSED=0

# Test function
test_file() {
    local file=$1
    local desc=$2
    
    echo -n "Testing $desc... "
    if timeout 5 $KNULL_BIN run "$file" > /dev/null 2>&1; then
        echo "✓"
        ((PASSED++))
    else
        echo "✗"
        ((FAILED++))
    fi
}

echo "1. Basic Examples"
echo "-----------------"
test_file "examples/hello_world.knull" "Hello World"
test_file "examples/fibonacci.knull" "Fibonacci"
test_file "examples/primes.knull" "Primes"
test_file "examples/calculator.knull" "Calculator"
test_file "examples/guessing_game.knull" "Guessing Game"

echo ""
echo "2. File I/O Examples"
echo "--------------------"
test_file "examples/file_io_demo.knull" "File I/O Demo"

echo ""
echo "3. Threading Examples"
echo "---------------------"
test_file "examples/showcase.knull" "Showcase"

echo ""
echo "4. Networking Examples"
echo "---------------------"
test_file "examples/network_demo.knull" "Network Demo"
test_file "examples/tcp_example.knull" "TCP Example"

echo ""
echo "5. Advanced Examples"
echo "--------------------"
test_file "examples/game_of_life.knull" "Game of Life"
test_file "examples/brainfuck_compiler.knull" "Brainfuck Compiler"
test_file "examples/adventure_game.knull" "Adventure Game"

echo ""
echo "6. Feature Demonstrations"
echo "-------------------------"
test_file "examples/ffi_demo.knull" "FFI Demo"
test_file "examples/gc_demo.knull" "GC Demo"
test_file "examples/complete_demo.knull" "Complete Demo"

echo ""
echo "7. Test Suite"
echo "-------------"
for f in examples/test_*.knull; do
    test_file "$f" "$(basename $f)"
done

echo ""
echo "=================================="
echo "  RESULTS: $PASSED passed, $FAILED failed"
echo "=================================="

if [ $FAILED -eq 0 ]; then
    echo "All tests passed! ✓"
    exit 0
else
    echo "Some tests failed. ✗"
    exit 1
fi
