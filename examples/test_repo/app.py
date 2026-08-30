# app.py — main application, uses helpers
from helpers import add, Calculator, format_result  # type: ignore[import-not-found]

def run_pipeline():
    """Run the calculation pipeline."""
    calc = Calculator()
    result = calc.calculate(10, 20)
    print(format_result(result))

    total = add(result, 5)
    print(format_result(total))

def main():
    run_pipeline()

if __name__ == "__main__":
    main()
